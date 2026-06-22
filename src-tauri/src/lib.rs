pub mod backend;
pub mod domain;
pub mod fake_backend;
pub mod importers;
pub mod keypeek_backend;
pub mod profile_codec;

use crate::backend::{FakeProtocolBackend, ProtocolBackend};
use crate::domain::{
    apply_runtime_event, ImportCandidate, KeyboardSnapshot, Profile, RuntimeEvent,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn initial_snapshot() -> KeyboardSnapshot {
    FakeProtocolBackend
        .initial_snapshot()
        .expect("fake backend always provides a snapshot")
}

#[tauri::command]
fn fake_runtime_events() -> Vec<RuntimeEvent> {
    FakeProtocolBackend.demo_events()
}

#[tauri::command]
fn apply_event(mut snapshot: KeyboardSnapshot, event: RuntimeEvent) -> KeyboardSnapshot {
    apply_runtime_event(&mut snapshot, event);
    snapshot
}

#[tauri::command]
fn save_profile_edn(profile: Profile) -> String {
    profile_codec::save_profile(&profile)
}

#[tauri::command]
fn load_profile_edn(contents: String) -> Result<Profile, String> {
    profile_codec::load_profile(&contents).map_err(|err| err.to_string())
}

#[tauri::command]
fn import_vial_file(contents: String) -> Result<ImportCandidate, String> {
    importers::import_vial_json(&contents).map_err(|err| err.to_string())
}

#[tauri::command]
fn set_overlay_positioning_mode(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window does not exist".to_string())?;
    window
        .set_ignore_cursor_events(!enabled)
        .map_err(|err| err.to_string())?;
    if enabled {
        window.show().map_err(|err| err.to_string())?;
        window.set_focus().map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            create_overlay_window(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            initial_snapshot,
            fake_runtime_events,
            apply_event,
            save_profile_edn,
            load_profile_edn,
            import_vial_file,
            set_overlay_positioning_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Keyplane");
}

fn create_overlay_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("overlay").is_some() {
        return Ok(());
    }

    let overlay = WebviewWindowBuilder::new(
        app,
        "overlay",
        WebviewUrl::App("index.html#/overlay".into()),
    )
    .title("Keyplane Overlay")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .build()?;

    overlay.set_ignore_cursor_events(true)?;
    Ok(())
}
