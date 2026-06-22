// Keyplane — Sentinel-key Protocol Backend.
// Copyright (C) 2026 Keyplane contributors. GPL-3.0-only.

//! Sentinel keys are a separate, lower-confidence Protocol Backend (ADR 0016):
//! when no authoritative source reports layer state, the app maps configured
//! Host Input Events ("Sentinel Keys") to layer changes. Because startup state,
//! dropped events, and out-of-band changes can make it wrong, this backend
//! always reports [`StateConfidence::Inferred`].
//!
//! The inference is pure ([`SentinelTracker`]) and unit tested. Capturing real
//! OS key events is the remaining transport: feed [`HostEvent`]s in via
//! [`SentinelBackend::feed`] (from an OS hook, or the frontend) and the backend
//! emits Layer Stack updates.

use keyplane_core::backend::{BackendDescriptor, BackendUpdate, ProtocolBackend};
use keyplane_core::health::{Capability, CapabilitySet, HealthState};
use keyplane_core::ids::LayerId;
use keyplane_core::model::{ActivationKind, ActiveLayer, LayerStack, StateConfidence};
use keyplane_core::provenance::SourceKind;
use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

/// What a Sentinel Key does to the Layer Stack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SentinelAction {
    /// Activate `layer` while the key is held.
    Momentary(LayerId),
    /// Flip `layer` on/off on each press.
    Toggle(LayerId),
}

/// A configured Sentinel Key: a Host Input Event name and its action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentinelKey {
    pub host_key: String,
    pub action: SentinelAction,
}

/// A Host Input Event the tracker consumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostEvent {
    KeyDown(String),
    KeyUp(String),
}

/// Flip a layer in the toggled set (remove if present, else insert).
fn toggle(set: &mut BTreeSet<LayerId>, layer: LayerId) {
    if !set.remove(&layer) {
        set.insert(layer);
    }
}

/// Pure layer-state inference from sentinel-key events.
pub struct SentinelTracker {
    keys: Vec<SentinelKey>,
    base: LayerId,
    held: BTreeSet<LayerId>,
    toggled: BTreeSet<LayerId>,
}

impl SentinelTracker {
    pub fn new(keys: Vec<SentinelKey>, base: LayerId) -> Self {
        Self {
            keys,
            base,
            held: BTreeSet::new(),
            toggled: BTreeSet::new(),
        }
    }

    fn action_for(&self, host_key: &str) -> Option<&SentinelAction> {
        self.keys
            .iter()
            .find(|k| k.host_key == host_key)
            .map(|k| &k.action)
    }

    /// Apply a Host Input Event and return the resulting Layer Stack.
    pub fn on_event(&mut self, event: &HostEvent) -> LayerStack {
        match event {
            HostEvent::KeyDown(key) => match self.action_for(key).cloned() {
                Some(SentinelAction::Momentary(layer)) => {
                    self.held.insert(layer);
                }
                Some(SentinelAction::Toggle(layer)) => toggle(&mut self.toggled, layer),
                None => {}
            },
            HostEvent::KeyUp(key) => {
                if let Some(SentinelAction::Momentary(layer)) = self.action_for(key).cloned() {
                    self.held.remove(&layer);
                }
            }
        }
        self.stack()
    }

    /// The current inferred Layer Stack: base, then toggled layers, then held.
    pub fn stack(&self) -> LayerStack {
        let mut active = vec![ActiveLayer::new(self.base.clone(), ActivationKind::Default)];
        for layer in &self.toggled {
            if *layer != self.base {
                active.push(ActiveLayer::new(layer.clone(), ActivationKind::Toggle));
            }
        }
        for layer in &self.held {
            if *layer != self.base {
                active.push(ActiveLayer::new(layer.clone(), ActivationKind::Momentary));
            }
        }
        LayerStack::new(active)
    }
}

/// A sentinel-key backend. Feed it [`HostEvent`]s; it emits inferred Layer Stack
/// updates at [`StateConfidence::Inferred`].
pub struct SentinelBackend {
    descriptor: BackendDescriptor,
    tracker: Arc<Mutex<SentinelTracker>>,
    queue: Arc<Mutex<VecDeque<BackendUpdate>>>,
}

impl SentinelBackend {
    pub fn new(keys: Vec<SentinelKey>, base: LayerId) -> Self {
        Self {
            descriptor: BackendDescriptor {
                id: "sentinel".to_string(),
                name: "Sentinel keys".to_string(),
                kind: SourceKind::Sentinel,
                capabilities: CapabilitySet::new([
                    Capability::StreamLayerStack,
                    Capability::PreviewOnly,
                ]),
            },
            tracker: Arc::new(Mutex::new(SentinelTracker::new(keys, base))),
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Feed one Host Input Event, queueing the resulting Layer Stack update.
    pub fn feed(&self, event: HostEvent) {
        let stack = self.tracker.lock().expect("tracker").on_event(&event);
        self.queue.lock().expect("queue").push_back(BackendUpdate::LayerStack {
            stack,
            confidence: StateConfidence::Inferred,
        });
    }
}

impl ProtocolBackend for SentinelBackend {
    fn descriptor(&self) -> BackendDescriptor {
        self.descriptor.clone()
    }

    fn health(&self) -> HealthState {
        HealthState::Ok
    }

    fn poll(&mut self) -> Vec<BackendUpdate> {
        self.queue.lock().expect("queue").drain(..).collect()
    }
}
