//! Pure schema migrations (ADR 0042).
//!
//! Migrations are pure `vN` → `vN+1` transforms over a parsed EDN value, run in
//! sequence until the profile reaches [`CURRENT_SCHEMA`]. Keeping them pure and
//! data-shaped (no I/O, no app state) makes them trivially testable and
//! replayable.

use super::{ProfileError, CURRENT_SCHEMA};
use crate::profile::edn::Edn;

/// Read the `:schema/version` from a parsed profile, defaulting to 0 when the
/// field is absent (the implicit pre-versioned shape).
fn read_version(edn: &Edn) -> Result<u32, ProfileError> {
    match edn.get("schema/version") {
        Some(v) => v
            .as_i64()
            .map(|n| n as u32)
            .ok_or_else(|| ProfileError::schema(":schema/version must be an integer")),
        None => Ok(0),
    }
}

/// Apply migrations until the value is at [`CURRENT_SCHEMA`].
pub fn migrate_to_current(mut edn: Edn) -> Result<Edn, ProfileError> {
    let mut version = read_version(&edn)?;
    if version > CURRENT_SCHEMA {
        return Err(ProfileError::UnsupportedVersion(version));
    }
    while version < CURRENT_SCHEMA {
        edn = step(version, edn)?;
        version += 1;
    }
    Ok(edn)
}

/// Migrate one version forward. Each arm is a pure data transform.
fn step(from: u32, edn: Edn) -> Result<Edn, ProfileError> {
    match from {
        // v0 → v1: the pre-versioned shape had no `:visual/style` and no
        // explicit `:schema/version`. Insert a default style and stamp v1.
        0 => Ok(migrate_v0_to_v1(edn)),
        other => Err(ProfileError::schema(format!(
            "no migration registered from schema version {other}"
        ))),
    }
}

fn migrate_v0_to_v1(edn: Edn) -> Edn {
    let Edn::Map(mut pairs) = edn else {
        return edn;
    };
    set_keyword(&mut pairs, "schema/version", Edn::Int(1));
    if !has_keyword(&pairs, "visual/style") {
        pairs.push((
            Edn::keyword("visual/style"),
            Edn::Map(vec![
                (Edn::keyword("style/variant"), Edn::keyword("detailed")),
                (Edn::keyword("style/show-inherited"), Edn::Bool(true)),
                (Edn::keyword("style/opacity"), Edn::Float(0.92)),
            ]),
        ));
    }
    Edn::Map(pairs)
}

fn has_keyword(pairs: &[(Edn, Edn)], name: &str) -> bool {
    pairs.iter().any(|(k, _)| k.as_keyword() == Some(name))
}

fn set_keyword(pairs: &mut Vec<(Edn, Edn)>, name: &str, value: Edn) {
    if let Some(slot) = pairs.iter_mut().find(|(k, _)| k.as_keyword() == Some(name)) {
        slot.1 = value;
    } else {
        pairs.push((Edn::keyword(name), value));
    }
}
