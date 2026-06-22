//! The Fake Backend driver loop (ADR 0003 first vertical slice).
//!
//! A background thread advances the scripted backend one step at a time and
//! emits the resulting Runtime Events to the frontend, animating live layer
//! changes in the Overlay Window without real hardware. Time lives here, not in
//! `keyplane-core`, so core resolution stays deterministic.

use crate::state::{AppState, EVENT_RUNTIME, FAKE_BACKEND_ID};
use keyplane_core::backend::ProtocolBackend;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// How long each demo layer state is held before advancing.
const STEP: Duration = Duration::from_millis(1200);

/// Spawn the driver thread. It owns no state directly; it locks [`AppState`]
/// each tick so commands can interleave (imports, hand edits, positioning).
pub fn spawn(handle: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(STEP);
        tick(&handle);
    });
}

fn tick(handle: &AppHandle) {
    let state = handle.state::<AppState>();
    let mut inner = state.inner.lock().expect("state poisoned");

    // Pull one scripted update, looping the demo when it runs dry.
    let mut updates = inner.backend.poll();
    if updates.is_empty() {
        inner.backend.replay();
        updates = inner.backend.poll();
    }

    for update in updates {
        if let Some(event) = inner.composer.apply(FAKE_BACKEND_ID, update) {
            let _ = handle.emit(EVENT_RUNTIME, &event);
        }
    }

    let reveal = !inner.overlay_shown;
    if reveal {
        inner.overlay_shown = true;
    }
    drop(inner);

    // Reveal the overlay on the first snapshot, click-through by default.
    if reveal {
        if let Some(overlay) = handle.get_webview_window("overlay") {
            let _ = overlay.set_ignore_cursor_events(true);
            let _ = overlay.show();
        }
    }
}
