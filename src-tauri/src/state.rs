//! Shared application state behind a mutex.
//!
//! The [`Composer`] (model + runtime + health), the active backend, and the
//! active [`Profile`] live here. Commands and the driver loop both lock this
//! state; all keyboard logic stays in `keyplane-core` and `keyplane-keypeek`.

use keyplane_core::backend::{BackendDescriptor, BackendUpdate, FakeBackend, ProtocolBackend};
use keyplane_core::compose::Composer;
use keyplane_core::health::HealthState;
use keyplane_core::model::{KeyboardModel, StateConfidence};
use keyplane_core::profile::Profile;
use keyplane_kanata::KanataBackend;
use keyplane_keypeek::KeyPeekBackend;
use keyplane_sentinel::{HostEvent, SentinelBackend};
use std::sync::Mutex;

/// Tauri event names the frontend subscribes to.
pub const EVENT_RUNTIME: &str = "keyplane://runtime-event";
pub const EVENT_SNAPSHOT: &str = "keyplane://snapshot";

/// The active Protocol Backend: the scripted Fake Backend, or a live
/// KeyPeek-derived device backend. Both implement
/// [`ProtocolBackend`](keyplane_core::backend::ProtocolBackend); the enum keeps
/// the Fake Backend's `replay` (for looping the demo) without a trait object.
pub enum Backend {
    Fake(FakeBackend),
    KeyPeek(KeyPeekBackend),
    Kanata(KanataBackend),
    Sentinel(SentinelBackend),
}

impl Backend {
    pub fn descriptor(&self) -> BackendDescriptor {
        match self {
            Backend::Fake(b) => b.descriptor(),
            Backend::KeyPeek(b) => b.descriptor(),
            Backend::Kanata(b) => b.descriptor(),
            Backend::Sentinel(b) => b.descriptor(),
        }
    }

    pub fn health(&self) -> HealthState {
        match self {
            Backend::Fake(b) => b.health(),
            Backend::KeyPeek(b) => b.health(),
            Backend::Kanata(b) => b.health(),
            Backend::Sentinel(b) => b.health(),
        }
    }

    pub fn poll(&mut self) -> Vec<BackendUpdate> {
        match self {
            Backend::Fake(b) => b.poll(),
            Backend::KeyPeek(b) => b.poll(),
            Backend::Kanata(b) => b.poll(),
            Backend::Sentinel(b) => b.poll(),
        }
    }

    /// Feed a Host Input Event to a sentinel backend (a no-op otherwise).
    pub fn feed_host_event(&self, host_key: String, down: bool) {
        if let Backend::Sentinel(b) = self {
            let event = if down {
                HostEvent::KeyDown(host_key)
            } else {
                HostEvent::KeyUp(host_key)
            };
            b.feed(event);
        }
    }

    /// The id this backend reports updates under.
    pub fn id(&self) -> String {
        self.descriptor().id
    }

    /// Re-queue the scripted demo (Fake only); a no-op for live backends.
    pub fn replay(&mut self) {
        if let Backend::Fake(b) = self {
            b.replay();
        }
    }
}

pub struct AppState {
    pub inner: Mutex<Inner>,
}

pub struct Inner {
    pub composer: Composer,
    pub backend: Backend,
    pub profile: Profile,
    /// Whether the overlay has been revealed (it stays hidden until the first
    /// snapshot, ADR 0044).
    pub overlay_shown: bool,
}

impl AppState {
    pub fn new() -> Self {
        let (backend, model) = FakeBackend::demo();
        Self {
            inner: Mutex::new(Inner::with_backend(Backend::Fake(backend), model)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl Inner {
    fn with_backend(backend: Backend, model: KeyboardModel) -> Self {
        let mut composer = Composer::new(model.clone(), StateConfidence::Authoritative);
        composer.register_backend(&backend.descriptor(), backend.health());
        let profile = Profile::new("demo-profile", model);
        Self {
            composer,
            backend,
            profile,
            overlay_shown: false,
        }
    }

    /// Rebuild the composer from the active profile with User Overrides applied
    /// (ADR 0018), keeping the current backend registered. Call after the
    /// profile changes (import, hand edit, override promotion).
    pub fn rebuild_from_profile(&mut self) {
        let model = self.profile.resolved_model();
        let mut composer = Composer::new(model, StateConfidence::Authoritative);
        composer.register_backend(&self.backend.descriptor(), self.backend.health());
        self.composer = composer;
    }

    /// Swap in a new backend and its model (e.g. after connecting a device).
    pub fn set_backend(&mut self, backend: Backend, model: KeyboardModel) {
        let mut composer = Composer::new(model, StateConfidence::Authoritative);
        composer.register_backend(&backend.descriptor(), backend.health());
        self.composer = composer;
        self.backend = backend;
    }

    /// Swap the backend while keeping the current model (e.g. Kanata, which
    /// streams runtime layer state but relies on a companion profile for the
    /// keyboard model, ADR 0010).
    pub fn set_backend_keep_model(&mut self, backend: Backend) {
        let model = self.composer.model().clone();
        let mut composer = Composer::new(model, StateConfidence::Authoritative);
        composer.register_backend(&backend.descriptor(), backend.health());
        self.composer = composer;
        self.backend = backend;
    }
}
