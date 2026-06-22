//! Source Precedence (ADR 0018).
//!
//! Resolves Source Conflicts per field. User Overrides always win. Otherwise
//! the winner depends on the field: firmware-aware sources win Runtime State,
//! imported geometry/keymap sources win Physical Layout and Logical Keymap, and
//! keyviz wins only Visual Style. Losing candidates stay inspectable as
//! [`Sourced::alternatives`].

use crate::provenance::{Candidate, SourceKind, Sourced};
use serde::{Deserialize, Serialize};

/// The class of Profile/Keyboard-Model field a value belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Field {
    /// Live layer/runtime values.
    RuntimeState,
    /// Per-key geometry.
    PhysicalLayout,
    /// Layer keymap data.
    LogicalKeymap,
    /// Presentation-only fields.
    VisualStyle,
}

/// The precedence rank of a source for a field. Higher wins; `None` means the
/// source may not supply this field at all (it is recorded but never selected).
pub fn rank(kind: SourceKind, field: Field) -> Option<u32> {
    // User Overrides always win, for every field (ADR 0018).
    if kind == SourceKind::User {
        return Some(1000);
    }
    match field {
        Field::RuntimeState => match kind {
            // Firmware-aware and authoritative remapper sources win runtime.
            SourceKind::KeyPeek => Some(90),
            SourceKind::Kanata => Some(85),
            SourceKind::Fake => Some(80),
            SourceKind::Sentinel => Some(40), // lower confidence (ADR 0016)
            _ => Some(10),
        },
        Field::PhysicalLayout | Field::LogicalKeymap => match kind {
            SourceKind::KeyPeek => Some(90),
            SourceKind::Vial => Some(80),
            SourceKind::Via => Some(78),
            SourceKind::Zmk => Some(76),
            // OverKeys row arrays import only as a Fallback Layout.
            SourceKind::OverKeys => Some(40),
            SourceKind::Fake => Some(30),
            // keyviz affects only Visual Style.
            SourceKind::Keyviz => None,
            _ => Some(10),
        },
        Field::VisualStyle => match kind {
            SourceKind::Keyviz => Some(80),
            SourceKind::OverKeys => Some(60),
            _ => Some(20),
        },
    }
}

/// Resolve a set of candidates for `field` into a winner plus alternatives.
///
/// Returns `None` only when `candidates` is empty. Candidates a source may not
/// supply (rank `None`) are never selected but are retained as alternatives so
/// the Source Inspector can show them.
pub fn resolve<T: Clone>(field: Field, candidates: Vec<Candidate<T>>) -> Option<Sourced<T>> {
    if candidates.is_empty() {
        return None;
    }

    // Find the best eligible candidate by rank; ties keep first-seen order.
    let mut best: Option<usize> = None;
    let mut best_rank: u32 = 0;
    for (i, c) in candidates.iter().enumerate() {
        if let Some(r) = rank(c.provenance.kind, field) {
            if best.is_none() || r > best_rank {
                best = Some(i);
                best_rank = r;
            }
        }
    }
    // If nothing is eligible for this field, fall back to the first candidate.
    let winner_idx = best.unwrap_or(0);

    let mut alternatives = Vec::new();
    let mut winner: Option<Candidate<T>> = None;
    for (i, c) in candidates.into_iter().enumerate() {
        if i == winner_idx {
            winner = Some(c);
        } else {
            alternatives.push(c);
        }
    }
    let winner = winner.expect("winner_idx is in range");

    Some(Sourced {
        value: winner.value,
        provenance: winner.provenance,
        alternatives,
    })
}
