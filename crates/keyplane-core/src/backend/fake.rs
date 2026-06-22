//! The Fake Backend (ADR 0003 first slice).
//!
//! A deterministic Protocol Backend for development, tests, and demos. It owns
//! no hardware: it ships a small demo [`KeyboardModel`] with transparent keys
//! (to exercise inheritance) and a scripted sequence of Layer Stack changes.
//! The first vertical slice renders these in the Overlay Window without real
//! hardware.

use super::{BackendDescriptor, BackendUpdate, ProtocolBackend};
use crate::action::RawAction;
use crate::geometry::KeyGeometry;
use crate::health::{Capability, CapabilitySet, HealthState};
use crate::ids::{KeyId, LayerId};
use crate::model::keymap::{Layer, LayerEntry, LogicalKeymap};
use crate::model::physical::{PhysicalKey, PhysicalLayout};
use crate::model::{ActivationKind, ActiveLayer, KeyboardModel, LayerStack, StateConfidence};
use crate::provenance::{Provenance, SourceKind};
use crate::resolve::semantic;
use std::collections::VecDeque;

/// A scripted, deterministic backend.
pub struct FakeBackend {
    descriptor: BackendDescriptor,
    health: HealthState,
    pending: VecDeque<BackendUpdate>,
    /// The script to (optionally) replay; the driver can reload it to loop.
    script: Vec<BackendUpdate>,
}

impl FakeBackend {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            descriptor: BackendDescriptor {
                id: id.into(),
                name: name.into(),
                kind: SourceKind::Fake,
                capabilities: CapabilitySet::new([
                    Capability::ImportGeometry,
                    Capability::ImportKeymap,
                    Capability::StreamLayerStack,
                    Capability::StreamPressedKeys,
                ]),
            },
            health: HealthState::Ok,
            pending: VecDeque::new(),
            script: Vec::new(),
        }
    }

    /// Build the demo backend plus its keyboard model and a layer-cycling
    /// script. The first vertical slice wires this straight into the overlay.
    pub fn demo() -> (Self, KeyboardModel) {
        let model = demo_model();
        let mut backend = FakeBackend::new("fake", "Fake Backend");
        backend.load_script(demo_script(&model));
        (backend, model)
    }

    /// Replace the pending queue with `script` so it replays from the top.
    pub fn load_script(&mut self, script: Vec<BackendUpdate>) {
        self.script = script.clone();
        self.pending = script.into_iter().collect();
    }

    /// Re-queue the loaded script so a driver can loop the demo.
    pub fn replay(&mut self) {
        self.pending = self.script.clone().into_iter().collect();
    }

    pub fn enqueue(&mut self, update: BackendUpdate) {
        self.pending.push_back(update);
    }

    pub fn set_health(&mut self, health: HealthState) {
        self.health = health;
    }
}

impl ProtocolBackend for FakeBackend {
    fn descriptor(&self) -> BackendDescriptor {
        self.descriptor.clone()
    }

    fn health(&self) -> HealthState {
        self.health.clone()
    }

    fn poll(&mut self) -> Vec<BackendUpdate> {
        // Pop exactly one queued update per poll so a timer-driven driver
        // animates the demo one step at a time.
        self.pending.drain(..1.min(self.pending.len())).collect()
    }
}

/// The id used for the demo backend's layers, shared by the model and script.
fn layer_id(index: u16) -> LayerId {
    LayerId::new(format!("layer-{index}"))
}

fn prov() -> Provenance {
    Provenance::new("fake", SourceKind::Fake)
}

/// Build a 4x4 demo keyboard with three layers and transparent keys that
/// inherit from the base layer, so the overlay exercises real resolution.
pub fn demo_model() -> KeyboardModel {
    // Physical layout: 4 columns x 4 rows of 1u keys.
    let mut keys = Vec::new();
    for row in 0..4u16 {
        for col in 0..4u16 {
            let id = KeyId::new(format!("k{}", row * 4 + col));
            keys.push(
                PhysicalKey::new(id, KeyGeometry::unit(col as f64, row as f64))
                    .with_matrix(crate::geometry::MatrixPosition::new(row, col))
                    .with_provenance(prov()),
            );
        }
    }
    let physical = PhysicalLayout::new(keys);

    let base = build_layer(
        0,
        "Base",
        &[
            "KC_A", "KC_B", "KC_C", "KC_D", //
            "KC_E", "KC_F", "KC_G", "KC_H", //
            "KC_I", "KC_J", "KC_K", "KC_L", //
            "MO(1)", "MO(2)", "TG(2)", "KC_SPC",
        ],
    );
    let nav = build_layer(
        1,
        "Numbers",
        &[
            "KC_1", "KC_2", "KC_3", "KC_4", //
            "KC_5", "KC_6", "KC_7", "KC_8", //
            "KC_TRNS", "KC_TRNS", "KC_TRNS", "KC_TRNS", // inherit I J K L
            "KC_TRNS", "KC_TRNS", "KC_TRNS", "KC_TRNS",
        ],
    );
    let arrows = build_layer(
        2,
        "Arrows",
        &[
            "KC_LEFT", "KC_DOWN", "KC_UP", "KC_RGHT", //
            "KC_TRNS", "KC_TRNS", "KC_TRNS", "KC_TRNS", //
            "KC_TRNS", "KC_TRNS", "KC_TRNS", "KC_TRNS", //
            "KC_TRNS", "KC_TRNS", "KC_TRNS", "KC_TRNS",
        ],
    );

    let keymap = LogicalKeymap::new(vec![base, nav, arrows]).with_default(layer_id(0));
    KeyboardModel::new(physical, keymap).with_name("Keyplane Demo 4x4")
}

/// Build a layer from 16 QMK tokens, deriving semantics with layer-index
/// awareness so `MO(1)` resolves to the right [`LayerId`].
fn build_layer(index: u16, name: &str, tokens: &[&str; 16]) -> Layer {
    let resolver = |n: u16| layer_id(n);
    let mut layer = Layer::new(layer_id(index), index).with_name(name);
    for (i, token) in tokens.iter().enumerate() {
        let key = KeyId::new(format!("k{i}"));
        let raw = RawAction::Qmk((*token).to_string());
        let semantic = semantic::derive(&raw, &resolver);
        layer.entries.insert(
            key,
            LayerEntry::new(raw, semantic).with_provenance(prov().with_raw(*token)),
        );
    }
    layer
}

/// A short demo: base → momentary nav → base → toggle arrows → base.
fn demo_script(_model: &KeyboardModel) -> Vec<BackendUpdate> {
    let base = LayerStack::base(layer_id(0));
    let nav = LayerStack::new(vec![
        ActiveLayer::new(layer_id(0), ActivationKind::Default),
        ActiveLayer::new(layer_id(1), ActivationKind::Momentary),
    ]);
    let arrows = LayerStack::new(vec![
        ActiveLayer::new(layer_id(0), ActivationKind::Default),
        ActiveLayer::new(layer_id(2), ActivationKind::Toggle),
    ]);
    let c = StateConfidence::Authoritative;
    vec![
        BackendUpdate::LayerStack {
            stack: base.clone(),
            confidence: c,
        },
        BackendUpdate::LayerStack {
            stack: nav,
            confidence: c,
        },
        BackendUpdate::LayerStack {
            stack: base.clone(),
            confidence: c,
        },
        BackendUpdate::LayerStack {
            stack: arrows,
            confidence: c,
        },
        BackendUpdate::LayerStack {
            stack: base,
            confidence: c,
        },
    ]
}
