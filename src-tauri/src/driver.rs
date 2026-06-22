//! The backend driver loop (ADR 0003 first vertical slice; ADR 0026 fade).
//!
//! A background thread polls the active backend, emits the resulting Runtime
//! Events to the frontend, and drives Overlay Visibility (Pinned or Fade). Time
//! lives here, not in `keyplane-core`, so core resolution stays deterministic;
//! the Fade timing is the clock-injected `FadeController`.

use crate::state::{AppState, EVENT_RUNTIME};
use keyplane_core::profile::VisibilityPolicy;
use keyplane_core::visibility::FadeController;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// How long each demo layer state is held before advancing.
const STEP: Duration = Duration::from_millis(1200);

/// Inactivity before the overlay fades under the Fade policy.
const FADE_TIMEOUT_MS: u64 = 4000;

/// Spawn the driver thread. It owns the Fade timing and the wall clock; it locks
/// [`AppState`] each tick so commands can interleave (imports, edits, connect).
pub fn spawn(handle: AppHandle) {
    std::thread::spawn(move || {
        let start = Instant::now();
        let mut fade = FadeController::new(FADE_TIMEOUT_MS);
        let mut overlay_visible = false;
        loop {
            std::thread::sleep(STEP);
            tick(&handle, start, &mut fade, &mut overlay_visible);
        }
    });
}

fn tick(
    handle: &AppHandle,
    start: Instant,
    fade: &mut FadeController,
    overlay_visible: &mut bool,
) {
    let now_ms = start.elapsed().as_millis() as u64;
    let state = handle.state::<AppState>();
    let mut inner = state.inner.lock().expect("state poisoned");

    // Pull pending updates, looping the Fake Backend's demo when it runs dry.
    let backend_id = inner.backend.id();
    let mut updates = inner.backend.poll();
    if updates.is_empty() {
        inner.backend.replay();
        updates = inner.backend.poll();
    }

    let mut activity = false;
    for update in updates {
        if let Some(event) = inner.composer.apply(&backend_id, update) {
            let _ = handle.emit(EVENT_RUNTIME, &event);
            activity = true;
        }
    }
    if activity {
        fade.on_activity(now_ms);
    }

    let reveal = !inner.overlay_shown;
    if reveal {
        inner.overlay_shown = true;
    }
    let display = inner.profile.overlay.display.clone();
    let policy = inner.profile.overlay.visibility;
    drop(inner);

    // Reveal the overlay on the first snapshot, click-through by default, placed
    // per the Profile's Display Targeting (ADR 0027).
    if reveal {
        if let Some(overlay) = handle.get_webview_window("overlay") {
            crate::commands::apply_display_targeting(&overlay, &display);
            let _ = overlay.set_ignore_cursor_events(true);
            let _ = overlay.show();
            *overlay_visible = true;
        }
    }

    // Fade Visibility: show on activity, hide after the inactivity interval.
    // Pinned and ManualToggle leave window visibility to the reveal/commands.
    if policy == VisibilityPolicy::Fade {
        let should = fade.visible(now_ms);
        if should != *overlay_visible {
            if let Some(overlay) = handle.get_webview_window("overlay") {
                let _ = if should { overlay.show() } else { overlay.hide() };
                *overlay_visible = should;
            }
        }
    }
}
