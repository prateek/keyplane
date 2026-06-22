//! Importers and Import Review (ADR 0008, 0034, 0046).

use keyplane_core::action::SemanticAction;
use keyplane_core::import::{
    ImportReview, Importer, KeyvizStyleImporter, OverKeysImporter, VialFileImporter,
};
use keyplane_core::model::StyleVariant;
use keyplane_core::profile::Profile;
use keyplane_core::provenance::SourceKind;

const DEMO_VIL: &str = include_str!("fixtures/demo.vil");
const OVERKEYS: &str = include_str!("fixtures/overkeys.json");
const KEYVIZ: &str = include_str!("fixtures/keyviz-style.json");

#[test]
fn vial_import_is_best_effort_preview_with_fallback_geometry() {
    let candidate = VialFileImporter::new().import(DEMO_VIL).expect("import");
    assert_eq!(candidate.source.kind, SourceKind::Vial);
    assert!(candidate.best_effort_preview);
    assert!(
        candidate.model.physical_layout.fallback,
        "a .vil has no real geometry"
    );
    assert_eq!(candidate.model.keymap.layers.len(), 2);
    assert!(!candidate.notes.is_empty());
}

#[test]
fn vial_entries_preserve_raw_provenance_and_derive_semantics() {
    let candidate = VialFileImporter::new().import(DEMO_VIL).expect("import");
    let base = &candidate.model.keymap.layers[0];
    let mo = base
        .entries
        .values()
        .find(|e| matches!(e.semantic, SemanticAction::Layer { .. }))
        .expect("MO(1) becomes a layer action");
    let prov = mo.provenance.as_ref().expect("provenance preserved");
    assert_eq!(prov.kind, SourceKind::Vial);
    assert_eq!(prov.raw.as_deref(), Some("MO(1)"));
}

#[test]
fn overkeys_row_arrays_import_as_fallback_layout() {
    let candidate = OverKeysImporter::new().import(OVERKEYS).expect("import");
    assert_eq!(candidate.source.kind, SourceKind::OverKeys);
    assert!(candidate.model.physical_layout.fallback);
    assert_eq!(candidate.model.keymap.layers.len(), 1);
    assert_eq!(candidate.model.physical_layout.keys.len(), 8);
}

#[test]
fn keyviz_import_affects_visual_style_only() {
    let candidate = KeyvizStyleImporter::new().import(KEYVIZ).expect("import");
    assert_eq!(candidate.source.kind, SourceKind::Keyviz);
    assert!(!candidate.best_effort_preview);
    // No keyboard data — style only.
    assert!(candidate.model.keymap.layers.is_empty());
    assert!(candidate.model.physical_layout.keys.is_empty());
    assert_eq!(candidate.model.style.accent.as_deref(), Some("#ff6600"));
    assert_eq!(candidate.model.style.variant, StyleVariant::Minimal);
}

#[test]
fn import_review_against_empty_profile_is_all_additions() {
    let candidate = VialFileImporter::new().import(DEMO_VIL).unwrap();
    let review = ImportReview::build(None, &candidate);
    assert!(review.conflicts.is_empty());
    assert!(review.additions > 0);
    assert!(review.best_effort_preview);
}

#[test]
fn import_review_surfaces_conflicts_with_provenance() {
    // Commit one .vil, then re-import a variant that changes r0c0.
    let original = VialFileImporter::new().import(DEMO_VIL).unwrap();
    let profile: Profile = original.into_new_profile("p");

    let changed = DEMO_VIL.replacen("\"KC_A\"", "\"KC_Z\"", 1);
    let candidate = VialFileImporter::new().import(&changed).unwrap();

    let before = profile.clone();
    let review = ImportReview::build(Some(&profile), &candidate);

    // Importing/reviewing must not mutate the Active Profile (ADR 0034).
    assert_eq!(profile, before);

    let conflict = review
        .conflicts
        .iter()
        .find(|c| c.field == "keymap.layer-0.r0c0")
        .expect("the changed cell is a conflict");
    assert_eq!(conflict.current.as_ref().unwrap().value, "KC_A");
    assert_eq!(conflict.incoming.value, "KC_Z");
    // Both sides are Vial, so the incumbent wins the tie but the loser stays
    // inspectable on the conflict record.
    assert_eq!(conflict.winner.kind, SourceKind::Vial);
}
