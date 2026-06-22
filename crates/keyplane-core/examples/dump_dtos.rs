//! Dump serialized DTOs as JSON for the frontend contract tests.
//!
//! Run with `cargo run -p keyplane-core --example dump_dtos`. The output is the
//! exact serde shape the frontend consumes, so the TypeScript fixtures stay in
//! lockstep with Rust rather than being hand-authored.

use keyplane_core::backend::{BackendUpdate, FakeBackend, ProtocolBackend};
use keyplane_core::compose::Composer;
use keyplane_core::health::HealthState;
use keyplane_core::ids::LayerId;
use keyplane_core::model::{ActivationKind, ActiveLayer, LayerStack, StateConfidence};
use serde_json::json;

fn main() {
    let (backend, model) = FakeBackend::demo();
    let mut composer = Composer::new(model, StateConfidence::Authoritative);
    composer.register_backend(&backend.descriptor(), backend.health());

    let snapshot = composer.snapshot();

    let nav = LayerStack::new(vec![
        ActiveLayer::new(LayerId::new("layer-0"), ActivationKind::Default),
        ActiveLayer::new(LayerId::new("layer-1"), ActivationKind::Momentary),
    ]);
    let nav_event = composer
        .apply(
            "fake",
            BackendUpdate::LayerStack {
                stack: nav,
                confidence: StateConfidence::Authoritative,
            },
        )
        .expect("layer event");

    let health_event = composer
        .set_backend_health(
            "fake",
            HealthState::Disconnected {
                detail: "device unplugged".into(),
            },
        )
        .expect("health event");

    let out = json!({
        "snapshot": snapshot,
        "navEvent": nav_event,
        "healthEvent": health_event,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
