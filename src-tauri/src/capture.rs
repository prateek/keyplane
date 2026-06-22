//! OS-level Host Input Event capture for the sentinel backend (ADR 0016, 0023).
//!
//! Captures global key events with `rdev` and feeds them to the active sentinel
//! backend. If the OS denies the input hook (e.g. macOS Input Monitoring /
//! Accessibility not granted), that surfaces as persistent
//! [`HealthState::PermissionMissing`] Backend Health rather than a silent
//! failure (ADR 0023), which is exactly the permission-visibility the PRD wants.
//!
//! This is opt-in (the `connect_sentinel` `os_capture` flag) and starts a global
//! key listener, so it is never enabled implicitly.
//!
//! macOS caveat (validated): `rdev::listen` must run on the thread that owns the
//! CFRunLoop — the main thread. The capture *mechanism* is proven end to end by
//! `keyplane-sentinel`'s `validate_capture` example (a real OS hook captures a
//! synthesized key and drives the sentinel Layer Stack). Tauri owns the main
//! thread for its own event loop, so this spawned-thread listener is reliable on
//! Linux/Windows but limited on macOS; a denied hook or no events still surfaces
//! as Backend Health rather than a silent failure. A main-thread tap (or a small
//! helper process) on macOS is the follow-up to wire it into the running app.

use crate::state::{AppState, EVENT_RUNTIME};
use keyplane_core::health::HealthState;
use tauri::{AppHandle, Emitter, Manager};

/// Start capturing global key events and feeding them to the sentinel backend.
/// Runs on its own thread; reports permission/health failures back to the UI.
pub fn start_sentinel_capture(handle: AppHandle) {
    std::thread::spawn(move || {
        let cb_handle = handle.clone();
        let callback = move |event: rdev::Event| {
            let (key, down) = match event.event_type {
                rdev::EventType::KeyPress(k) => (format!("{k:?}"), true),
                rdev::EventType::KeyRelease(k) => (format!("{k:?}"), false),
                _ => return,
            };
            feed(&cb_handle, key, down);
        };

        if let Err(err) = rdev::listen(callback) {
            report_permission_missing(&handle, format!("{err:?}"));
        }
    });
}

/// Feed one captured event to the active backend and emit any resulting change.
fn feed(handle: &AppHandle, key: String, down: bool) {
    let state = handle.state::<AppState>();
    let mut inner = state.inner.lock().expect("state poisoned");
    let backend_id = inner.backend.id();
    inner.backend.feed_host_event(key, down);
    let updates = inner.backend.poll();
    for update in updates {
        if let Some(event) = inner.composer.apply(&backend_id, update) {
            let _ = handle.emit(EVENT_RUNTIME, &event);
        }
    }
}

/// Record a missing-permission Health State for the sentinel backend.
fn report_permission_missing(handle: &AppHandle, detail: String) {
    let state = handle.state::<AppState>();
    let mut inner = state.inner.lock().expect("state poisoned");
    if let Some(event) = inner.composer.set_backend_health(
        "sentinel",
        HealthState::PermissionMissing {
            permission: "input-monitoring".to_string(),
            detail,
        },
    ) {
        let _ = handle.emit(EVENT_RUNTIME, &event);
    }
}
