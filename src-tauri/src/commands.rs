//! The Tauri command boundary (ADR 0015, 0036).
//!
//! Commands are thin: they serialize `keyplane-core` DTOs to the frontend and
//! translate UI intents (import, hand edit, positioning) into core calls. No
//! keyboard logic lives here.

use crate::state::{AppState, Backend, EVENT_SNAPSHOT};
use keyplane_core::import::{
    ImportCandidate, ImportReview, Importer, KeyvizStyleImporter, OverKeysImporter, VialFileImporter,
};
use keyplane_core::profile::Profile;
use keyplane_core::snapshot::KeyboardSnapshot;
use keyplane_kanata::{KanataBackend, LayerMap};
use keyplane_keypeek::{ConnectionSpec, KeyPeekBackend};
use tauri::{AppHandle, Emitter, Manager, State};

/// The current fully-resolved snapshot for first paint.
#[tauri::command]
pub fn get_snapshot(state: State<AppState>) -> KeyboardSnapshot {
    state.inner.lock().expect("state").composer.snapshot()
}

/// The active Profile as structured data.
#[tauri::command]
pub fn get_profile(state: State<AppState>) -> Profile {
    state.inner.lock().expect("state").profile.clone()
}

/// The active Profile as hand-editable EDN text (ADR 0020).
#[tauri::command]
pub fn active_profile_edn(state: State<AppState>) -> String {
    state.inner.lock().expect("state").profile.to_edn_str()
}

/// Apply hand-edited EDN, replacing the active profile and re-resolving.
#[tauri::command]
pub fn apply_profile_edn(
    app: AppHandle,
    state: State<AppState>,
    edn: String,
) -> Result<Profile, String> {
    let profile = Profile::from_edn_str(&edn).map_err(|e| e.to_string())?;
    let mut inner = state.inner.lock().expect("state");
    inner.profile = profile.clone();
    inner.rebuild_from_profile();
    let snapshot = inner.composer.snapshot();
    drop(inner);
    let _ = app.emit(EVENT_SNAPSHOT, &snapshot);
    Ok(profile)
}

/// Preview an import against the active profile, surfacing Source Conflicts.
/// Never mutates the profile (ADR 0034).
#[tauri::command]
pub fn import_preview(
    state: State<AppState>,
    format: String,
    contents: String,
) -> Result<ImportReview, String> {
    let candidate = run_importer(&format, &contents)?;
    let inner = state.inner.lock().expect("state");
    Ok(ImportReview::build(Some(&inner.profile), &candidate))
}

/// Commit an import as a new active profile and re-resolve the overlay.
#[tauri::command]
pub fn commit_import(
    app: AppHandle,
    state: State<AppState>,
    format: String,
    contents: String,
) -> Result<Profile, String> {
    let candidate = run_importer(&format, &contents)?;
    let profile = candidate.into_new_profile("imported-profile");
    let mut inner = state.inner.lock().expect("state");
    inner.profile = profile.clone();
    inner.rebuild_from_profile();
    let snapshot = inner.composer.snapshot();
    drop(inner);
    let _ = app.emit(EVENT_SNAPSHOT, &snapshot);
    Ok(profile)
}

/// Connect a live KeyPeek-supported device (Vial by `vid`/`pid`, or VIA by a
/// keyboard-definition `json_path`), reusing KeyPeek's protocol code. Replaces
/// the active backend + model and starts streaming live Layer Stack changes.
/// Hardware-gated: errors when no supported device is connected.
#[tauri::command]
pub fn connect_keypeek(
    app: AppHandle,
    state: State<AppState>,
    kind: String,
    vid: Option<u16>,
    pid: Option<u16>,
    json_path: Option<String>,
    layout: Option<String>,
) -> Result<Profile, String> {
    let spec = match kind.as_str() {
        "vial" => ConnectionSpec::Vial {
            vid: vid.ok_or("vial requires vid")?,
            pid: pid.ok_or("vial requires pid")?,
        },
        "via" => ConnectionSpec::Via {
            json_path: json_path.ok_or("via requires json_path")?,
        },
        other => return Err(format!("unknown KeyPeek connection kind: {other}")),
    };

    let (backend, model) = KeyPeekBackend::connect(spec, layout.as_deref())?;
    let profile = Profile::new("keypeek-profile", model.clone());
    let mut inner = state.inner.lock().expect("state");
    inner.set_backend(Backend::KeyPeek(backend), model);
    inner.profile = profile.clone();
    let snapshot = inner.composer.snapshot();
    drop(inner);
    let _ = app.emit(EVENT_SNAPSHOT, &snapshot);
    Ok(profile)
}

/// Connect to a running Kanata instance's TCP server (ADR 0009). Kanata is
/// authoritative for runtime layer state but supplies no keyboard data, so the
/// current Active Profile is used as the companion model (ADR 0010): its layer
/// names map Kanata's `LayerChange` events onto the rendered layers. Start
/// Kanata with `--port <port>`.
#[tauri::command]
pub fn connect_kanata(
    app: AppHandle,
    state: State<AppState>,
    host: Option<String>,
    port: u16,
) -> Result<(), String> {
    let mut inner = state.inner.lock().expect("state");

    // Build the Kanata layer-name → companion LayerId map from the active model.
    let model = &inner.profile.model;
    let pairs = model.keymap.layers.iter().map(|l| {
        let name = l.name.clone().unwrap_or_else(|| l.id.to_string());
        (name, l.id.clone())
    });
    let base = model
        .base_layer()
        .unwrap_or_else(|| keyplane_core::ids::LayerId::new("layer-0"));
    let layers = LayerMap::new(pairs, base);

    let host = host.unwrap_or_else(|| "127.0.0.1".to_string());
    let backend = KanataBackend::connect((host.as_str(), port), layers)?;
    inner.set_backend_keep_model(Backend::Kanata(backend));
    let snapshot = inner.composer.snapshot();
    drop(inner);
    let _ = app.emit(EVENT_SNAPSHOT, &snapshot);
    Ok(())
}

/// Promote a value to a User Override so future imports cannot replace it
/// (ADR 0018). Recorded in the profile and persisted in EDN; applying overrides
/// to live resolution is tracked as follow-up work.
#[tauri::command]
pub fn promote_override(
    app: AppHandle,
    state: State<AppState>,
    field: String,
    value: serde_json::Value,
    note: Option<String>,
) -> Profile {
    let mut inner = state.inner.lock().expect("state");
    keyplane_core::import::promote_override(&mut inner.profile, field, value, note);
    // User Overrides win immediately: re-resolve and repaint the overlay.
    inner.rebuild_from_profile();
    let snapshot = inner.composer.snapshot();
    let profile = inner.profile.clone();
    drop(inner);
    let _ = app.emit(EVENT_SNAPSHOT, &snapshot);
    profile
}

/// Toggle Positioning Mode: disable click-through and allow resize so the user
/// can move/size the overlay, or re-enable click-through when done (ADR 0025).
#[tauri::command]
pub fn set_positioning_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or("overlay window not found")?;
    overlay
        .set_ignore_cursor_events(!enabled)
        .map_err(|e| e.to_string())?;
    let _ = overlay.set_resizable(enabled);
    if enabled {
        let _ = overlay.set_focus();
    }
    Ok(())
}

/// Show or hide the Overlay Window.
#[tauri::command]
pub fn set_overlay_visible(app: AppHandle, visible: bool) -> Result<(), String> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or("overlay window not found")?;
    if visible {
        overlay.show().map_err(|e| e.to_string())
    } else {
        overlay.hide().map_err(|e| e.to_string())
    }
}

/// Dispatch an import by format string to the matching Importer.
fn run_importer(format: &str, contents: &str) -> Result<ImportCandidate, String> {
    let result = match format {
        "vial" | "vil" => VialFileImporter::new().import(contents),
        "overkeys" => OverKeysImporter::new().import(contents),
        "keyviz" => KeyvizStyleImporter::new().import(contents),
        other => return Err(format!("unknown import format: {other}")),
    };
    result.map_err(|e| e.to_string())
}
