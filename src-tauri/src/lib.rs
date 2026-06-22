pub mod active_profile;
pub mod backend;
pub mod domain;
pub mod fake_backend;
pub mod importers;
pub mod kanata_backend;
pub mod keypeek_backend;
pub mod keypeek_contract;
pub mod profile_codec;

use crate::active_profile::ActiveProfileStore;
use crate::backend::{FakeProtocolBackend, ProtocolBackend};
use crate::domain::{
    apply_runtime_event, ImportCandidate, KeyboardSnapshot, OverlayWindowConfig, Profile,
    RuntimeEvent, SourceConflict, VisibilityPolicy,
};
use serde::Deserialize;
use tauri::{LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindowBuilder};
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
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<KeyboardSnapshot, String> {
    let snapshot = active_profile.snapshot().map_err(|err| err.to_string())?;
    apply_overlay_window_config_to_app(&app, &snapshot.overlay_window)?;
    Ok(snapshot)
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
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    contents: String,
) -> Result<KeyboardSnapshot, String> {
    let profile = profile_codec::load_profile(&contents).map_err(|err| err.to_string())?;
    let snapshot = active_profile
        .load_profile(profile)
        .map_err(|err| err.to_string())?;
    apply_overlay_window_config_to_app(&app, &snapshot.overlay_window)?;
    Ok(snapshot)
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
fn import_zmk_keymap_file(contents: String) -> Result<ImportCandidate, String> {
    importers::import_zmk_keymap(&contents).map_err(|err| err.to_string())
}

#[tauri::command]
fn commit_import_candidate(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    candidate: ImportCandidate,
) -> Result<KeyboardSnapshot, String> {
    let snapshot = active_profile
        .commit_import_candidate(candidate)
        .map_err(|err| err.to_string())?;
    apply_overlay_window_config_to_app(&app, &snapshot.overlay_window)?;
    Ok(snapshot)
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
fn set_overlay_positioning_mode(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    enabled: bool,
) -> Result<KeyboardSnapshot, String> {
    let snapshot = active_profile
        .set_overlay_positioning_mode(enabled)
        .map_err(|err| err.to_string())?;
    apply_overlay_window_config_to_app(&app, &snapshot.overlay_window)?;
    if enabled {
        overlay_window(&app)?
            .set_focus()
            .map_err(|err| err.to_string())?;
    }
    Ok(snapshot)
}

#[tauri::command]
fn apply_overlay_window_config(
    app: tauri::AppHandle,
    config: OverlayWindowConfig,
) -> Result<(), String> {
    apply_overlay_window_config_to_app(&app, &config)
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
            import_zmk_keymap_file,
            commit_import_candidate,
            promote_source_candidate,
            apply_overlay_window_config,
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

fn apply_overlay_window_config_to_app(
    app: &tauri::AppHandle,
    config: &OverlayWindowConfig,
) -> Result<(), String> {
    let window = overlay_window(app)?;
    let plan = overlay_window_plan(config);

    window
        .set_position(LogicalPosition::new(plan.x, plan.y))
        .map_err(|err| err.to_string())?;
    window
        .set_size(LogicalSize::new(plan.width, plan.height))
        .map_err(|err| err.to_string())?;
    window
        .set_ignore_cursor_events(plan.ignore_cursor_events)
        .map_err(|err| err.to_string())?;
    window
        .set_resizable(plan.resizable)
        .map_err(|err| err.to_string())?;

    if plan.visible {
        window.show().map_err(|err| err.to_string())?;
    } else {
        window.hide().map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
struct OverlayWindowPlan {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    visible: bool,
    ignore_cursor_events: bool,
    resizable: bool,
}

fn overlay_window_plan(config: &OverlayWindowConfig) -> OverlayWindowPlan {
    let target = &config.display_targeting;
    OverlayWindowPlan {
        x: finite_or_default(target.x, 72.0),
        y: finite_or_default(target.y, 72.0),
        width: clamp_window_dimension(target.width, 320.0),
        height: clamp_window_dimension(target.height, 180.0),
        visible: config.visibility == VisibilityPolicy::Pinned,
        ignore_cursor_events: config.click_through && !config.positioning_mode,
        resizable: config.positioning_mode,
    }
}

fn clamp_window_dimension(value: f32, min: f64) -> f64 {
    finite_or_default(value, min).max(min)
}

fn finite_or_default(value: f32, default: f64) -> f64 {
    let value = f64::from(value);
    if value.is_finite() {
        value
    } else {
        default
    }
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

    #[test]
    fn overlay_window_plan_uses_profile_display_targeting() {
        let mut config = crate::fake_backend::fake_profile().overlay_window;
        config.display_targeting.x = 140.0;
        config.display_targeting.y = 92.0;
        config.display_targeting.width = 880.0;
        config.display_targeting.height = 280.0;
        config.visibility = VisibilityPolicy::Pinned;
        config.click_through = true;
        config.positioning_mode = false;

        let plan = overlay_window_plan(&config);

        assert_eq!(plan.x, 140.0);
        assert_eq!(plan.y, 92.0);
        assert_eq!(plan.width, 880.0);
        assert_eq!(plan.height, 280.0);
        assert!(plan.visible);
        assert!(plan.ignore_cursor_events);
        assert!(!plan.resizable);
    }

    #[test]
    fn overlay_window_plan_switches_to_interactive_positioning_mode() {
        let mut config = crate::fake_backend::fake_profile().overlay_window;
        config.positioning_mode = true;
        config.click_through = false;

        let plan = overlay_window_plan(&config);

        assert!(!plan.ignore_cursor_events);
        assert!(plan.resizable);
    }

    #[test]
    fn overlay_window_plan_clamps_unusable_sizes() {
        let mut config = crate::fake_backend::fake_profile().overlay_window;
        config.display_targeting.width = 0.0;
        config.display_targeting.height = f32::NAN;
        config.visibility = VisibilityPolicy::ManualToggle;

        let plan = overlay_window_plan(&config);

        assert_eq!(plan.width, 320.0);
        assert_eq!(plan.height, 180.0);
        assert!(!plan.visible);
    }
}
