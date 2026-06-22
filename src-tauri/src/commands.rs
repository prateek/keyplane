//! The Tauri command boundary (ADR 0015, 0036).
//!
//! Commands are thin: they serialize `keyplane-core` DTOs to the frontend and
//! translate UI intents (import, hand edit, positioning) into core calls. No
//! keyboard logic lives here.

use crate::state::{AppState, Backend, EVENT_SNAPSHOT};
use keyplane_core::import::{
    ImportCandidate, ImportReview, Importer, KeyvizStyleImporter, OverKeysImporter, VialFileImporter,
    ZmkKeymapImporter,
};
use keyplane_core::profile::{DisplayTargeting, Profile};
use keyplane_core::snapshot::KeyboardSnapshot;
use crate::state::EVENT_RUNTIME;
use keyplane_kanata::{KanataBackend, LayerMap};
use keyplane_keypeek::{ConnectionSpec, KeyPeekBackend, ZmkTransportConfig};
use keyplane_sentinel::{SentinelAction, SentinelBackend, SentinelKey};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, State};

/// A sentinel-key config entry from the frontend.
#[derive(Deserialize)]
pub struct SentinelKeyDto {
    pub host_key: String,
    /// "momentary" or "toggle".
    pub action: String,
    pub layer: String,
}

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
#[allow(clippy::too_many_arguments)] // Tauri command: one optional param per transport field.
pub fn connect_keypeek(
    app: AppHandle,
    state: State<AppState>,
    kind: String,
    vid: Option<u16>,
    pid: Option<u16>,
    json_path: Option<String>,
    layout: Option<String>,
    serial_port: Option<String>,
    ble_id: Option<String>,
) -> Result<Profile, String> {
    let spec = match kind.as_str() {
        "vial" => ConnectionSpec::Vial {
            vid: vid.ok_or("vial requires vid")?,
            pid: pid.ok_or("vial requires pid")?,
        },
        "via" => ConnectionSpec::Via {
            json_path: json_path.ok_or("via requires json_path")?,
        },
        "zmk" => {
            let transport = if let Some(port) = serial_port {
                ZmkTransportConfig::Serial(port)
            } else if let Some(id) = ble_id {
                ZmkTransportConfig::Ble(id)
            } else {
                return Err("zmk requires serial_port or ble_id".to_string());
            };
            ConnectionSpec::Zmk {
                vid: vid.ok_or("zmk requires vid")?,
                pid: pid.ok_or("zmk requires pid")?,
                transport,
            }
        }
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

/// Enable the sentinel-key backend (ADR 0016): a lower-confidence source that
/// infers layer changes from Host Input Events when no authoritative source is
/// available. Capturing real OS key events is the remaining transport; events
/// are fed via [`feed_host_event`]. Keeps the current model.
#[tauri::command]
pub fn connect_sentinel(
    app: AppHandle,
    state: State<AppState>,
    keys: Vec<SentinelKeyDto>,
    os_capture: Option<bool>,
) -> Result<(), String> {
    let mut inner = state.inner.lock().expect("state");
    let base = inner
        .profile
        .model
        .base_layer()
        .unwrap_or_else(|| keyplane_core::ids::LayerId::new("layer-0"));
    let sentinel_keys = keys
        .into_iter()
        .map(|k| {
            let layer = keyplane_core::ids::LayerId::new(k.layer);
            let action = match k.action.as_str() {
                "toggle" => SentinelAction::Toggle(layer),
                _ => SentinelAction::Momentary(layer),
            };
            SentinelKey {
                host_key: k.host_key,
                action,
            }
        })
        .collect();
    inner.set_backend_keep_model(Backend::Sentinel(SentinelBackend::new(sentinel_keys, base)));
    let snapshot = inner.composer.snapshot();
    drop(inner);
    let _ = app.emit(EVENT_SNAPSHOT, &snapshot);

    // Opt-in OS-level key capture. Off by default; starts a global key listener
    // and surfaces missing OS permission as Backend Health (ADR 0023).
    if os_capture.unwrap_or(false) {
        crate::capture::start_sentinel_capture(app);
    }
    Ok(())
}

/// Feed one Host Input Event to the sentinel backend and emit any resulting
/// Layer Stack change immediately. A no-op unless the sentinel backend is active.
#[tauri::command]
pub fn feed_host_event(app: AppHandle, state: State<AppState>, key: String, down: bool) {
    let mut inner = state.inner.lock().expect("state");
    let backend_id = inner.backend.id();
    inner.backend.feed_host_event(key, down);
    let updates = inner.backend.poll();
    for update in updates {
        if let Some(event) = inner.composer.apply(&backend_id, update) {
            let _ = app.emit(EVENT_RUNTIME, &event);
        }
    }
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
    // Tell the overlay UI to show or hide its drag/resize affordances.
    let _ = app.emit_to("overlay", crate::state::EVENT_POSITIONING, enabled);
    Ok(())
}

/// Apply Profile-owned Display Targeting to the overlay window (ADR 0027). When
/// a monitor name is set, the overlay is placed on that monitor (with `x`/`y` as
/// an offset within it); otherwise `x`/`y` are global logical coordinates.
pub fn apply_display_targeting(window: &tauri::WebviewWindow, display: &DisplayTargeting) {
    if let (Some(w), Some(h)) = (display.width, display.height) {
        let _ = window.set_size(tauri::LogicalSize::new(w, h));
    }

    if let Some(name) = &display.monitor {
        if let Ok(monitors) = window.available_monitors() {
            if let Some(monitor) = monitors.iter().find(|m| m.name() == Some(name)) {
                let origin = monitor.position();
                let _ = window.set_position(tauri::PhysicalPosition::new(
                    origin.x + display.x.unwrap_or(0.0) as i32,
                    origin.y + display.y.unwrap_or(0.0) as i32,
                ));
                return;
            }
        }
    }

    if let (Some(x), Some(y)) = (display.x, display.y) {
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    }
}

/// Update the active Profile's Display Targeting and apply it immediately.
#[tauri::command]
pub fn set_display_targeting(
    app: AppHandle,
    state: State<AppState>,
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<Profile, String> {
    let mut inner = state.inner.lock().expect("state");
    let display = &mut inner.profile.overlay.display;
    display.x = x.or(display.x);
    display.y = y.or(display.y);
    display.width = width.or(display.width);
    display.height = height.or(display.height);
    let display = inner.profile.overlay.display.clone();
    let profile = inner.profile.clone();
    drop(inner);
    if let Some(overlay) = app.get_webview_window("overlay") {
        apply_display_targeting(&overlay, &display);
    }
    Ok(profile)
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
        "zmk" | "keymap" => ZmkKeymapImporter::new().import(contents),
        other => return Err(format!("unknown import format: {other}")),
    };
    result.map_err(|e| e.to_string())
}
