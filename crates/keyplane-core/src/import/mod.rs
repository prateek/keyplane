//! Importers and the Import Review pipeline (ADR 0008, 0034, 0046).
//!
//! An [`Importer`] reads an external source and produces an [`ImportCandidate`]
//! with normalized data and Source Provenance. Importers never mutate the
//! Active Profile (ADR 0034). [`ImportReview`] previews a candidate against an
//! existing profile, surfacing Source Conflicts resolved by
//! [`crate::precedence`] before the user commits.

pub mod keyviz;
pub mod overkeys;
pub mod vial;
pub mod zmk;

pub use keyviz::KeyvizStyleImporter;
pub use overkeys::OverKeysImporter;
pub use vial::VialFileImporter;
pub use zmk::ZmkKeymapImporter;

use crate::ids::{KeyId, LayerId};
use crate::model::KeyboardModel;
use crate::precedence::{self, Field};
use crate::profile::{Profile, SourceRef, UserOverride};
use crate::provenance::{Candidate, Provenance, SourceKind};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Errors raised while importing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    #[error("failed to parse {format}: {detail}")]
    Parse { format: String, detail: String },
    #[error("unsupported {format} data: {detail}")]
    Unsupported { format: String, detail: String },
}

/// Normalized data produced by an importer, before it becomes a Profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportCandidate {
    pub source: SourceRef,
    pub model: KeyboardModel,
    /// Human-facing notes for Import Review, e.g. why it is Best-Effort Preview.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// True when the source cannot guarantee live layer correctness (ADR 0007).
    pub best_effort_preview: bool,
}

impl ImportCandidate {
    /// Commit this candidate as a brand-new Profile (ADR 0034).
    pub fn into_new_profile(self, profile_id: impl Into<String>) -> Profile {
        let mut profile = Profile::new(profile_id, self.model);
        profile.sources = vec![self.source];
        profile
    }
}

/// A component that reads one external format into an [`ImportCandidate`].
pub trait Importer {
    /// A stable importer id (also used as the produced source id).
    fn id(&self) -> &str;
    fn kind(&self) -> SourceKind;
    /// Parse `input` into a candidate. Must not touch any Profile.
    fn import(&self, input: &str) -> Result<ImportCandidate, ImportError>;
}

/// One Source Conflict surfaced during Import Review.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldConflict {
    /// Dotted field path, e.g. `keymap.layer-0.k3`.
    pub field: String,
    /// The value currently in the profile (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<Candidate<String>>,
    /// The value the candidate would introduce.
    pub incoming: Candidate<String>,
    /// The provenance of the value precedence would select.
    pub winner: Provenance,
}

/// The diff and conflicts for committing a candidate over an existing profile.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportReview {
    pub source: Option<SourceRef>,
    /// Count of keymap entries the candidate introduces that the profile lacks.
    pub additions: usize,
    /// Fields where the candidate and the profile disagree.
    pub conflicts: Vec<FieldConflict>,
    pub notes: Vec<String>,
    pub best_effort_preview: bool,
}

impl ImportReview {
    /// Build a review of `candidate` against `existing` (the Active Profile).
    ///
    /// With no existing profile every entry is an addition and there are no
    /// conflicts. Conflicts compare keymap entries by `(layer, key)` raw token.
    pub fn build(existing: Option<&Profile>, candidate: &ImportCandidate) -> ImportReview {
        let mut review = ImportReview {
            source: Some(candidate.source.clone()),
            notes: candidate.notes.clone(),
            best_effort_preview: candidate.best_effort_preview,
            ..Default::default()
        };

        for layer in &candidate.model.keymap.layers {
            for (key, entry) in &layer.entries {
                let incoming_token = entry.raw.token();
                let incoming = Candidate {
                    value: incoming_token.clone(),
                    provenance: Provenance::new(
                        candidate.source.id.clone(),
                        candidate.source.kind,
                    )
                    .with_raw(incoming_token.clone()),
                };
                match existing_entry(existing, &layer.id, key) {
                    None => review.additions += 1,
                    Some(current) if current.value != incoming.value => {
                        let resolved = precedence::resolve(
                            Field::LogicalKeymap,
                            vec![current.clone(), incoming.clone()],
                        )
                        .expect("two candidates resolve");
                        review.conflicts.push(FieldConflict {
                            field: format!("keymap.{}.{}", layer.id, key),
                            current: Some(current),
                            incoming,
                            winner: resolved.provenance,
                        });
                    }
                    Some(_) => { /* identical: not a conflict */ }
                }
            }
        }
        review
    }
}

/// Promote a chosen value to a User Override so future imports cannot silently
/// replace it (ADR 0018). The value is stored as free-form JSON.
pub fn promote_override(
    profile: &mut Profile,
    field: impl Into<String>,
    value: JsonValue,
    note: Option<String>,
) {
    let field = field.into();
    profile.user_overrides.retain(|o| o.field != field);
    profile.user_overrides.push(UserOverride { field, value, note });
}

fn existing_entry(
    profile: Option<&Profile>,
    layer: &LayerId,
    key: &KeyId,
) -> Option<Candidate<String>> {
    let profile = profile?;
    let entry = profile.model.keymap.layer(layer)?.entry(key)?;
    let provenance = entry
        .provenance
        .clone()
        .unwrap_or_else(|| Provenance::new(profile.id.clone(), SourceKind::User));
    Some(Candidate {
        value: entry.raw.token(),
        provenance,
    })
}
