//! End-to-end validation of the KeyPeek backend pipeline with a fake transport.
//!
//! `KeyPeekBackend::from_protocol` is the body of `connect` with the HID/serial
//! transport injected. By implementing a fake `KeyboardProtocol` that returns
//! canned geometry/keycodes and a scripted sequence of real KeyPeek `0xff` layer
//! packets, we validate the full live path — model build, the reader thread
//! decoding actual packet bytes, and streamed Layer Stack updates into the
//! `ProtocolBackend` queue — deterministically, with no device. The only thing
//! left unexercised is the literal USB/serial syscall inside `qmk-via-api`.

use keyplane_core::action::SemanticAction;
use keyplane_core::backend::{BackendUpdate, ProtocolBackend};
use keyplane_core::ids::{KeyId, LayerId};
use keyplane_keypeek::backend::KeyPeekBackend;
use keyplane_keypeek::vendor::layout_key::{Label, LayoutKey};
use keyplane_keypeek::vendor::protocols::{
    Key, KeyboardDefinition, KeyboardLayout, KeyboardProtocol,
};
use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A fake `KeyboardProtocol`: canned definition + raw keycodes, and a scripted
/// queue of HID packets (`hid_read` errors once the script is drained, which
/// stops the reader thread).
struct FakeProtocol {
    definition: KeyboardDefinition,
    raw: Vec<Vec<Vec<u16>>>,
    layout_keys: Vec<Vec<Vec<Option<LayoutKey>>>>,
    packets: Arc<Mutex<VecDeque<Vec<u8>>>>,
}

impl KeyboardProtocol for FakeProtocol {
    fn get_layout_definition(&self) -> &KeyboardDefinition {
        &self.definition
    }

    fn get_layer_count(&self) -> Result<usize, Box<dyn Error>> {
        Ok(self.raw.len())
    }

    fn read_all_keys(
        &self,
        _layers: usize,
        _rows: usize,
        _cols: usize,
    ) -> Vec<Vec<Vec<Option<LayoutKey>>>> {
        self.layout_keys.clone() // used on the ZMK path
    }

    fn read_all_raw(&self, _layers: usize, _rows: usize, _cols: usize) -> Vec<Vec<Vec<u16>>> {
        self.raw.clone()
    }

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        match self.packets.lock().unwrap().pop_front() {
            Some(packet) => Ok(packet),
            None => Err("no more packets".into()),
        }
    }
}

fn demo_protocol(packets: Vec<Vec<u8>>) -> FakeProtocol {
    let definition = KeyboardDefinition {
        vid: 0x1234,
        pid: 0x5678,
        rows: 1,
        cols: 2,
        layouts: vec![KeyboardLayout {
            name: "Default".into(),
            keys: vec![
                Key { row: 0, col: 0, x: 0.0, y: 0.0, w: 1.0, h: 1.0, r: 0.0 },
                Key { row: 0, col: 1, x: 1.0, y: 0.0, w: 1.0, h: 1.0, r: 0.0 },
            ],
        }],
    };
    // Two layers; base = [A, MO(1)], layer 1 = [1, transparent].
    let raw = vec![
        vec![vec![0x0004, 0x5221]],
        vec![vec![0x001E, 0x0001]],
    ];
    FakeProtocol {
        definition,
        raw,
        layout_keys: Vec::new(),
        packets: Arc::new(Mutex::new(packets.into())),
    }
}

fn zmk_protocol(packets: Vec<Vec<u8>>) -> FakeProtocol {
    let mut p = demo_protocol(packets);
    // ZMK supplies a LayoutKey keymap (as ZMK Studio resolves it). One layer,
    // two keys: Q and W.
    let key = |label: &str| {
        Some(LayoutKey {
            tap: Label::new(label),
            ..Default::default()
        })
    };
    p.layout_keys = vec![vec![vec![key("Q"), key("W")]]];
    p
}

#[test]
fn backend_builds_model_and_streams_live_layer_changes() {
    // A real KeyPeek `0xff` layer packet: size=1, default=0x01, active=0x02
    // (momentary layer 1 active).
    let layer1_packet = vec![0xff, 0x01, 0x01, 0x02];
    let protocol = demo_protocol(vec![layer1_packet]);

    let (mut backend, model) =
        KeyPeekBackend::from_protocol(Box::new(protocol), None, false).expect("build");

    // The model came from the device geometry + raw keycodes, labeled via
    // KeyPeek's tables.
    assert_eq!(model.physical_layout.keys.len(), 2);
    let a = model
        .keymap
        .layer(&LayerId::new("layer-0"))
        .unwrap()
        .entry(&KeyId::new("r0c0"))
        .unwrap();
    assert_eq!(a.semantic, SemanticAction::Key { label: "A".into() });

    // The reader thread decodes the scripted packet into a Layer Stack update.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut got = None;
    while Instant::now() < deadline {
        if let Some(update) = backend.poll().into_iter().next() {
            got = Some(update);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    match got {
        Some(BackendUpdate::LayerStack { stack, .. }) => {
            assert_eq!(stack.top().unwrap().layer, LayerId::new("layer-1"));
        }
        other => panic!("expected a streamed layer-stack update, got {other:?}"),
    }
}

#[test]
fn zmk_path_builds_model_from_layout_keys_and_streams() {
    let layer1_packet = vec![0xff, 0x01, 0x01, 0x02];
    let (mut backend, model) =
        KeyPeekBackend::from_protocol(Box::new(zmk_protocol(vec![layer1_packet])), None, true)
            .expect("build");

    assert_eq!(backend.descriptor().kind, keyplane_core::provenance::SourceKind::Zmk);
    // The ZMK keymap (LayoutKeys) is classified through the bridge.
    let q = model
        .keymap
        .layer(&LayerId::new("layer-0"))
        .unwrap()
        .entry(&KeyId::new("r0c0"))
        .unwrap();
    assert_eq!(q.semantic, SemanticAction::Key { label: "Q".into() });

    // And it streams live layer state over the same 0xff HID path.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut got = None;
    while Instant::now() < deadline {
        if let Some(update) = backend.poll().into_iter().next() {
            got = Some(update);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(matches!(got, Some(BackendUpdate::LayerStack { .. })));
}

#[test]
fn backend_reports_disconnect_after_repeated_read_errors() {
    // No packets: hid_read errors immediately, so after the consecutive-error
    // threshold the backend reports Disconnected health (ADR 0023).
    let (mut backend, _model) =
        KeyPeekBackend::from_protocol(Box::new(demo_protocol(vec![])), None, false).expect("build");

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut disconnected = false;
    while Instant::now() < deadline {
        if backend
            .poll()
            .iter()
            .any(|u| matches!(u, BackendUpdate::Health(h) if h.tag() == "disconnected"))
        {
            disconnected = true;
            break;
        }
        if backend.health().tag() == "disconnected" {
            disconnected = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(disconnected, "backend surfaced a disconnect");
}
