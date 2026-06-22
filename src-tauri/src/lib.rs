pub mod active_profile;
pub mod backend;
pub mod domain;
pub mod fake_backend;
pub mod importers;
pub mod keypeek_backend;
pub mod profile_codec;

use crate::active_profile::ActiveProfileStore;
use crate::backend::{FakeProtocolBackend, ProtocolBackend};
use crate::domain::{
    apply_runtime_event, ImportCandidate, KeyboardSnapshot, Profile, RuntimeEvent, SourceConflict,
};
use serde::Deserialize;
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_runtime::ResizeDirection;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum OverlayResizeDirection {
    East,
    North,
    NorthEast,
    NorthWest,
    South,
    SouthEast,
    SouthWest,
    West,
}

impl From<OverlayResizeDirection> for ResizeDirection {
    fn from(direction: OverlayResizeDirection) -> Self {
        match direction {
            OverlayResizeDirection::East => ResizeDirection::East,
            OverlayResizeDirection::North => ResizeDirection::North,
            OverlayResizeDirection::NorthEast => ResizeDirection::NorthEast,
            OverlayResizeDirection::NorthWest => ResizeDirection::NorthWest,
            OverlayResizeDirection::South => ResizeDirection::South,
            OverlayResizeDirection::SouthEast => ResizeDirection::SouthEast,
            OverlayResizeDirection::SouthWest => ResizeDirection::SouthWest,
            OverlayResizeDirection::West => ResizeDirection::West,
        }
    }
}

#[tauri::command]
fn initial_snapshot(
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<KeyboardSnapshot, String> {
    active_profile.snapshot().map_err(|err| err.to_string())
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
fn save_active_profile_edn(
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<String, String> {
    active_profile
        .save_profile_edn()
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn load_active_profile_edn(
    active_profile: State<'_, ActiveProfileStore>,
    contents: String,
) -> Result<KeyboardSnapshot, String> {
    let profile = profile_codec::load_profile(&contents).map_err(|err| err.to_string())?;
    active_profile
        .load_profile(profile)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn import_vial_file(contents: String) -> Result<ImportCandidate, String> {
    importers::import_vial_json(&contents).map_err(|err| err.to_string())
}

#[tauri::command]
fn import_keyviz_style_file(
    active_profile: State<'_, ActiveProfileStore>,
    contents: String,
) -> Result<ImportCandidate, String> {
    let profile = active_profile
        .profile_snapshot()
        .map_err(|err| err.to_string())?;
    importers::import_keyviz_style_json(&contents, &profile).map_err(|err| err.to_string())
}

#[tauri::command]
fn import_overkeys_companion_file(contents: String) -> Result<ImportCandidate, String> {
    importers::import_overkeys_companion_json(&contents).map_err(|err| err.to_string())
}

#[tauri::command]
fn commit_import_candidate(
    active_profile: State<'_, ActiveProfileStore>,
    candidate: ImportCandidate,
) -> Result<KeyboardSnapshot, String> {
    active_profile
        .commit_import_candidate(candidate)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn promote_source_candidate(
    active_profile: State<'_, ActiveProfileStore>,
    conflict: SourceConflict,
    source_id: String,
) -> Result<KeyboardSnapshot, String> {
    active_profile
        .promote_source_candidate(conflict, &source_id)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn set_overlay_positioning_mode(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let window = overlay_window(&app)?;
    window
        .set_ignore_cursor_events(!enabled)
        .map_err(|err| err.to_string())?;
    window
        .set_resizable(enabled)
        .map_err(|err| err.to_string())?;
    if enabled {
        window.show().map_err(|err| err.to_string())?;
        window.set_focus().map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn start_overlay_drag(app: tauri::AppHandle) -> Result<(), String> {
    overlay_window(&app)?
        .start_dragging()
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn start_overlay_resize(
    app: tauri::AppHandle,
    direction: OverlayResizeDirection,
) -> Result<(), String> {
    overlay_window(&app)?
        .start_resize_dragging(direction.into())
        .map_err(|err| err.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ActiveProfileStore::new(fake_backend::fake_profile()))
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
            save_active_profile_edn,
            load_active_profile_edn,
            import_vial_file,
            import_keyviz_style_file,
            import_overkeys_companion_file,
            commit_import_candidate,
            promote_source_candidate,
            set_overlay_positioning_mode,
            start_overlay_drag,
            start_overlay_resize,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Keyplane");
}

fn overlay_window(app: &tauri::AppHandle) -> Result<tauri::Window, String> {
    app.get_window("overlay")
        .ok_or_else(|| "overlay window does not exist".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_resize_direction_maps_to_tauri_runtime_direction() {
        assert_eq!(
            ResizeDirection::from(OverlayResizeDirection::SouthEast),
            ResizeDirection::SouthEast
        );
        assert_eq!(
            ResizeDirection::from(OverlayResizeDirection::NorthWest),
            ResizeDirection::NorthWest
        );
    }
}
