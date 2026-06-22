//! MVP acceptance criteria (PRD "Testing Decisions" + ADR 0037), exercised
//! through the public seams without hardware. The one criterion that needs a
//! real device — a live KeyPeek-backed layer change — is covered structurally by
//! the Fake Backend here and by the hardware-gated `keyplane-keypeek` backend.
//!
//! A passing MVP demo should show: fake-backend live layer changes in the
//! overlay, a NocFree/Vial `.vil` Best-Effort Preview import, EDN save/load,
//! visible Backend Health, and Positioning Mode.

use keyplane_core::backend::{BackendUpdate, FakeBackend, ProtocolBackend};
use keyplane_core::compose::Composer;
use keyplane_core::event::RuntimeEvent;
use keyplane_core::health::HealthState;
use keyplane_core::import::{Importer, VialFileImporter};
use keyplane_core::model::StateConfidence;
use keyplane_core::profile::{Profile, VisibilityPolicy};

const DEMO_VIL: &str = include_str!("fixtures/demo.vil");

/// 1. Fake-backend live layer changes appear in the overlay (the resolved keys
///    actually change when the Layer Stack changes).
#[test]
fn fake_backend_drives_visible_layer_changes() {
    let (mut backend, model) = FakeBackend::demo();
    let mut composer = Composer::new(model, StateConfidence::Authoritative);
    composer.register_backend(&backend.descriptor(), backend.health());

    let base_keys = composer.snapshot().keys;

    // Drive the scripted demo until a layer-stack change re-resolves the keys.
    let mut changed = false;
    for _ in 0..32 {
        let updates = backend.poll();
        if updates.is_empty() {
            backend.replay();
            continue;
        }
        for update in updates {
            if let Some(RuntimeEvent::LayerStack { keys, .. }) = composer.apply("fake", update) {
                if keys != base_keys {
                    changed = true;
                }
            }
        }
        if changed {
            break;
        }
    }
    assert!(changed, "a layer change produced different rendered keys");
}

/// 2. A NocFree/Vial `.vil` import is a Best-Effort Preview (ADR 0007, 0046).
#[test]
fn vial_import_is_best_effort_preview() {
    let candidate = VialFileImporter::new().import(DEMO_VIL).expect("import");
    assert!(candidate.best_effort_preview);
    assert!(candidate.model.physical_layout.fallback);
}

/// 3. EDN save/load round-trips a profile deterministically.
#[test]
fn edn_save_and_load_round_trips() {
    let (_backend, model) = FakeBackend::demo();
    let profile = Profile::new("mvp", model);
    let edn = profile.to_edn_str();
    let reloaded = Profile::from_edn_str(&edn).expect("reload");
    assert_eq!(reloaded, profile);
    assert_eq!(reloaded.to_edn_str(), edn);
}

/// 4. Backend Health is visible — a disconnect flows into the snapshot and an
///    event the UI can render (ADR 0023).
#[test]
fn backend_health_is_visible() {
    let (backend, model) = FakeBackend::demo();
    let mut composer = Composer::new(model, StateConfidence::Authoritative);
    composer.register_backend(&backend.descriptor(), backend.health());
    assert!(composer.snapshot().backends[0].health.is_ok());

    let event = composer.set_backend_health(
        "fake",
        HealthState::Disconnected {
            detail: "unplugged".into(),
        },
    );
    assert!(matches!(event, Some(RuntimeEvent::BackendHealth { .. })));
    assert_eq!(composer.snapshot().backends[0].health.tag(), "disconnected");
}

/// 5. Positioning Mode is modeled: the overlay defaults to Pinned + click-through
///    (ADR 0025, 0026), the states Positioning Mode toggles between.
#[test]
fn positioning_mode_defaults_are_modeled() {
    let (_backend, model) = FakeBackend::demo();
    let profile = Profile::new("mvp", model);
    assert_eq!(profile.overlay.visibility, VisibilityPolicy::Pinned);
    assert!(profile.overlay.click_through);
}

/// A backend can also be driven directly (the path a real KeyPeek backend uses):
/// an injected layer-stack update re-resolves and emits.
#[test]
fn injected_layer_update_resolves_like_a_real_backend() {
    let (_backend, model) = FakeBackend::demo();
    let mut composer = Composer::new(model, StateConfidence::Authoritative);
    let nav = keyplane_core::model::LayerStack::new(vec![
        keyplane_core::model::ActiveLayer::new(
            keyplane_core::ids::LayerId::new("layer-0"),
            keyplane_core::model::ActivationKind::Default,
        ),
        keyplane_core::model::ActiveLayer::new(
            keyplane_core::ids::LayerId::new("layer-1"),
            keyplane_core::model::ActivationKind::Momentary,
        ),
    ]);
    let event = composer.apply(
        "any",
        BackendUpdate::LayerStack {
            stack: nav,
            confidence: StateConfidence::Authoritative,
        },
    );
    assert!(matches!(event, Some(RuntimeEvent::LayerStack { .. })));
}
