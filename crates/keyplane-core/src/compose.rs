//! State composition (ADR 0036) — the app's primary test seam.
//!
//! The [`Composer`] turns a [`KeyboardModel`] plus backend updates into a
//! [`KeyboardSnapshot`] and a stream of [`RuntimeEvent`]s. Per the PRD this is
//! the highest-value seam: it validates the contract the whole app depends on,
//! independent of HID, Tauri, or the frontend.

use crate::backend::{BackendDescriptor, BackendUpdate};
use crate::event::RuntimeEvent;
use crate::health::{BackendHealth, HealthState};
use crate::model::{KeyboardModel, LayerStack, RuntimeState, StateConfidence};
use crate::resolve::{resolve_layout, ResolvedKey};
use crate::snapshot::{KeyboardSnapshot, LayerInfo, SNAPSHOT_SCHEMA};

/// Owns the live keyboard view: the static model, current Runtime State, and
/// each backend's health. Folds backend updates into runtime events.
pub struct Composer {
    model: KeyboardModel,
    runtime: RuntimeState,
    backends: Vec<BackendHealth>,
}

impl Composer {
    /// Create a composer for `model`, seeding the Layer Stack to the base layer
    /// with the given starting confidence.
    pub fn new(model: KeyboardModel, confidence: StateConfidence) -> Self {
        let base = model
            .base_layer()
            .map(LayerStack::base)
            .unwrap_or_default();
        let runtime = RuntimeState::new(base, confidence);
        Self {
            model,
            runtime,
            backends: Vec::new(),
        }
    }

    pub fn model(&self) -> &KeyboardModel {
        &self.model
    }

    pub fn runtime(&self) -> &RuntimeState {
        &self.runtime
    }

    /// Register (or replace) a backend's descriptor and current health.
    pub fn register_backend(&mut self, descriptor: &BackendDescriptor, health: HealthState) {
        let entry = BackendHealth::new(
            descriptor.id.clone(),
            descriptor.name.clone(),
            descriptor.capabilities.clone(),
            health,
        );
        match self.backends.iter_mut().find(|b| b.backend_id == entry.backend_id) {
            Some(existing) => *existing = entry,
            None => self.backends.push(entry),
        }
    }

    /// Resolve the current model + runtime into a renderable snapshot.
    pub fn snapshot(&self) -> KeyboardSnapshot {
        KeyboardSnapshot {
            schema: SNAPSHOT_SCHEMA,
            keyboard_name: self.model.name.clone(),
            extent: self.model.physical_layout.extent(),
            style: self.model.style.clone(),
            layers: self
                .model
                .keymap
                .layers
                .iter()
                .map(|l| LayerInfo {
                    id: l.id.clone(),
                    index: l.index,
                    name: l.name.clone(),
                })
                .collect(),
            layer_stack: self.runtime.layer_stack.clone(),
            confidence: self.runtime.confidence,
            keys: self.resolve(),
            backends: self.backends.clone(),
        }
    }

    fn resolve(&self) -> Vec<ResolvedKey> {
        resolve_layout(&self.model, &self.runtime.layer_stack)
    }

    /// Apply one backend update, mutating Runtime State and emitting the matching
    /// [`RuntimeEvent`]. Returns `None` when the update is a no-op (e.g. the
    /// layer stack did not actually change).
    pub fn apply(&mut self, backend_id: &str, update: BackendUpdate) -> Option<RuntimeEvent> {
        match update {
            BackendUpdate::LayerStack { stack, confidence } => {
                self.apply_layer_stack(stack, confidence)
            }
            BackendUpdate::PressedKeys(pressed) => {
                if self.runtime.pressed == pressed {
                    return None;
                }
                self.runtime.pressed = pressed.clone();
                Some(RuntimeEvent::PressedKeys { pressed })
            }
            BackendUpdate::Health(health) => self.set_backend_health(backend_id, health),
        }
    }

    fn apply_layer_stack(
        &mut self,
        stack: LayerStack,
        confidence: StateConfidence,
    ) -> Option<RuntimeEvent> {
        if self.runtime.layer_stack == stack && self.runtime.confidence == confidence {
            return None;
        }
        self.runtime.layer_stack = stack.clone();
        self.runtime.confidence = confidence;
        let keys = self.resolve();
        Some(RuntimeEvent::LayerStack {
            layer_stack: stack,
            confidence,
            keys,
        })
    }

    /// Update one backend's health and emit a [`RuntimeEvent::BackendHealth`].
    pub fn set_backend_health(
        &mut self,
        backend_id: &str,
        health: HealthState,
    ) -> Option<RuntimeEvent> {
        let entry = self.backends.iter_mut().find(|b| b.backend_id == backend_id)?;
        if entry.health == health {
            return None;
        }
        entry.health = health;
        Some(RuntimeEvent::BackendHealth {
            health: entry.clone(),
        })
    }

    /// Backend health snapshot, for the App Window status area.
    pub fn backends(&self) -> &[BackendHealth] {
        &self.backends
    }
}
