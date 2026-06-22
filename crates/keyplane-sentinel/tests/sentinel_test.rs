//! Sentinel-key layer inference (ADR 0016).

use keyplane_core::backend::{BackendUpdate, ProtocolBackend};
use keyplane_core::ids::LayerId;
use keyplane_core::model::{ActivationKind, StateConfidence};
use keyplane_core::provenance::SourceKind;
use keyplane_sentinel::{HostEvent, SentinelAction, SentinelBackend, SentinelKey, SentinelTracker};

fn tracker() -> SentinelTracker {
    SentinelTracker::new(
        vec![
            SentinelKey {
                host_key: "F13".into(),
                action: SentinelAction::Momentary(LayerId::new("layer-1")),
            },
            SentinelKey {
                host_key: "F14".into(),
                action: SentinelAction::Toggle(LayerId::new("layer-2")),
            },
        ],
        LayerId::new("layer-0"),
    )
}

#[test]
fn momentary_sentinel_activates_while_held_and_releases() {
    let mut t = tracker();
    let down = t.on_event(&HostEvent::KeyDown("F13".into()));
    assert_eq!(down.top().unwrap().layer, LayerId::new("layer-1"));
    assert_eq!(down.top().unwrap().activation, ActivationKind::Momentary);

    let up = t.on_event(&HostEvent::KeyUp("F13".into()));
    assert_eq!(up.active.len(), 1); // back to base
}

#[test]
fn toggle_sentinel_flips_on_and_off() {
    let mut t = tracker();
    let on = t.on_event(&HostEvent::KeyDown("F14".into()));
    assert!(on.contains(&LayerId::new("layer-2")));
    // Key-up does not clear a toggle.
    t.on_event(&HostEvent::KeyUp("F14".into()));
    let off = t.on_event(&HostEvent::KeyDown("F14".into()));
    assert!(!off.contains(&LayerId::new("layer-2")));
}

#[test]
fn unconfigured_keys_are_ignored() {
    let mut t = tracker();
    let stack = t.on_event(&HostEvent::KeyDown("A".into()));
    assert_eq!(stack.active.len(), 1);
}

#[test]
fn backend_emits_inferred_confidence() {
    let mut backend = SentinelBackend::new(
        vec![SentinelKey {
            host_key: "F13".into(),
            action: SentinelAction::Momentary(LayerId::new("layer-1")),
        }],
        LayerId::new("layer-0"),
    );
    assert_eq!(backend.descriptor().kind, SourceKind::Sentinel);
    backend.feed(HostEvent::KeyDown("F13".into()));
    match backend.poll().first() {
        Some(BackendUpdate::LayerStack { confidence, stack }) => {
            // Sentinel keys are lower confidence than firmware (ADR 0016).
            assert_eq!(*confidence, StateConfidence::Inferred);
            assert_eq!(stack.top().unwrap().layer, LayerId::new("layer-1"));
        }
        other => panic!("expected a layer-stack update, got {other:?}"),
    }
}
