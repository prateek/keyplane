//! Focused behavior tests for legends, capabilities, and Source Precedence.

use keyplane_core::health::{Capability, CapabilitySet};
use keyplane_core::legend::DisplayLegend;
use keyplane_core::precedence::{rank, resolve, Field};
use keyplane_core::provenance::{Candidate, Provenance, SourceKind};
use keyplane_core::resolve::legend_for;
use keyplane_core::SemanticAction;

#[test]
fn minimal_variant_collapses_tap_hold_to_its_tap_role() {
    let legend = legend_for(&SemanticAction::TapHold {
        tap: "Space".into(),
        hold: "Ctrl".into(),
    });
    // Detailed keeps both roles...
    assert_eq!(legend.tap.as_deref(), Some("Space"));
    assert_eq!(legend.hold.as_deref(), Some("Ctrl"));
    // ...Minimal collapses to the tap action.
    assert_eq!(legend.collapse(), "Space");
}

#[test]
fn empty_legend_collapses_to_empty_string() {
    assert_eq!(DisplayLegend::default().collapse(), "");
}

#[test]
fn capability_set_is_sorted_and_deduped() {
    let set = CapabilitySet::new([
        Capability::StreamLayerStack,
        Capability::Poll,
        Capability::StreamLayerStack,
    ]);
    let caps: Vec<_> = set.iter().collect();
    assert_eq!(caps.len(), 2, "duplicates removed");
    assert!(set.has(Capability::Poll));
}

#[test]
fn user_override_always_wins_precedence() {
    assert!(rank(SourceKind::User, Field::RuntimeState) > rank(SourceKind::KeyPeek, Field::RuntimeState));
    assert!(rank(SourceKind::User, Field::VisualStyle) > rank(SourceKind::Keyviz, Field::VisualStyle));
}

#[test]
fn keyviz_cannot_supply_keymap_but_wins_style() {
    assert_eq!(rank(SourceKind::Keyviz, Field::LogicalKeymap), None);
    assert!(rank(SourceKind::Keyviz, Field::VisualStyle).is_some());
}

#[test]
fn resolve_picks_highest_rank_and_keeps_losers() {
    let candidates = vec![
        Candidate {
            value: "from-overkeys",
            provenance: Provenance::new("ok", SourceKind::OverKeys),
        },
        Candidate {
            value: "from-vial",
            provenance: Provenance::new("vil", SourceKind::Vial),
        },
    ];
    let resolved = resolve(Field::PhysicalLayout, candidates).expect("non-empty");
    // Vial outranks an OverKeys Fallback Layout.
    assert_eq!(resolved.value, "from-vial");
    assert_eq!(resolved.alternatives.len(), 1);
    assert!(resolved.has_conflict());
}

#[test]
fn keypeek_wins_runtime_over_sentinel() {
    let candidates = vec![
        Candidate {
            value: "sentinel-guess",
            provenance: Provenance::new("s", SourceKind::Sentinel),
        },
        Candidate {
            value: "firmware-truth",
            provenance: Provenance::new("kp", SourceKind::KeyPeek),
        },
    ];
    let resolved = resolve(Field::RuntimeState, candidates).unwrap();
    assert_eq!(resolved.value, "firmware-truth");
}
