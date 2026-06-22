use crate::domain::{BackendHealth, HealthState, PhysicalLayout, RuntimeEvent};
use crate::keypeek_backend::{
    keypeek_backend_status, keypeek_subscribe_message, parse_keypeek_pressed_key_packet,
    runtime_event_from_keypeek_layer_packet,
};
use qmk_via_api::api::KeyboardApi;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Emitter;
use thiserror::Error;

pub const RUNTIME_EVENT_NAME: &str = "runtime-event";

const RAW_HID_USAGE_PAGE: u16 = 0xff60;
const MAX_CONSECUTIVE_READ_ERRORS: u8 = 5;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KeyPeekLiveError {
    #[error("invalid USB id {0}")]
    InvalidUsbId(String),
    #[error("{0}")]
    Transport(String),
}

pub trait RawHidTransport: Send + 'static {
    fn write(&mut self, report: &[u8]) -> Result<(), KeyPeekLiveError>;
    fn read(&mut self) -> Result<Vec<u8>, KeyPeekLiveError>;
}

pub struct QmkViaRawHidTransport {
    api: KeyboardApi,
}

impl QmkViaRawHidTransport {
    pub fn open(vid: u16, pid: u16) -> Result<Self, KeyPeekLiveError> {
        KeyboardApi::new(vid, pid, RAW_HID_USAGE_PAGE, None)
            .map(|api| Self { api })
            .map_err(|err| KeyPeekLiveError::Transport(format!("HID open failed: {err}")))
    }
}

impl RawHidTransport for QmkViaRawHidTransport {
    fn write(&mut self, report: &[u8]) -> Result<(), KeyPeekLiveError> {
        self.api
            .hid_send(report.to_vec())
            .map_err(|err| KeyPeekLiveError::Transport(format!("HID write failed: {err}")))
    }

    fn read(&mut self) -> Result<Vec<u8>, KeyPeekLiveError> {
        self.api
            .hid_read()
            .map_err(|err| KeyPeekLiveError::Transport(format!("HID read failed: {err}")))
    }
}

pub struct KeyPeekLiveSession<T> {
    transport: T,
    layer_count: usize,
    matrix_key_ids: BTreeMap<(u16, u16), String>,
    pressed_key_ids: BTreeSet<String>,
}

impl<T: RawHidTransport> KeyPeekLiveSession<T> {
    pub fn new(
        transport: T,
        layer_count: usize,
        matrix_key_ids: BTreeMap<(u16, u16), String>,
    ) -> Self {
        Self {
            transport,
            layer_count,
            matrix_key_ids,
            pressed_key_ids: BTreeSet::new(),
        }
    }

    pub fn start_subscription(&mut self) -> Result<(), KeyPeekLiveError> {
        self.transport.write(&keypeek_subscribe_message(true))
    }

    pub fn stop_subscription(&mut self) -> Result<(), KeyPeekLiveError> {
        self.transport.write(&keypeek_subscribe_message(false))
    }

    pub fn poll_next_event(&mut self) -> Result<Option<RuntimeEvent>, KeyPeekLiveError> {
        let report = self.transport.read()?;
        Ok(self.event_from_report(&report))
    }

    pub fn event_from_report(&mut self, report: &[u8]) -> Option<RuntimeEvent> {
        if let Some(event) = runtime_event_from_keypeek_layer_packet(report, self.layer_count) {
            return Some(event);
        }

        let packet = parse_keypeek_pressed_key_packet(report)?;
        let key_id = self.matrix_key_ids.get(&(packet.row, packet.col))?;
        if packet.pressed {
            self.pressed_key_ids.insert(key_id.clone());
        } else {
            self.pressed_key_ids.remove(key_id);
        }

        Some(RuntimeEvent::PressedKeysChanged {
            pressed_keys: self.pressed_key_ids.iter().cloned().collect(),
        })
    }
}

pub fn matrix_key_ids_from_layout(layout: &PhysicalLayout) -> BTreeMap<(u16, u16), String> {
    layout
        .keys
        .iter()
        .filter_map(|key| {
            key.matrix
                .as_ref()
                .map(|matrix| ((matrix.row, matrix.col), key.id.clone()))
        })
        .collect()
}

pub fn parse_usb_id(value: &str) -> Result<u16, KeyPeekLiveError> {
    let value = value.trim();
    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);

    if hex.is_empty() {
        return Err(KeyPeekLiveError::InvalidUsbId(value.to_string()));
    }

    u16::from_str_radix(hex, 16).map_err(|_| KeyPeekLiveError::InvalidUsbId(value.to_string()))
}

pub fn connected_health(vid: u16, pid: u16) -> BackendHealth {
    keypeek_backend_status(
        HealthState::Ok,
        format!(
            "Connected to KeyPeek-compatible HID {:04x}:{:04x}",
            vid, pid
        ),
    )
    .health
}

pub fn disconnected_health(message: impl Into<String>) -> BackendHealth {
    keypeek_backend_status(HealthState::Disconnected, message).health
}

pub fn protocol_error_health(message: impl Into<String>) -> BackendHealth {
    keypeek_backend_status(HealthState::ProtocolError, message).health
}

#[derive(Default)]
pub struct KeyPeekLiveRuntime {
    worker: Mutex<Option<KeyPeekLiveWorker>>,
}

struct KeyPeekLiveWorker {
    stop: Arc<AtomicBool>,
    _join: thread::JoinHandle<()>,
}

impl KeyPeekLiveRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start<T: RawHidTransport>(
        &self,
        app: tauri::AppHandle,
        session: KeyPeekLiveSession<T>,
    ) -> Result<(), String> {
        self.stop();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let join = thread::spawn(move || run_live_loop(app, session, worker_stop));
        let mut worker = self
            .worker
            .lock()
            .map_err(|_| "KeyPeek live runtime is unavailable".to_string())?;
        *worker = Some(KeyPeekLiveWorker { stop, _join: join });
        Ok(())
    }

    pub fn stop(&self) {
        let Ok(mut worker) = self.worker.lock() else {
            return;
        };
        if let Some(worker) = worker.take() {
            worker.stop.store(true, Ordering::Relaxed);
        }
    }
}

impl Drop for KeyPeekLiveRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_live_loop<T: RawHidTransport>(
    app: tauri::AppHandle,
    mut session: KeyPeekLiveSession<T>,
    stop: Arc<AtomicBool>,
) {
    let mut consecutive_read_errors = 0_u8;

    while !stop.load(Ordering::Relaxed) {
        match session.poll_next_event() {
            Ok(Some(event)) => {
                consecutive_read_errors = 0;
                let _ = app.emit(RUNTIME_EVENT_NAME, event);
            }
            Ok(None) => {
                consecutive_read_errors = 0;
            }
            Err(err) => {
                consecutive_read_errors = consecutive_read_errors.saturating_add(1);
                if consecutive_read_errors >= MAX_CONSECUTIVE_READ_ERRORS {
                    let _ = app.emit(
                        RUNTIME_EVENT_NAME,
                        RuntimeEvent::BackendHealthChanged {
                            health: disconnected_health(format!(
                                "KeyPeek HID read failed repeatedly: {err}"
                            )),
                        },
                    );
                    break;
                }
                thread::sleep(Duration::from_millis(200));
            }
        }
    }

    let _ = session.stop_subscription();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{KeyGeometry, MatrixPosition, PhysicalKey, SourceRef};
    use crate::keypeek_backend::{LAYER_STATE_PACKET_MARKER, PRESSED_KEY_PACKET_MARKER};
    use std::collections::VecDeque;

    #[derive(Default)]
    struct FakeRawHidTransport {
        writes: Vec<Vec<u8>>,
        reads: VecDeque<Result<Vec<u8>, KeyPeekLiveError>>,
    }

    impl RawHidTransport for FakeRawHidTransport {
        fn write(&mut self, report: &[u8]) -> Result<(), KeyPeekLiveError> {
            self.writes.push(report.to_vec());
            Ok(())
        }

        fn read(&mut self) -> Result<Vec<u8>, KeyPeekLiveError> {
            self.reads
                .pop_front()
                .unwrap_or_else(|| Err(KeyPeekLiveError::Transport("empty fake read".to_string())))
        }
    }

    fn fake_layout() -> PhysicalLayout {
        PhysicalLayout {
            fallback: false,
            keys: vec![
                physical_key("k-a", 0, 0),
                physical_key("k-b", 0, 1),
                physical_key("k-c", 1, 0),
            ],
        }
    }

    fn physical_key(id: &str, row: u16, col: u16) -> PhysicalKey {
        PhysicalKey {
            id: id.to_string(),
            matrix: Some(MatrixPosition { row, col }),
            geometry: KeyGeometry {
                x: f32::from(col),
                y: f32::from(row),
                width: 1.0,
                height: 1.0,
                rotation: 0.0,
            },
            provenance: SourceRef {
                source_id: "fixture".to_string(),
                field_path: ":keyboard/physical-layout".to_string(),
                raw: None,
            },
        }
    }

    #[test]
    fn parse_usb_id_accepts_plain_or_prefixed_hex() {
        assert_eq!(parse_usb_id("feed").unwrap(), 0xfeed);
        assert_eq!(parse_usb_id("0x046d").unwrap(), 0x046d);
        assert_eq!(parse_usb_id("0Xcafe").unwrap(), 0xcafe);
        assert!(parse_usb_id("").is_err());
        assert!(parse_usb_id("not-a-vid").is_err());
    }

    #[test]
    fn matrix_key_ids_use_profile_physical_key_matrix_positions() {
        let matrix = matrix_key_ids_from_layout(&fake_layout());

        assert_eq!(matrix.get(&(0, 0)), Some(&"k-a".to_string()));
        assert_eq!(matrix.get(&(1, 0)), Some(&"k-c".to_string()));
        assert_eq!(matrix.get(&(9, 9)), None);
    }

    #[test]
    fn live_session_writes_keypeek_subscription_markers() {
        let transport = FakeRawHidTransport::default();
        let mut session =
            KeyPeekLiveSession::new(transport, 3, matrix_key_ids_from_layout(&fake_layout()));

        session.start_subscription().unwrap();
        session.stop_subscription().unwrap();

        assert_eq!(
            session.transport.writes,
            vec![
                keypeek_subscribe_message(true),
                keypeek_subscribe_message(false)
            ]
        );
    }

    #[test]
    fn live_session_maps_layer_reports_to_runtime_events() {
        let transport = FakeRawHidTransport {
            reads: VecDeque::from([Ok(vec![LAYER_STATE_PACKET_MARKER, 1, 1, 0b0000_0100])]),
            ..FakeRawHidTransport::default()
        };
        let mut session =
            KeyPeekLiveSession::new(transport, 3, matrix_key_ids_from_layout(&fake_layout()));

        let event = session
            .poll_next_event()
            .expect("read succeeds")
            .expect("layer event");

        match event {
            RuntimeEvent::LayerStackChanged { layer_stack } => {
                assert_eq!(layer_stack[0].layer_id, "layer-2");
                assert_eq!(layer_stack[1].layer_id, "layer-0");
            }
            _ => panic!("expected layer stack event"),
        }
    }

    #[test]
    fn live_session_maps_pressed_key_reports_to_stable_key_ids() {
        let transport = FakeRawHidTransport {
            reads: VecDeque::from([
                Ok(vec![PRESSED_KEY_PACKET_MARKER, 0, 1, 1]),
                Ok(vec![PRESSED_KEY_PACKET_MARKER, 0, 1, 0]),
            ]),
            ..FakeRawHidTransport::default()
        };
        let mut session =
            KeyPeekLiveSession::new(transport, 3, matrix_key_ids_from_layout(&fake_layout()));

        let pressed = session
            .poll_next_event()
            .expect("read succeeds")
            .expect("pressed event");
        let released = session
            .poll_next_event()
            .expect("read succeeds")
            .expect("release event");

        assert_eq!(
            pressed,
            RuntimeEvent::PressedKeysChanged {
                pressed_keys: vec!["k-b".to_string()]
            }
        );
        assert_eq!(
            released,
            RuntimeEvent::PressedKeysChanged {
                pressed_keys: Vec::new()
            }
        );
    }

    #[test]
    fn live_session_ignores_pressed_key_reports_without_matrix_mapping() {
        let transport = FakeRawHidTransport {
            reads: VecDeque::from([Ok(vec![PRESSED_KEY_PACKET_MARKER, 9, 9, 1])]),
            ..FakeRawHidTransport::default()
        };
        let mut session =
            KeyPeekLiveSession::new(transport, 3, matrix_key_ids_from_layout(&fake_layout()));

        assert!(session.poll_next_event().expect("read succeeds").is_none());
    }

    #[test]
    #[ignore = "requires KEYPLANE_KEYPEEK_LIVE_VID and KEYPLANE_KEYPEEK_LIVE_PID to point at a KeyPeek-compatible Raw HID device"]
    fn local_keypeek_live_device_accepts_subscription_when_env_is_set() {
        let vid = parse_usb_id(
            &std::env::var("KEYPLANE_KEYPEEK_LIVE_VID").expect("KEYPLANE_KEYPEEK_LIVE_VID is set"),
        )
        .expect("VID is hex");
        let pid = parse_usb_id(
            &std::env::var("KEYPLANE_KEYPEEK_LIVE_PID").expect("KEYPLANE_KEYPEEK_LIVE_PID is set"),
        )
        .expect("PID is hex");

        let transport = QmkViaRawHidTransport::open(vid, pid).expect("KeyPeek Raw HID opens");
        let mut session =
            KeyPeekLiveSession::new(transport, 3, matrix_key_ids_from_layout(&fake_layout()));

        session
            .start_subscription()
            .expect("KeyPeek subscription starts");
        session
            .stop_subscription()
            .expect("KeyPeek subscription stops");
    }
}
