//! Keyplane Tauri shell.
//!
//! This crate owns the desktop runtime: the App and Overlay windows, the
//! command/event boundary the frontend talks to, and the driver loop that
//! advances the Fake Backend and emits Runtime Events. All keyboard meaning —
//! resolution, health, EDN persistence — lives in `keyplane-core`; this layer
//! only does windows, scheduling, and serialization (ADR 0015, 0036, 0044).

mod capture;
mod commands;
mod driver;
mod state;

use state::AppState;

/// Build and run the Keyplane application.
pub fn run() {
    tauri::Builder::default()
        // Launch-at-login designed in from day one (ADR 0039, story 59). Off
        // until the user enables it via `set_autostart`.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_profile,
            commands::active_profile_edn,
            commands::apply_profile_edn,
            commands::import_preview,
            commands::commit_import,
            commands::connect_keypeek,
            commands::connect_kanata,
            commands::connect_sentinel,
            commands::feed_host_event,
            commands::promote_override,
            commands::set_positioning_mode,
            commands::set_overlay_visible,
            commands::set_display_targeting,
            commands::set_autostart,
            commands::get_autostart,
        ])
        .setup(|app| {
            driver::spawn(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Keyplane");
}
