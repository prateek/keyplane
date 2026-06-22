//! App-native EDN Profiles (ADR 0012, 0020, 0035, 0042).
//!
//! A [`Profile`] is the canonical saved configuration: imported keyboard data,
//! selected backends, visual style, overlay window settings, and user
//! overrides. The [`Profile Codec`](self) parses, validates, writes, and
//! migrates profiles; deterministic save formatting is owned here, not by the
//! EDN layer. Raw, hand-editable EDN is the public format.

pub mod codec;
pub mod edn;
pub mod migrate;

pub use edn::{Edn, EdnError};

use crate::ids::SourceId;
use crate::model::KeyboardModel;
use crate::provenance::SourceKind;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// The current Profile Schema Version (ADR 0042). Bump when the on-disk shape
/// changes; add a migration in [`migrate`].
pub const CURRENT_SCHEMA: u32 = 1;

/// Errors from loading or saving a Profile.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProfileError {
    #[error("EDN parse error: {0}")]
    Parse(#[from] EdnError),
    #[error("profile schema error: {0}")]
    Schema(String),
    #[error("unsupported profile schema version: {0} (max supported {max})", max = CURRENT_SCHEMA)]
    UnsupportedVersion(u32),
}

impl ProfileError {
    pub(crate) fn schema(msg: impl Into<String>) -> Self {
        ProfileError::Schema(msg.into())
    }
}

/// A declared source the profile draws data from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub id: SourceId,
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A configured Protocol Backend entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendConfig {
    pub id: String,
    pub kind: SourceKind,
    pub enabled: bool,
}

/// Overlay Visibility Policy (ADR 0026).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityPolicy {
    /// Visible until explicitly hidden — the MVP default.
    #[default]
    Pinned,
    /// Shown/hidden by a manual toggle.
    ManualToggle,
    /// Appears on activity, fades after inactivity.
    Fade,
}

/// Profile-owned Display Targeting (ADR 0027) with a Global Display Fallback:
/// any `None` field falls back to the app-level default at window-placement
/// time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DisplayTargeting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

/// Overlay Window configuration (ADR 0024, 0025, 0026, 0027).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub visibility: VisibilityPolicy,
    /// Click-Through Mode default (ADR 0025).
    pub click_through: bool,
    pub always_on_top: bool,
    pub display: DisplayTargeting,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            visibility: VisibilityPolicy::Pinned,
            click_through: true,
            always_on_top: true,
            display: DisplayTargeting::default(),
        }
    }
}

/// A User Override (ADR 0018): a user-authored value that always wins. The
/// value is stored as free-form JSON so overrides can target any field without
/// the codec knowing the field's type ahead of time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserOverride {
    /// Dotted field path the override targets, e.g. `keymap.layer-0.k0`.
    pub field: String,
    pub value: JsonValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The canonical saved Profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub schema_version: u32,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub sources: Vec<SourceRef>,
    /// Physical Layout + Logical Keymap + Visual Style live here; the codec
    /// splits them across the EDN `:keyboard/*` and `:visual/style` sections.
    pub model: KeyboardModel,
    pub backends: Vec<BackendConfig>,
    pub overlay: OverlayConfig,
    pub user_overrides: Vec<UserOverride>,
}

impl Profile {
    /// A new, empty profile at the current schema version.
    pub fn new(id: impl Into<String>, model: KeyboardModel) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA,
            id: id.into(),
            name: model.name.clone(),
            sources: Vec::new(),
            model,
            backends: Vec::new(),
            overlay: OverlayConfig::default(),
            user_overrides: Vec::new(),
        }
    }

    /// Parse, migrate, and decode a Profile from EDN text.
    pub fn from_edn_str(input: &str) -> Result<Profile, ProfileError> {
        let parsed = Edn::parse(input)?;
        let migrated = migrate::migrate_to_current(parsed)?;
        codec::decode(&migrated)
    }

    /// Encode and deterministically serialize this Profile to EDN text.
    pub fn to_edn_str(&self) -> String {
        codec::encode(self).to_edn_string()
    }
}
