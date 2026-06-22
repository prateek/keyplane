//! Vendored KeyPeek code, reshaped for Keyplane.
//!
//! These modules are derived from KeyPeek (<https://github.com/srwi/keypeek>),
//! copyright Stephan Rumswinkel, licensed GPL-3.0-only. Keyplane reuses
//! KeyPeek's keycode-label tables and protocol code per the PRD; per the GPL the
//! derivation and attribution are explicit (see `NOTICE`).
//!
//! Changes from upstream:
//! - module paths rehomed under `crate::vendor::*`;
//! - modifier symbols use plain text instead of egui icon-font glyphs, dropping
//!   the egui/eframe UI dependency (Keyplane renders in a web frontend);
//! - the keycode-label data and parsing logic are otherwise unchanged.

pub mod device_discovery;
pub mod glyphs;
pub mod keycode_labels;
pub mod layout_key;
pub mod protocols;
pub mod zmk_keycode_labels;
