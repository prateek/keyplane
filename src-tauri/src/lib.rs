pub mod active_profile;
pub mod backend;
pub mod domain;
pub mod fake_backend;
pub mod host_permissions;
pub mod importers;
pub mod kanata_backend;
pub mod kanata_tcp;
pub mod keypeek_backend;
pub mod keypeek_contract;
pub mod keypeek_live;
pub mod overlay_backend;
pub mod profile_codec;
pub mod sentinel_backend;
#[cfg(desktop)]
pub mod sentinel_shortcuts;
pub mod vial_device;

use crate::active_profile::ActiveProfileStore;
use crate::backend::{FakeProtocolBackend, ProtocolBackend};
use crate::domain::{
    apply_runtime_event, BackendConfig, BackendStatus, HealthState, HostInputEvent,
    ImportCandidate, KeyPeekDeviceDiscovery, KeyboardSnapshot, OverlayWindowConfig, Profile,
    RuntimeEvent, SourceConflict, StyleDensity, VisibilityPolicy,
};
use crate::kanata_tcp::{KanataLayerMap, KanataTcpRuntime, KanataTcpSession, TcpKanataTransport};
use crate::keypeek_live::{KeyPeekLiveRuntime, KeyPeekLiveSession, QmkViaRawHidTransport};
#[cfg(desktop)]
use crate::sentinel_shortcuts::SentinelShortcutRuntime;
use serde::Deserialize;
use tauri::{
    Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindowBuilder,
};
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};
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

#[derive(Debug, Deserialize)]
struct KeyPeekConnectionRequest {
    vid: String,
    pid: String,
}

#[derive(Debug, Deserialize)]
struct KanataConnectionRequest {
    host: String,
    port: u16,
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
    overlay_window_snapshot_from_result(
        &active_profile,
        apply_overlay_window_config_to_app(&app, &snapshot.overlay_window),
        "Overlay Window is configured",
        "Could not apply Overlay Window configuration",
    )
}

#[tauri::command]
fn fake_runtime_events() -> Vec<RuntimeEvent> {
    FakeProtocolBackend.demo_events()
}

#[tauri::command]
fn ingest_sentinel_host_input_event(
    active_profile: State<'_, ActiveProfileStore>,
    event: HostInputEvent,
) -> Result<Option<RuntimeEvent>, String> {
    active_profile
        .ingest_sentinel_host_input_event(event)
        .map_err(|err| err.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn register_sentinel_key_shortcuts(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    sentinel_shortcuts: State<'_, SentinelShortcutRuntime>,
) -> Result<KeyboardSnapshot, String> {
    let profile = active_profile
        .profile_snapshot()
        .map_err(|err| err.to_string())?;
    let registrations =
        match sentinel_shortcuts::shortcut_registrations_from_bindings(&profile.sentinel_keys) {
            Ok(registrations) => registrations,
            Err(err @ sentinel_shortcuts::SentinelShortcutError::NoBindings) => {
                return active_profile
                    .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
                        HealthState::Unsupported,
                        err.to_string(),
                    ))
                    .map_err(|err| err.to_string());
            }
            Err(err) => {
                return active_profile
                    .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
                        HealthState::ParseError,
                        err.to_string(),
                    ))
                    .map_err(|err| err.to_string());
            }
        };

    if let Err(err) = unregister_registered_sentinel_shortcuts(&app, &sentinel_shortcuts) {
        return active_profile
            .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
                sentinel_shortcuts::registration_health_state(&err),
                format!("Could not refresh Sentinel Key shortcuts: {err}"),
            ))
            .map_err(|err| err.to_string());
    }

    let mut registered_accelerators = Vec::<String>::new();
    for registration in &registrations {
        if let Err(err) = app
            .global_shortcut()
            .register(registration.accelerator.as_str())
        {
            let message = err.to_string();
            rollback_sentinel_shortcuts(&app, &registered_accelerators);
            sentinel_shortcuts
                .clear_registered()
                .map_err(|err| err.to_string())?;
            return active_profile
                .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
                    sentinel_shortcuts::registration_health_state(&message),
                    format!("Could not register Sentinel Key shortcuts: {message}"),
                ))
                .map_err(|err| err.to_string());
        }
        registered_accelerators.push(registration.accelerator.clone());
    }

    sentinel_shortcuts
        .replace_registered(registrations.clone())
        .map_err(|err| err.to_string())?;
    active_profile
        .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
            HealthState::Ok,
            format!(
                "Registered {} Sentinel Key shortcut{} for Host Input Events",
                registrations.len(),
                if registrations.len() == 1 { "" } else { "s" }
            ),
        ))
        .map_err(|err| err.to_string())
}

#[cfg(not(desktop))]
#[tauri::command]
fn register_sentinel_key_shortcuts(
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<KeyboardSnapshot, String> {
    active_profile
        .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
            HealthState::Unsupported,
            "Sentinel Key shortcut registration is unavailable on this platform",
        ))
        .map_err(|err| err.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn unregister_sentinel_key_shortcuts(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    sentinel_shortcuts: State<'_, SentinelShortcutRuntime>,
) -> Result<KeyboardSnapshot, String> {
    if let Err(err) = unregister_registered_sentinel_shortcuts(&app, &sentinel_shortcuts) {
        return active_profile
            .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
                sentinel_shortcuts::registration_health_state(&err),
                format!("Could not unregister Sentinel Key shortcuts: {err}"),
            ))
            .map_err(|err| err.to_string());
    }

    active_profile
        .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
            HealthState::Disconnected,
            "Sentinel Key shortcuts disabled",
        ))
        .map_err(|err| err.to_string())
}

#[cfg(not(desktop))]
#[tauri::command]
fn unregister_sentinel_key_shortcuts(
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<KeyboardSnapshot, String> {
    active_profile
        .set_runtime_backend_status(sentinel_backend::sentinel_backend_status(
            HealthState::Unsupported,
            "Sentinel Key shortcut registration is unavailable on this platform",
        ))
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn refresh_host_permission_health(
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<KeyboardSnapshot, String> {
    active_profile
        .set_runtime_backend_status(host_permissions::host_permission_backend_status(
            host_permissions::current_host_permission_state(),
        ))
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn request_host_input_permissions(
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<KeyboardSnapshot, String> {
    active_profile
        .set_runtime_backend_status(host_permissions::host_permission_backend_status(
            host_permissions::request_host_input_permissions(),
        ))
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn discover_keypeek_devices(
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<KeyPeekDeviceDiscovery, String> {
    let (devices, status) = match keypeek_live::discover_keypeek_devices() {
        Ok(devices) => {
            let status = keypeek_backend::keypeek_discovery_backend_status(devices.len());
            (devices, status)
        }
        Err(err) => {
            let status = keypeek_backend::keypeek_backend_status(
                keypeek_discovery_error_state(&err),
                format!("Could not discover KeyPeek-compatible VIA Raw HID devices: {err}"),
            );
            (Vec::new(), status)
        }
    };
    let snapshot = active_profile
        .set_runtime_backend_status(status)
        .map_err(|err| err.to_string())?;

    Ok(KeyPeekDeviceDiscovery { devices, snapshot })
}

fn keypeek_discovery_error_state(error: &qmk_via_api::Error) -> HealthState {
    match error {
        qmk_via_api::Error::MaybePermissionDenied(_) => HealthState::PermissionMissing,
        qmk_via_api::Error::UnsupportedFeature(_) => HealthState::Unsupported,
        _ => HealthState::ProtocolError,
    }
}

#[tauri::command]
fn start_keypeek_live_backend(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    live_runtime: State<'_, KeyPeekLiveRuntime>,
    request: KeyPeekConnectionRequest,
) -> Result<KeyboardSnapshot, String> {
    let vid = match keypeek_live::parse_usb_id(&request.vid) {
        Ok(vid) => vid,
        Err(err) => {
            return active_profile
                .set_runtime_backend_status(keypeek_backend::keypeek_backend_status(
                    crate::domain::HealthState::ParseError,
                    format!("Invalid KeyPeek VID: {err}"),
                ))
                .map_err(|err| err.to_string());
        }
    };
    let pid = match keypeek_live::parse_usb_id(&request.pid) {
        Ok(pid) => pid,
        Err(err) => {
            return active_profile
                .set_runtime_backend_status(keypeek_backend::keypeek_backend_status(
                    crate::domain::HealthState::ParseError,
                    format!("Invalid KeyPeek PID: {err}"),
                ))
                .map_err(|err| err.to_string());
        }
    };
    let profile = active_profile
        .profile_snapshot()
        .map_err(|err| err.to_string())?;
    let matrix_key_ids = keypeek_live::matrix_key_ids_from_layout(&profile.physical_layout);
    let layer_count = profile.keymap.layers.len();
    let transport = match QmkViaRawHidTransport::open(vid, pid) {
        Ok(transport) => transport,
        Err(err) => {
            let status = keypeek_backend::keypeek_backend_status(
                crate::domain::HealthState::Disconnected,
                format!("Could not open KeyPeek HID {:04x}:{:04x}: {err}", vid, pid),
            );
            return active_profile
                .set_runtime_backend_status(status)
                .map_err(|err| err.to_string());
        }
    };
    let mut session = KeyPeekLiveSession::new(transport, layer_count, matrix_key_ids);
    if let Err(err) = session.start_subscription() {
        let status = keypeek_backend::keypeek_backend_status(
            crate::domain::HealthState::ProtocolError,
            format!(
                "Could not start KeyPeek live subscription {:04x}:{:04x}: {err}",
                vid, pid
            ),
        );
        return active_profile
            .set_runtime_backend_status(status)
            .map_err(|err| err.to_string());
    }

    let snapshot = active_profile
        .set_runtime_backend_status(keypeek_backend::keypeek_backend_status(
            crate::domain::HealthState::Ok,
            format!(
                "Connected to KeyPeek-compatible HID {:04x}:{:04x}",
                vid, pid
            ),
        ))
        .map_err(|err| err.to_string())?;
    live_runtime.start(app, session)?;
    Ok(snapshot)
}

#[tauri::command]
fn stop_keypeek_live_backend(
    active_profile: State<'_, ActiveProfileStore>,
    live_runtime: State<'_, KeyPeekLiveRuntime>,
) -> Result<KeyboardSnapshot, String> {
    live_runtime.stop();
    active_profile
        .set_runtime_backend_status(keypeek_backend::keypeek_backend_status(
            crate::domain::HealthState::Disconnected,
            "KeyPeek live backend stopped",
        ))
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn start_kanata_tcp_backend(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    kanata_runtime: State<'_, KanataTcpRuntime>,
    request: KanataConnectionRequest,
) -> Result<KeyboardSnapshot, String> {
    let host = request.host.trim();
    if host.is_empty() || request.port == 0 {
        return active_profile
            .set_runtime_backend_status(kanata_backend::kanata_backend_status(
                HealthState::ParseError,
                "Kanata TCP host and port are required",
            ))
            .map_err(|err| err.to_string());
    }

    let kanata_config = BackendConfig::KanataTcp {
        host: host.to_string(),
        port: request.port,
    };
    let profile = active_profile
        .profile_snapshot()
        .map_err(|err| err.to_string())?;
    let transport = match TcpKanataTransport::connect(host, request.port) {
        Ok(transport) => transport,
        Err(err) => {
            return active_profile
                .set_runtime_backend_status(kanata_backend_status_with_config(
                    HealthState::Disconnected,
                    format!(
                        "Could not connect to Kanata TCP {host}:{}: {err}",
                        request.port
                    ),
                    kanata_config,
                ))
                .map_err(|err| err.to_string());
        }
    };
    let mut session = KanataTcpSession::new(
        transport,
        KanataLayerMap::from_layers(&profile.keymap.layers),
    );
    if let Err(err) = session.start() {
        return active_profile
            .set_runtime_backend_status(kanata_backend_status_with_config(
                HealthState::ProtocolError,
                format!(
                    "Could not start Kanata TCP session {host}:{}: {err}",
                    request.port
                ),
                kanata_config,
            ))
            .map_err(|err| err.to_string());
    }

    let snapshot = active_profile
        .set_runtime_backend_status(kanata_backend_status_with_config(
            HealthState::Ok,
            format!("Connected to Kanata TCP {host}:{}", request.port),
            kanata_config,
        ))
        .map_err(|err| err.to_string())?;
    kanata_runtime.start(app, session)?;
    Ok(snapshot)
}

#[tauri::command]
fn stop_kanata_tcp_backend(
    active_profile: State<'_, ActiveProfileStore>,
    kanata_runtime: State<'_, KanataTcpRuntime>,
) -> Result<KeyboardSnapshot, String> {
    kanata_runtime.stop();
    active_profile
        .set_runtime_backend_status(kanata_backend::kanata_backend_status(
            HealthState::Disconnected,
            "Kanata TCP backend stopped",
        ))
        .map_err(|err| err.to_string())
}

fn kanata_backend_status_with_config(
    health: HealthState,
    message: impl Into<String>,
    config: BackendConfig,
) -> BackendStatus {
    let mut status = kanata_backend::kanata_backend_status(health, message);
    status.config = Some(config);
    status
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
    overlay_window_snapshot_from_result(
        &active_profile,
        apply_overlay_window_config_to_app(&app, &snapshot.overlay_window),
        "Overlay Window applied loaded Profile display targeting",
        "Could not apply loaded Profile Overlay Window configuration",
    )
}

#[tauri::command]
fn import_vial_file(contents: String) -> Result<ImportCandidate, String> {
    importers::import_vial_json(&contents).map_err(|err| err.to_string())
}

#[tauri::command]
fn import_via_json_file(contents: String) -> Result<ImportCandidate, String> {
    importers::import_via_json(&contents).map_err(|err| err.to_string())
}

#[tauri::command]
fn import_vial_device(request: KeyPeekConnectionRequest) -> Result<ImportCandidate, String> {
    let vid = keypeek_live::parse_usb_id(&request.vid)
        .map_err(|err| format!("Invalid Vial device VID: {err}"))?;
    let pid = keypeek_live::parse_usb_id(&request.pid)
        .map_err(|err| format!("Invalid Vial device PID: {err}"))?;
    let mut transport = vial_device::QmkViaVialTransport::open(vid, pid)
        .map_err(|err| format!("Could not open Vial HID {:04x}:{:04x}: {err}", vid, pid))?;

    vial_device::import_vial_device(&mut transport, vid, pid).map_err(|err| err.to_string())
}

#[tauri::command]
fn import_keypeek_qmk_info_file(contents: String) -> Result<ImportCandidate, String> {
    importers::import_keypeek_qmk_info_json(&contents).map_err(|err| err.to_string())
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
    overlay_window_snapshot_from_result(
        &active_profile,
        apply_overlay_window_config_to_app(&app, &snapshot.overlay_window),
        "Overlay Window applied imported Profile display targeting",
        "Could not apply imported Profile Overlay Window configuration",
    )
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
    let mut next_snapshot = overlay_window_snapshot_from_result(
        &active_profile,
        apply_overlay_window_config_to_app(&app, &snapshot.overlay_window),
        if enabled {
            "Overlay Window entered Positioning Mode"
        } else {
            "Overlay Window returned to Click-Through Mode"
        },
        "Could not update Overlay Window positioning state",
    )?;

    if enabled {
        next_snapshot = overlay_window_snapshot_from_result(
            &active_profile,
            overlay_window(&app)
                .and_then(|window| window.set_focus().map_err(|err| err.to_string())),
            "Overlay Window is focused for Positioning Mode",
            "Could not focus Overlay Window for Positioning Mode",
        )?;
    }
    Ok(next_snapshot)
}

#[tauri::command]
fn set_overlay_visibility_policy(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    visibility: VisibilityPolicy,
) -> Result<KeyboardSnapshot, String> {
    let snapshot = active_profile
        .set_overlay_visibility_policy(visibility.clone())
        .map_err(|err| err.to_string())?;
    overlay_window_snapshot_from_result(
        &active_profile,
        apply_overlay_window_config_to_app(&app, &snapshot.overlay_window),
        format!("Overlay Visibility Policy set to {visibility:?}"),
        "Could not update Overlay Visibility Policy",
    )
}

#[tauri::command]
fn set_overlay_visible(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    visible: bool,
) -> Result<KeyboardSnapshot, String> {
    let snapshot = active_profile
        .set_overlay_visible(visible)
        .map_err(|err| err.to_string())?;
    overlay_window_snapshot_from_result(
        &active_profile,
        apply_overlay_window_config_to_app(&app, &snapshot.overlay_window),
        if visible {
            "Overlay Window shown"
        } else {
            "Overlay Window hidden"
        },
        "Could not update Overlay Window visibility",
    )
}

#[tauri::command]
fn set_visual_style_density(
    active_profile: State<'_, ActiveProfileStore>,
    density: StyleDensity,
) -> Result<KeyboardSnapshot, String> {
    active_profile
        .set_visual_style_density(density)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn apply_overlay_window_config(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    config: OverlayWindowConfig,
) -> Result<KeyboardSnapshot, String> {
    overlay_window_snapshot_from_result(
        &active_profile,
        apply_overlay_window_config_to_app(&app, &config),
        "Overlay Window configuration applied",
        "Could not apply Overlay Window configuration",
    )
}

#[tauri::command]
fn start_overlay_drag(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
) -> Result<KeyboardSnapshot, String> {
    overlay_window_snapshot_from_result(
        &active_profile,
        overlay_window(&app)
            .and_then(|window| window.start_dragging().map_err(|err| err.to_string())),
        "Overlay Window drag started",
        "Could not start Overlay Window drag",
    )
}

#[tauri::command]
fn start_overlay_resize(
    app: tauri::AppHandle,
    active_profile: State<'_, ActiveProfileStore>,
    direction: OverlayResizeDirection,
) -> Result<KeyboardSnapshot, String> {
    overlay_window_snapshot_from_result(
        &active_profile,
        overlay_window(&app).and_then(|window| {
            window
                .start_resize_dragging(direction.into())
                .map_err(|err| err.to_string())
        }),
        "Overlay Window resize started",
        "Could not start Overlay Window resize",
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .manage(ActiveProfileStore::new(fake_backend::fake_profile()))
        .manage(KeyPeekLiveRuntime::new())
        .manage(KanataTcpRuntime::new());

    #[cfg(desktop)]
    {
        builder = builder.manage(SentinelShortcutRuntime::new());
    }

    builder
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                    None,
                ))?;
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(handle_global_shortcut_event)
                        .build(),
                )?;
            }
            let _ = overlay_window_snapshot_from_result(
                &app.state::<ActiveProfileStore>(),
                create_overlay_window(app.handle()).map_err(|err| err.to_string()),
                "Overlay Window was created",
                "Could not create Overlay Window",
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            initial_snapshot,
            fake_runtime_events,
            ingest_sentinel_host_input_event,
            register_sentinel_key_shortcuts,
            unregister_sentinel_key_shortcuts,
            refresh_host_permission_health,
            request_host_input_permissions,
            discover_keypeek_devices,
            start_keypeek_live_backend,
            stop_keypeek_live_backend,
            start_kanata_tcp_backend,
            stop_kanata_tcp_backend,
            apply_event,
            save_profile_edn,
            load_profile_edn,
            save_active_profile_edn,
            load_active_profile_edn,
            import_vial_file,
            import_via_json_file,
            import_vial_device,
            import_keypeek_qmk_info_file,
            import_keyviz_style_file,
            import_overkeys_companion_file,
            import_zmk_keymap_file,
            commit_import_candidate,
            promote_source_candidate,
            apply_overlay_window_config,
            set_overlay_positioning_mode,
            set_overlay_visibility_policy,
            set_overlay_visible,
            set_visual_style_density,
            start_overlay_drag,
            start_overlay_resize,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Keyplane");
}

#[cfg(desktop)]
fn unregister_registered_sentinel_shortcuts(
    app: &tauri::AppHandle,
    sentinel_shortcuts: &SentinelShortcutRuntime,
) -> Result<(), String> {
    let accelerators = sentinel_shortcuts
        .registered_accelerators()
        .map_err(|err| err.to_string())?;
    let registered: Vec<&str> = accelerators
        .iter()
        .filter(|accelerator| app.global_shortcut().is_registered(accelerator.as_str()))
        .map(String::as_str)
        .collect();

    if !registered.is_empty() {
        app.global_shortcut()
            .unregister_multiple(registered)
            .map_err(|err| err.to_string())?;
    }
    sentinel_shortcuts
        .clear_registered()
        .map_err(|err| err.to_string())
}

#[cfg(desktop)]
fn rollback_sentinel_shortcuts(app: &tauri::AppHandle, accelerators: &[String]) {
    if accelerators.is_empty() {
        return;
    }
    let registered: Vec<&str> = accelerators
        .iter()
        .filter(|accelerator| app.global_shortcut().is_registered(accelerator.as_str()))
        .map(String::as_str)
        .collect();
    if !registered.is_empty() {
        let _ = app.global_shortcut().unregister_multiple(registered);
    }
}

#[cfg(desktop)]
fn handle_global_shortcut_event(app: &tauri::AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    let host_input_code = match app
        .state::<SentinelShortcutRuntime>()
        .host_input_code_for_shortcut(shortcut.id())
    {
        Ok(Some(code)) => code,
        _ => return,
    };
    let pressed = match event.state() {
        ShortcutState::Pressed => true,
        ShortcutState::Released => false,
    };

    match app
        .state::<ActiveProfileStore>()
        .ingest_sentinel_host_input_event(HostInputEvent {
            code: host_input_code,
            pressed,
        }) {
        Ok(Some(runtime_event)) => {
            let _ = app.emit(keypeek_live::RUNTIME_EVENT_NAME, runtime_event);
        }
        Ok(None) => {}
        Err(err) => {
            let _ = app.emit(
                keypeek_live::RUNTIME_EVENT_NAME,
                RuntimeEvent::BackendHealthChanged {
                    health: sentinel_backend::sentinel_backend_status(
                        HealthState::ProtocolError,
                        format!("Could not apply Sentinel Key Host Input Event: {err}"),
                    )
                    .health,
                },
            );
        }
    }
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
    .visible_on_all_workspaces(true)
    .skip_taskbar(true)
    .focused(false)
    .focusable(false)
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
    let displays =
        overlay_displays_for_window(&window, config.display_targeting.display_id.as_deref())?;
    let plan = overlay_window_plan(config, &displays);

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
    window
        .set_focusable(plan.focusable)
        .map_err(|err| err.to_string())?;
    window
        .set_visible_on_all_workspaces(plan.visible_on_all_workspaces)
        .map_err(|err| err.to_string())?;

    if plan.visible {
        window.show().map_err(|err| err.to_string())?;
    } else {
        window.hide().map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
struct OverlayDisplayTarget {
    index: usize,
    id: Option<String>,
    x: f64,
    y: f64,
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
    focusable: bool,
    visible_on_all_workspaces: bool,
}

fn overlay_displays_for_window(
    window: &tauri::Window,
    requested_display_id: Option<&str>,
) -> Result<Vec<OverlayDisplayTarget>, String> {
    if requested_display_id.is_none() {
        return Ok(Vec::new());
    }

    window
        .available_monitors()
        .map_err(|err| format!("Could not resolve Overlay Window display target: {err}"))
        .map(|monitors| {
            monitors
                .iter()
                .enumerate()
                .map(overlay_display_target_from_monitor)
                .collect()
        })
}

fn overlay_display_target_from_monitor(
    (index, monitor): (usize, &tauri::Monitor),
) -> OverlayDisplayTarget {
    let scale_factor = monitor.scale_factor();
    let position = monitor.position();
    OverlayDisplayTarget {
        index,
        id: monitor.name().cloned(),
        x: f64::from(position.x) / scale_factor,
        y: f64::from(position.y) / scale_factor,
    }
}

fn overlay_window_plan(
    config: &OverlayWindowConfig,
    displays: &[OverlayDisplayTarget],
) -> OverlayWindowPlan {
    let target = &config.display_targeting;
    let (display_x, display_y) = target
        .display_id
        .as_deref()
        .and_then(|display_id| matching_overlay_display(display_id, displays))
        .map(|display| (display.x, display.y))
        .unwrap_or((0.0, 0.0));

    OverlayWindowPlan {
        x: display_x + finite_or_default(target.x, 72.0),
        y: display_y + finite_or_default(target.y, 72.0),
        width: clamp_window_dimension(target.width, 320.0),
        height: clamp_window_dimension(target.height, 180.0),
        visible: config.visible,
        ignore_cursor_events: config.click_through && !config.positioning_mode,
        resizable: config.positioning_mode,
        focusable: config.positioning_mode,
        visible_on_all_workspaces: true,
    }
}

fn matching_overlay_display<'a>(
    display_id: &str,
    displays: &'a [OverlayDisplayTarget],
) -> Option<&'a OverlayDisplayTarget> {
    displays.iter().find(|display| {
        display.id.as_deref() == Some(display_id)
            || display_id == display.index.to_string()
            || display_id == format!("display-{}", display.index)
    })
}

fn overlay_window_snapshot_from_result(
    active_profile: &ActiveProfileStore,
    result: Result<(), String>,
    success_message: impl Into<String>,
    failure_prefix: impl Into<String>,
) -> Result<KeyboardSnapshot, String> {
    let status = match result {
        Ok(()) => {
            overlay_backend::overlay_window_backend_status(HealthState::Ok, success_message.into())
        }
        Err(err) => overlay_backend::overlay_window_backend_status(
            HealthState::Unsupported,
            format!("{}: {err}", failure_prefix.into()),
        ),
    };

    active_profile
        .set_runtime_backend_status(status)
        .map_err(|err| err.to_string())
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
    fn keypeek_discovery_errors_map_to_typed_backend_health() {
        assert_eq!(
            keypeek_discovery_error_state(&qmk_via_api::Error::MaybePermissionDenied(
                "hid unavailable".to_string()
            )),
            HealthState::PermissionMissing
        );
        assert_eq!(
            keypeek_discovery_error_state(&qmk_via_api::Error::UnsupportedFeature("scan")),
            HealthState::Unsupported
        );
        assert_eq!(
            keypeek_discovery_error_state(&qmk_via_api::Error::Hid("boom".to_string())),
            HealthState::ProtocolError
        );
    }

    #[test]
    fn kanata_backend_status_with_config_carries_connection_settings() {
        let status = kanata_backend_status_with_config(
            HealthState::Disconnected,
            "Could not connect",
            BackendConfig::KanataTcp {
                host: "10.0.0.20".to_string(),
                port: 4039,
            },
        );

        assert_eq!(
            status.config,
            Some(BackendConfig::KanataTcp {
                host: "10.0.0.20".to_string(),
                port: 4039,
            })
        );
    }

    #[test]
    fn apply_event_command_uses_rust_effective_action_resolution() {
        let snapshot = crate::fake_backend::initial_snapshot();
        let event = crate::fake_backend::demo_runtime_events()
            .into_iter()
            .find(|event| matches!(event, RuntimeEvent::LayerStackChanged { .. }))
            .expect("demo includes a layer-stack Runtime Event");

        let snapshot = apply_event(snapshot, event);
        let effective_key = snapshot
            .effective_keys
            .iter()
            .find(|key| key.key_id == "k-q")
            .expect("k-q effective key exists");

        assert_eq!(snapshot.runtime_state.layer_stack[0].layer_id, "layer-1");
        assert_eq!(effective_key.source_layer_id, "layer-0");
        assert!(effective_key.inherited);
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

        let plan = overlay_window_plan(&config, &[]);

        assert_eq!(plan.x, 140.0);
        assert_eq!(plan.y, 92.0);
        assert_eq!(plan.width, 880.0);
        assert_eq!(plan.height, 280.0);
        assert!(plan.visible);
        assert!(plan.ignore_cursor_events);
        assert!(!plan.resizable);
        assert!(!plan.focusable);
        assert!(plan.visible_on_all_workspaces);
    }

    #[test]
    fn overlay_window_plan_offsets_profile_position_by_matching_display_target() {
        let mut config = crate::fake_backend::fake_profile().overlay_window;
        config.display_targeting.display_id = Some("Studio Display".to_string());
        config.display_targeting.x = 24.0;
        config.display_targeting.y = 36.0;

        let plan = overlay_window_plan(
            &config,
            &[
                OverlayDisplayTarget {
                    index: 0,
                    id: Some("Built-in Display".to_string()),
                    x: 0.0,
                    y: 0.0,
                },
                OverlayDisplayTarget {
                    index: 1,
                    id: Some("Studio Display".to_string()),
                    x: 1728.0,
                    y: -120.0,
                },
            ],
        );

        assert_eq!(plan.x, 1752.0);
        assert_eq!(plan.y, -84.0);
    }

    #[test]
    fn overlay_window_plan_supports_display_ordinal_targets_and_stale_target_fallback() {
        let mut config = crate::fake_backend::fake_profile().overlay_window;
        config.display_targeting.display_id = Some("display-1".to_string());
        config.display_targeting.x = 10.0;
        config.display_targeting.y = 20.0;
        let displays = [OverlayDisplayTarget {
            index: 1,
            id: None,
            x: -1280.0,
            y: 0.0,
        }];

        let matched = overlay_window_plan(&config, &displays);
        assert_eq!(matched.x, -1270.0);
        assert_eq!(matched.y, 20.0);

        config.display_targeting.display_id = Some("missing-display".to_string());
        let fallback = overlay_window_plan(&config, &displays);
        assert_eq!(fallback.x, 10.0);
        assert_eq!(fallback.y, 20.0);
    }

    #[test]
    fn overlay_window_plan_switches_to_interactive_positioning_mode() {
        let mut config = crate::fake_backend::fake_profile().overlay_window;
        config.positioning_mode = true;
        config.click_through = false;

        let plan = overlay_window_plan(&config, &[]);

        assert!(!plan.ignore_cursor_events);
        assert!(plan.resizable);
        assert!(plan.focusable);
    }

    #[test]
    fn overlay_window_plan_clamps_unusable_sizes() {
        let mut config = crate::fake_backend::fake_profile().overlay_window;
        config.display_targeting.width = 0.0;
        config.display_targeting.height = f32::NAN;
        config.visible = false;

        let plan = overlay_window_plan(&config, &[]);

        assert_eq!(plan.width, 320.0);
        assert_eq!(plan.height, 180.0);
        assert!(!plan.visible);
    }

    #[test]
    fn overlay_window_plan_does_not_hide_non_pinned_policies_by_default() {
        let mut config = crate::fake_backend::fake_profile().overlay_window;
        config.visibility = VisibilityPolicy::ManualToggle;
        config.visible = true;

        let manual_plan = overlay_window_plan(&config, &[]);
        assert!(manual_plan.visible);

        config.visibility = VisibilityPolicy::Fade;
        let fade_plan = overlay_window_plan(&config, &[]);
        assert!(fade_plan.visible);
    }

    #[test]
    fn overlay_window_result_surfaces_unsupported_health_in_snapshot() {
        let active_profile = ActiveProfileStore::new(crate::fake_backend::fake_profile());

        let snapshot = overlay_window_snapshot_from_result(
            &active_profile,
            Err("cursor event ignoring is unsupported".to_string()),
            "Overlay Window is configured",
            "Could not apply Overlay Window configuration",
        )
        .expect("snapshot should be returned with health");

        let health = snapshot
            .runtime_state
            .backend_health
            .iter()
            .find(|health| health.backend_id == overlay_backend::OVERLAY_WINDOW_BACKEND_ID)
            .expect("overlay health exists");
        assert_eq!(health.state, HealthState::Unsupported);
        assert!(health
            .message
            .contains("Could not apply Overlay Window configuration"));
        assert!(health
            .message
            .contains("cursor event ignoring is unsupported"));
    }

    #[test]
    fn overlay_window_result_surfaces_all_workspaces_unsupported_health() {
        let active_profile = ActiveProfileStore::new(crate::fake_backend::fake_profile());

        let snapshot = overlay_window_snapshot_from_result(
            &active_profile,
            Err("all-workspaces behavior is unsupported".to_string()),
            "Overlay Window is configured",
            "Could not apply Overlay Window configuration",
        )
        .expect("snapshot should be returned with health");

        let health = snapshot
            .runtime_state
            .backend_health
            .iter()
            .find(|health| health.backend_id == overlay_backend::OVERLAY_WINDOW_BACKEND_ID)
            .expect("overlay health exists");
        assert_eq!(health.state, HealthState::Unsupported);
        assert!(health
            .message
            .contains("all-workspaces behavior is unsupported"));
    }

    #[test]
    fn overlay_window_result_restores_ok_health_after_success() {
        let active_profile = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        overlay_window_snapshot_from_result(
            &active_profile,
            Err("not supported".to_string()),
            "Overlay Window is configured",
            "Could not apply Overlay Window configuration",
        )
        .expect("failure snapshot");

        let snapshot = overlay_window_snapshot_from_result(
            &active_profile,
            Ok(()),
            "Overlay Window is configured",
            "Could not apply Overlay Window configuration",
        )
        .expect("success snapshot");

        let health = snapshot
            .runtime_state
            .backend_health
            .iter()
            .find(|health| health.backend_id == overlay_backend::OVERLAY_WINDOW_BACKEND_ID)
            .expect("overlay health exists");
        assert_eq!(health.state, HealthState::Ok);
        assert_eq!(health.message, "Overlay Window is configured");
    }
}
