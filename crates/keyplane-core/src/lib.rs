// Keyplane — cross-platform keyboard layer overlay.
// Copyright (C) 2026 Keyplane contributors.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
// more details.
//
// Portions of the protocol/domain design derive from KeyPeek (GPL-3.0).

//! `keyplane-core` owns the Keyplane domain: the normalized [`Keyboard
//! Model`](model::KeyboardModel), Layer Stack resolution, Effective Action
//! derivation, the EDN [`Profile`](profile::Profile) codec, Importers, and
//! Protocol Backends.
//!
//! Per ADR 0036, Rust owns source composition, Backend Health, Layer Stack
//! resolution, and Effective Action derivation. The web frontend consumes
//! [`KeyboardSnapshot`](snapshot::KeyboardSnapshot) plus
//! [`RuntimeEvent`](event::RuntimeEvent)s and never re-implements protocol,
//! HID, or keymap resolution logic.

pub mod action;
pub mod backend;
pub mod compose;
pub mod event;
pub mod geometry;
pub mod health;
pub mod ids;
pub mod import;
pub mod legend;
pub mod model;
pub mod precedence;
pub mod profile;
pub mod provenance;
pub mod resolve;
pub mod snapshot;
pub mod visibility;

pub use action::{RawAction, SemanticAction};
pub use compose::Composer;
pub use event::RuntimeEvent;
pub use health::{BackendHealth, Capability, CapabilitySet, HealthState};
pub use ids::{KeyId, LayerId, SourceId, StyleId};
pub use model::KeyboardModel;
pub use profile::Profile;
pub use snapshot::KeyboardSnapshot;
