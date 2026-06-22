// Keyplane — KeyPeek-derived protocol and domain code.
// Copyright (C) 2026 Keyplane contributors.
// Derived from KeyPeek (https://github.com/srwi/keypeek), © Stephan
// Rumswinkel, GPL-3.0-only. See `NOTICE`.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License version 3.

//! `keyplane-keypeek` reuses KeyPeek's Rust protocol and keycode-label code
//! (per the PRD) and adapts it to the Keyplane domain. The vendored tables turn
//! raw VIA `u16` keycodes into structured labels; [`bridge`] converts those into
//! Keyplane [`SemanticAction`](keyplane_core::SemanticAction)s, replacing the
//! hand-rolled token prettifier in `keyplane-core` for the numeric-keycode path.

pub mod backend;
pub mod bridge;
pub mod vendor;

pub use backend::{KeyPeekBackend, KeyPeekConnection};
pub use bridge::via_code_to_semantic;
pub use vendor::device_discovery::{discover_devices, DeviceKind, DiscoveredDevice};
pub use vendor::protocols::{ConnectionSpec, ZmkTransportConfig};
