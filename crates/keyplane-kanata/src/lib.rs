// Keyplane — Kanata Protocol Backend.
// Copyright (C) 2026 Keyplane contributors. GPL-3.0-only.

//! Kanata is a Protocol Backend, not an importer (ADR 0009): it is authoritative
//! for runtime layer state but supplies no geometry or keymap, so the MVP pairs
//! it with an OverKeys-style companion profile that does (ADR 0010).
//!
//! Kanata's TCP server (`kanata --port <N>`) emits newline-delimited JSON
//! events; the one Keyplane consumes is `LayerChange`. This crate parses those
//! messages and maps the named layer onto the companion profile's layers,
//! emitting a Layer Stack update. The message parser and the name→layer mapping
//! are pure and unit-tested; the live TCP transport is covered with a local
//! server in the tests.

use keyplane_core::backend::{BackendDescriptor, BackendUpdate, ProtocolBackend};
use keyplane_core::health::{Capability, CapabilitySet, HealthState};
use keyplane_core::ids::LayerId;
use keyplane_core::model::{ActivationKind, ActiveLayer, LayerStack, StateConfidence};
use keyplane_core::provenance::SourceKind;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A parsed Kanata TCP event (only the subset Keyplane needs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KanataEvent {
    /// The active layer changed to `layer` (Kanata's layer name).
    LayerChange { layer: String },
}

/// Parse one line of Kanata's newline-delimited JSON. Returns `None` for
/// messages Keyplane does not consume (or malformed lines), so an unexpected
/// message never breaks the stream.
pub fn parse_message(line: &str) -> Option<KanataEvent> {
    let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let layer = value.get("LayerChange")?.get("new")?.as_str()?.to_string();
    Some(KanataEvent::LayerChange { layer })
}

/// Maps Kanata layer names to the companion profile's [`LayerId`]s.
#[derive(Clone, Debug)]
pub struct LayerMap {
    by_name: BTreeMap<String, LayerId>,
    base: LayerId,
}

impl LayerMap {
    /// Build from `(name, id)` pairs (typically the companion profile's layers,
    /// keyed by layer name) plus the base layer id.
    pub fn new(pairs: impl IntoIterator<Item = (String, LayerId)>, base: LayerId) -> Self {
        Self {
            by_name: pairs
                .into_iter()
                .map(|(n, id)| (n.to_lowercase(), id))
                .collect(),
            base,
        }
    }

    /// Resolve a Kanata layer name to a [`LayerId`], falling back to a synthetic
    /// id so an unknown layer still renders something.
    pub fn resolve(&self, name: &str) -> LayerId {
        self.by_name
            .get(&name.to_lowercase())
            .cloned()
            .unwrap_or_else(|| LayerId::new(name))
    }

    /// The Layer Stack for an active Kanata layer: the base layer plus the
    /// active layer on top (when different). Kanata is an authoritative
    /// remapper, so activation is [`ActivationKind::Remapper`].
    pub fn stack_for(&self, name: &str) -> LayerStack {
        let active = self.resolve(name);
        if active == self.base {
            LayerStack::base(self.base.clone())
        } else {
            LayerStack::new(vec![
                ActiveLayer::new(self.base.clone(), ActivationKind::Default),
                ActiveLayer::new(active, ActivationKind::Remapper),
            ])
        }
    }
}

/// A live Kanata backend. A reader thread maps Kanata `LayerChange` events to
/// Layer Stack updates; [`poll`](ProtocolBackend::poll) drains them.
pub struct KanataBackend {
    descriptor: BackendDescriptor,
    queue: Arc<Mutex<std::collections::VecDeque<BackendUpdate>>>,
    health: Arc<Mutex<HealthState>>,
}

impl KanataBackend {
    /// Connect to Kanata's TCP server and start streaming layer changes.
    /// `layers` maps Kanata layer names to the companion profile's layers.
    pub fn connect(addr: impl ToSocketAddrs, layers: LayerMap) -> Result<Self, String> {
        let stream = TcpStream::connect(addr).map_err(|e| format!("Kanata connect failed: {e}"))?;
        let queue = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        let health = Arc::new(Mutex::new(HealthState::Ok));
        spawn_reader(stream, layers, queue.clone(), health.clone());
        Ok(Self {
            descriptor: BackendDescriptor {
                id: "kanata".to_string(),
                name: "Kanata".to_string(),
                kind: SourceKind::Kanata,
                capabilities: CapabilitySet::new([Capability::StreamLayerStack]),
            },
            queue,
            health,
        })
    }
}

impl ProtocolBackend for KanataBackend {
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

fn spawn_reader(
    stream: TcpStream,
    layers: LayerMap,
    queue: Arc<Mutex<std::collections::VecDeque<BackendUpdate>>>,
    health: Arc<Mutex<HealthState>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let Ok(line) = line else {
                let state = HealthState::Disconnected {
                    detail: "Kanata TCP stream closed".into(),
                };
                *health.lock().unwrap() = state.clone();
                queue.lock().unwrap().push_back(BackendUpdate::Health(state));
                break;
            };
            if let Some(KanataEvent::LayerChange { layer }) = parse_message(&line) {
                let stack = layers.stack_for(&layer);
                queue.lock().unwrap().push_back(BackendUpdate::LayerStack {
                    stack,
                    confidence: StateConfidence::Authoritative,
                });
            }
        }
    });
}
