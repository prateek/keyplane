//! Shared application state behind a mutex.
//!
//! The [`Composer`] (model + runtime + health), the active [`FakeBackend`], and
//! the active [`Profile`] live here. Commands and the driver loop both lock this
//! state; all keyboard logic stays in `keyplane-core`.

use keyplane_core::backend::{FakeBackend, ProtocolBackend};
use keyplane_core::compose::Composer;
use keyplane_core::model::{KeyboardModel, StateConfidence};
use keyplane_core::profile::Profile;
use std::sync::Mutex;

/// Tauri event names the frontend subscribes to.
pub const EVENT_RUNTIME: &str = "keyplane://runtime-event";
pub const EVENT_SNAPSHOT: &str = "keyplane://snapshot";

/// The backend id used for the MVP fake backend.
pub const FAKE_BACKEND_ID: &str = "fake";

pub struct AppState {
    pub inner: Mutex<Inner>,
}

pub struct Inner {
    pub composer: Composer,
    pub backend: FakeBackend,
    pub profile: Profile,
    /// Whether the overlay has been revealed (it stays hidden until the first
    /// snapshot, ADR 0044).
    pub overlay_shown: bool,
}

impl AppState {
    pub fn new() -> Self {
        let (backend, model) = FakeBackend::demo();
        Self {
            inner: Mutex::new(Inner::with_model(backend, model)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl Inner {
    fn with_model(backend: FakeBackend, model: KeyboardModel) -> Self {
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

    /// Replace the active keyboard model (after an import or a hand edit),
    /// rebuilding the composer while keeping the fake backend connected.
    pub fn replace_model(&mut self, model: KeyboardModel) {
        let mut composer = Composer::new(model, StateConfidence::Authoritative);
        composer.register_backend(&self.backend.descriptor(), self.backend.health());
        self.composer = composer;
    }
}
