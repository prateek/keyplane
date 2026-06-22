//! The state composition seam (ADR 0036) — the app's primary contract.

use keyplane_core::backend::{BackendUpdate, FakeBackend, ProtocolBackend};
use keyplane_core::compose::Composer;
use keyplane_core::event::RuntimeEvent;
use keyplane_core::health::HealthState;
use keyplane_core::ids::LayerId;
use keyplane_core::model::{ActivationKind, ActiveLayer, LayerStack, StateConfidence};
use keyplane_core::snapshot::SNAPSHOT_SCHEMA;

fn demo_composer() -> (Composer, FakeBackend) {
    let (backend, model) = FakeBackend::demo();
    let mut composer = Composer::new(model, StateConfidence::Authoritative);
    let descriptor = backend.descriptor();
    composer.register_backend(&descriptor, backend.health());
    (composer, backend)
}

#[test]
fn snapshot_is_fully_resolved_and_carries_backends() {
    let (composer, _backend) = demo_composer();
    let snapshot = composer.snapshot();
    assert_eq!(snapshot.schema, SNAPSHOT_SCHEMA);
    assert_eq!(snapshot.keys.len(), 16);
    assert_eq!(snapshot.layers.len(), 3);
    assert_eq!(snapshot.backends.len(), 1);
    assert!(snapshot.backends[0].health.is_ok());
    // Seeded to the base layer.
    assert_eq!(snapshot.top_layer(), Some(&LayerId::new("layer-0")));
}

#[test]
fn layer_stack_update_re_resolves_and_emits_event() {
    let (mut composer, _backend) = demo_composer();
    let nav = LayerStack::new(vec![
        ActiveLayer::new(LayerId::new("layer-0"), ActivationKind::Default),
        ActiveLayer::new(LayerId::new("layer-1"), ActivationKind::Momentary),
    ]);
    let event = composer
        .apply(
            "fake",
            BackendUpdate::LayerStack {
                stack: nav.clone(),
                confidence: StateConfidence::Authoritative,
            },
        )
        .expect("layer change emits an event");
    match event {
        RuntimeEvent::LayerStack {
            layer_stack, keys, ..
        } => {
            assert_eq!(layer_stack, nav);
            // Rust ships re-resolved keys so the frontend just swaps them.
            assert_eq!(keys.len(), 16);
        }
        other => panic!("expected a layer-stack event, got {other:?}"),
    }
}

#[test]
fn identical_layer_stack_update_is_a_noop() {
    let (mut composer, _backend) = demo_composer();
    let base = LayerStack::base(LayerId::new("layer-0"));
    let event = composer.apply(
        "fake",
        BackendUpdate::LayerStack {
            stack: base,
            confidence: StateConfidence::Authoritative,
        },
    );
    assert!(event.is_none(), "the stack did not actually change");
}

#[test]
fn backend_health_change_flows_into_a_runtime_event() {
    let (mut composer, _backend) = demo_composer();
    let event = composer.set_backend_health(
        "fake",
        HealthState::Disconnected {
            detail: "device unplugged".into(),
        },
    );
    match event {
        Some(RuntimeEvent::BackendHealth { health }) => {
            assert_eq!(health.health.tag(), "disconnected");
        }
        other => panic!("expected a backend-health event, got {other:?}"),
    }
    // The snapshot now reflects the degraded backend so the UI can show it.
    assert_eq!(composer.snapshot().backends[0].health.tag(), "disconnected");
}

#[test]
fn fake_backend_streams_scripted_layer_changes() {
    let (mut composer, mut backend) = demo_composer();
    let mut layer_events = 0;
    // Drain the whole scripted demo.
    loop {
        let updates = backend.poll();
        if updates.is_empty() {
            break;
        }
        for update in updates {
            if let Some(RuntimeEvent::LayerStack { .. }) = composer.apply("fake", update) {
                layer_events += 1;
            }
        }
    }
    // The demo script visits base → nav → base → arrows → base; every actual
    // change emits an event (consecutive identical stacks are deduped).
    assert!(layer_events >= 3, "saw {layer_events} layer changes");
}
