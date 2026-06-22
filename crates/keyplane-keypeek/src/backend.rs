//! The KeyPeek-derived Protocol Backend (ADR 0002, 0003, 0045).
//!
//! Reuses KeyPeek's VIA/Vial HID protocol code to discover a keyboard, read its
//! geometry and keymap, and stream live Layer Stack changes from KeyPeek's
//! firmware-module `0xff` layer packets. The device's raw VIA keycodes are kept
//! (ADR 0005) and labeled through the KeyPeek keycode tables via [`crate::bridge`].
//!
//! The pure adapters ([`build_model`], [`layer_stack_from_masks`]) are unit
//! tested with synthetic device data; the live transport itself is
//! hardware-gated.

use crate::bridge::{layout_key_to_semantic, via_code_to_semantic};
use crate::vendor::layout_key::LayoutKey;
use crate::vendor::protocols::{
    connect_protocol, ConnectionSpec, Key, KeyboardLayout, KeyboardProtocol,
    KEYPEEK_SUBSCRIBE_MARKER,
};
use keyplane_core::action::RawAction;
use keyplane_core::backend::{BackendDescriptor, BackendUpdate, ProtocolBackend};
use keyplane_core::geometry::{KeyGeometry, MatrixPosition};
use keyplane_core::health::{Capability, CapabilitySet, HealthState};
use keyplane_core::ids::{KeyId, LayerId};
use keyplane_core::model::keymap::{Layer, LayerEntry, LogicalKeymap};
use keyplane_core::model::physical::{PhysicalKey, PhysicalLayout};
use keyplane_core::model::{ActivationKind, ActiveLayer, KeyboardModel, LayerStack};
use keyplane_core::provenance::{Provenance, SourceKind};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use crate::vendor::protocols::ConnectionSpec as KeyPeekConnection;

/// A live KeyPeek/VIA/Vial backend. A background thread decodes HID layer
/// packets into [`BackendUpdate`]s; [`poll`](ProtocolBackend::poll) drains them.
pub struct KeyPeekBackend {
    descriptor: BackendDescriptor,
    queue: Arc<Mutex<VecDeque<BackendUpdate>>>,
    health: Arc<Mutex<HealthState>>,
}

fn key_id(row: usize, col: usize) -> KeyId {
    KeyId::new(format!("r{row}c{col}"))
}

fn layer_id(index: usize) -> LayerId {
    LayerId::new(format!("layer-{index}"))
}

fn provenance(raw: impl Into<String>) -> Provenance {
    Provenance::new("keypeek", SourceKind::KeyPeek).with_raw(raw)
}

fn zmk_provenance(raw: impl Into<String>) -> Provenance {
    Provenance::new("zmk", SourceKind::Zmk).with_raw(raw)
}

impl KeyPeekBackend {
    /// Connect to a device, read its model, and start streaming layer state.
    ///
    /// `layout_name` selects a named layout option; `None` uses the first.
    /// Hardware-gated: requires a connected, KeyPeek-supported device.
    pub fn connect(
        spec: ConnectionSpec,
        layout_name: Option<&str>,
    ) -> Result<(Self, KeyboardModel), String> {
        let is_zmk = matches!(spec, ConnectionSpec::Zmk { .. });
        let protocol = connect_protocol(&spec).map_err(|e| e.to_string())?;
        let definition = protocol.get_layout_definition().clone();
        let layout = match layout_name {
            Some(name) => definition.get_layout(name)?,
            None => definition
                .layouts
                .first()
                .cloned()
                .ok_or("device exposes no layouts")?,
        };
        let layers = protocol.get_layer_count().map_err(|e| e.to_string())?;

        // ZMK supplies a LayoutKey keymap (from ZMK Studio); VIA/Vial supply raw
        // numeric keycodes.
        let model = if is_zmk {
            let layout_keys = protocol.read_all_keys(layers, definition.rows, definition.cols);
            build_model_from_layout_keys(&layout, &layout_keys)
        } else {
            let raw = protocol.read_all_raw(layers, definition.rows, definition.cols);
            build_model(&layout, &raw)
        };

        let (id, kind, family) = if is_zmk {
            ("zmk", SourceKind::Zmk, "ZMK")
        } else {
            ("keypeek", SourceKind::KeyPeek, "KeyPeek")
        };
        let descriptor = BackendDescriptor {
            id: id.to_string(),
            name: format!("{family} ({})", layout.name),
            kind,
            capabilities: CapabilitySet::new([
                Capability::ImportGeometry,
                Capability::ImportKeymap,
                Capability::StreamLayerStack,
                Capability::StreamPressedKeys,
            ]),
        };

        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let health = Arc::new(Mutex::new(HealthState::Ok));

        spawn_keepalive(protocol.subscription_sender());
        spawn_reader(protocol, layers, queue.clone(), health.clone());

        Ok((
            Self {
                descriptor,
                queue,
                health,
            },
            model,
        ))
    }
}

impl ProtocolBackend for KeyPeekBackend {
    fn descriptor(&self) -> BackendDescriptor {
        self.descriptor.clone()
    }

    fn health(&self) -> HealthState {
        self.health.lock().expect("health poisoned").clone()
    }

    fn poll(&mut self) -> Vec<BackendUpdate> {
        self.queue.lock().expect("queue poisoned").drain(..).collect()
    }
}

/// Re-send the firmware subscribe keepalive every second, mirroring KeyPeek.
fn spawn_keepalive(sender: Option<Box<dyn crate::vendor::protocols::SubscriptionSender>>) {
    let Some(sender) = sender else { return };
    std::thread::spawn(move || loop {
        let _ = sender.set_active(true);
        std::thread::sleep(Duration::from_millis(1000));
    });
}

/// Background HID reader: decode KeyPeek `0xff` layer packets into Layer Stack
/// updates. Marks the backend Disconnected after repeated read errors.
fn spawn_reader(
    protocol: Box<dyn KeyboardProtocol>,
    layers: usize,
    queue: Arc<Mutex<VecDeque<BackendUpdate>>>,
    health: Arc<Mutex<HealthState>>,
) {
    std::thread::spawn(move || {
        const MAX_CONSECUTIVE_ERRORS: u32 = 5;
        let mut errors = 0u32;
        let mut last: Option<LayerStack> = None;

        loop {
            let response = match protocol.hid_read() {
                Ok(r) => {
                    errors = 0;
                    r
                }
                Err(_) => {
                    errors += 1;
                    if errors >= MAX_CONSECUTIVE_ERRORS {
                        let state = HealthState::Disconnected {
                            detail: "no HID response from device".into(),
                        };
                        *health.lock().unwrap() = state.clone();
                        queue.lock().unwrap().push_back(BackendUpdate::Health(state));
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                    continue;
                }
            };

            if let Some((default_mask, active_mask)) = decode_layer_packet(&response) {
                let stack = layer_stack_from_masks(default_mask, active_mask, layers);
                if last.as_ref() != Some(&stack) {
                    last = Some(stack.clone());
                    queue.lock().unwrap().push_back(BackendUpdate::LayerStack {
                        stack,
                        confidence: keyplane_core::model::StateConfidence::Authoritative,
                    });
                }
            }
        }
    });
}

/// Decode a KeyPeek layer packet (`0xff`), returning `(default_mask, layer_mask)`.
/// Returns `None` for any other packet (including the firmware echoing our
/// subscribe command back).
pub fn decode_layer_packet(response: &[u8]) -> Option<(u32, u32)> {
    const MAX_LAYER_STATE_BYTES: usize = 4;
    if response.first() != Some(&0xff) {
        return None;
    }
    let size = *response.get(1)? as usize;
    if size == 0 || size > MAX_LAYER_STATE_BYTES || 2 + 2 * size > response.len() {
        return None;
    }
    let mut default_bytes = [0u8; 4];
    default_bytes[..size].copy_from_slice(&response[2..2 + size]);
    let mut layer_bytes = [0u8; 4];
    layer_bytes[..size].copy_from_slice(&response[2 + size..2 + 2 * size]);
    Some((u32::from_le_bytes(default_bytes), u32::from_le_bytes(layer_bytes)))
}

/// Build an ordered Layer Stack from KeyPeek's default and active layer
/// bitmasks: default layers (ascending) form the base, then momentary layers
/// (ascending) on top.
pub fn layer_stack_from_masks(default_mask: u32, active_mask: u32, layers: usize) -> LayerStack {
    let mut active = Vec::new();
    for i in 0..layers.min(32) {
        if default_mask & (1 << i) != 0 {
            active.push(ActiveLayer::new(layer_id(i), ActivationKind::Default));
        }
    }
    if active.is_empty() {
        active.push(ActiveLayer::new(layer_id(0), ActivationKind::Default));
    }
    for i in 0..layers.min(32) {
        if active_mask & (1 << i) != 0 && default_mask & (1 << i) == 0 {
            active.push(ActiveLayer::new(layer_id(i), ActivationKind::Momentary));
        }
    }
    LayerStack::new(active)
}

/// Build a Keyplane [`KeyboardModel`] from a KeyPeek device definition, the
/// chosen physical layout, and raw VIA keycodes for every layer.
pub fn build_model(layout: &KeyboardLayout, raw_layers: &[Vec<Vec<u16>>]) -> KeyboardModel {
    let physical = PhysicalLayout::new(layout.keys.iter().map(physical_key).collect());

    let resolver = |n: u16| layer_id(n as usize);
    let mut layers = Vec::new();
    for (index, layer_codes) in raw_layers.iter().enumerate() {
        let mut layer = Layer::new(layer_id(index), index as u16);
        for key in &layout.keys {
            let code = layer_codes
                .get(key.row)
                .and_then(|r| r.get(key.col))
                .copied()
                .unwrap_or(0);
            let raw = RawAction::ViaCode(code);
            let semantic = via_code_to_semantic(code, &resolver);
            layer.entries.insert(
                key_id(key.row, key.col),
                LayerEntry::new(raw, semantic).with_provenance(provenance(format!("0x{code:04X}"))),
            );
        }
        layers.push(layer);
    }

    let keymap = LogicalKeymap::new(layers).with_default(layer_id(0));
    let mut model = KeyboardModel::new(physical, keymap);
    model.name = Some(layout.name.clone());
    model
}

/// Build a Keyplane model from a ZMK keymap, whose entries are already
/// `LayoutKey`s (resolved by ZMK Studio) rather than raw VIA codes.
/// `layer_keys` is indexed `[layer][row][col]`.
pub fn build_model_from_layout_keys(
    layout: &KeyboardLayout,
    layer_keys: &[Vec<Vec<Option<LayoutKey>>>],
) -> KeyboardModel {
    let physical = PhysicalLayout::new(layout.keys.iter().map(physical_key).collect());

    let resolver = |n: u16| layer_id(n as usize);
    let mut layers = Vec::new();
    for (index, layer) in layer_keys.iter().enumerate() {
        let mut out = Layer::new(layer_id(index), index as u16);
        for key in &layout.keys {
            let Some(cell) = layer.get(key.row).and_then(|r| r.get(key.col)) else {
                continue;
            };
            let Some(lk) = cell else {
                continue;
            };
            let token = if lk.tap.full.is_empty() {
                "&trans".to_string()
            } else {
                lk.tap.full.clone()
            };
            let raw = RawAction::Zmk(token.clone());
            let semantic = layout_key_to_semantic(lk, &resolver);
            out.entries.insert(
                key_id(key.row, key.col),
                LayerEntry::new(raw, semantic).with_provenance(zmk_provenance(token)),
            );
        }
        layers.push(out);
    }

    let keymap = LogicalKeymap::new(layers).with_default(layer_id(0));
    let mut model = KeyboardModel::new(physical, keymap);
    model.name = Some(layout.name.clone());
    model
}

fn physical_key(key: &Key) -> PhysicalKey {
    let geometry = KeyGeometry {
        x: key.x as f64,
        y: key.y as f64,
        w: key.w as f64,
        h: key.h as f64,
        rotation: key.r as f64,
        rotation_origin: if key.r != 0.0 {
            Some((key.x as f64 + key.w as f64 / 2.0, key.y as f64 + key.h as f64 / 2.0))
        } else {
            None
        },
    };
    PhysicalKey::new(key_id(key.row, key.col), geometry)
        .with_matrix(MatrixPosition::new(key.row as u16, key.col as u16))
        .with_provenance(provenance("keypeek-geometry"))
}

// Re-export so the marker stays referenced and documented as the firmware ABI.
#[allow(dead_code)]
const _SUBSCRIBE_MARKER: u8 = KEYPEEK_SUBSCRIBE_MARKER;
