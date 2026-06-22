//! EDN Profile Codec: round trips, determinism, schema versioning, migrations,
//! stable IDs, and User Overrides (PRD Testing Decisions).

use keyplane_core::action::{RawAction, SemanticAction};
use keyplane_core::backend::fake::demo_model;
use keyplane_core::ids::{KeyId, LayerId};
use keyplane_core::import::promote_override;
use keyplane_core::profile::{Profile, ProfileError};
use keyplane_core::provenance::SourceKind;
use serde_json::json;

fn demo_profile() -> Profile {
    Profile::new("demo-profile", demo_model())
}

#[test]
fn edn_round_trip_preserves_the_profile() {
    let profile = demo_profile();
    let edn = profile.to_edn_str();
    let reloaded = Profile::from_edn_str(&edn).expect("reload");
    assert_eq!(reloaded, profile);
}

#[test]
fn save_is_deterministic() {
    let profile = demo_profile();
    assert_eq!(profile.to_edn_str(), profile.to_edn_str());
    // And stable across a reload, so diffs stay quiet.
    let reloaded = Profile::from_edn_str(&profile.to_edn_str()).unwrap();
    assert_eq!(reloaded.to_edn_str(), profile.to_edn_str());
}

#[test]
fn saved_edn_uses_namespaced_sections_and_records_schema() {
    let edn = demo_profile().to_edn_str();
    assert!(edn.starts_with("{\n  :schema/version 1"));
    for section in [
        ":profile/id",
        ":keyboard/physical-layout",
        ":keyboard/keymap",
        ":runtime/backends",
        ":visual/style",
        ":overlay/window",
        ":user/overrides",
    ] {
        assert!(edn.contains(section), "missing section {section}");
    }
    // Geometry must serialize as floats so it reloads as Float, not Int.
    assert!(edn.contains(":w 1.0"));
}

#[test]
fn geometry_round_trips_as_float_not_int() {
    let profile = demo_profile();
    let reloaded = Profile::from_edn_str(&profile.to_edn_str()).unwrap();
    let original = &profile.model.physical_layout.keys[0].geometry;
    let again = &reloaded.model.physical_layout.keys[0].geometry;
    assert_eq!(original.w, again.w);
    assert_eq!(original.x, again.x);
}

#[test]
fn future_schema_version_is_rejected() {
    let edn = r#"{:schema/version 999 :profile/id "x"}"#;
    assert_eq!(
        Profile::from_edn_str(edn),
        Err(ProfileError::UnsupportedVersion(999))
    );
}

#[test]
fn v0_profile_migrates_to_current_and_gains_default_style() {
    // A pre-versioned profile: no :schema/version, no :visual/style.
    let v0 = r#"
    {:profile/id "old"
     :keyboard/physical-layout {:layout/fallback false :layout/keys []}
     :keyboard/keymap {:keymap/layers []}
     :runtime/backends []
     :overlay/window {:overlay/visibility :pinned
                      :overlay/click-through true
                      :overlay/always-on-top true
                      :overlay/display {}}
     :user/overrides []}
    "#;
    let profile = Profile::from_edn_str(v0).expect("migrate + decode");
    assert_eq!(profile.schema_version, 1);
    // The migration supplied the default style.
    assert_eq!(profile.model.style.opacity, 0.92);
}

#[test]
fn stable_key_ids_survive_a_round_trip() {
    let profile = demo_profile();
    let ids: Vec<_> = profile
        .model
        .physical_layout
        .keys
        .iter()
        .map(|k| k.id.clone())
        .collect();
    let reloaded = Profile::from_edn_str(&profile.to_edn_str()).unwrap();
    let reloaded_ids: Vec<_> = reloaded
        .model
        .physical_layout
        .keys
        .iter()
        .map(|k| k.id.clone())
        .collect();
    assert_eq!(ids, reloaded_ids);
}

#[test]
fn user_override_promotion_is_recorded_and_wins_future_saves() {
    let mut profile = demo_profile();
    promote_override(
        &mut profile,
        "keymap.layer-0.k0",
        json!({"qmk": "KC_ESC"}),
        Some("I remapped this".into()),
    );
    assert_eq!(profile.user_overrides.len(), 1);
    assert_eq!(profile.user_overrides[0].field, "keymap.layer-0.k0");

    // Overrides survive an EDN round trip.
    let reloaded = Profile::from_edn_str(&profile.to_edn_str()).unwrap();
    assert_eq!(reloaded.user_overrides, profile.user_overrides);

    // Re-promoting the same field replaces rather than duplicates.
    promote_override(&mut profile, "keymap.layer-0.k0", json!({"qmk": "KC_TAB"}), None);
    assert_eq!(profile.user_overrides.len(), 1);
}

#[test]
fn user_override_wins_in_the_resolved_model() {
    let mut profile = demo_profile();
    // Base-layer k0 is "A"; the user remaps it to Escape.
    promote_override(
        &mut profile,
        "keymap.layer-0.k0",
        json!({"source": "qmk", "value": "KC_ESC"}),
        Some("remap".into()),
    );
    let model = profile.resolved_model();
    let entry = model
        .keymap
        .layer(&LayerId::new("layer-0"))
        .unwrap()
        .entry(&KeyId::new("k0"))
        .unwrap();
    assert_eq!(entry.raw, RawAction::Qmk("KC_ESC".into()));
    assert_eq!(entry.semantic, SemanticAction::Key { label: "Esc".into() });
    assert_eq!(entry.provenance.as_ref().unwrap().kind, SourceKind::User);

    // The un-resolved model is untouched (the override only applies on resolve).
    let original = profile
        .model
        .keymap
        .layer(&LayerId::new("layer-0"))
        .unwrap()
        .entry(&KeyId::new("k0"))
        .unwrap();
    assert_eq!(original.raw, RawAction::Qmk("KC_A".into()));
}
