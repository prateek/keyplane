# Implementation Log

## 2026-06-22

Started the Tauri v2 implementation from the PRD.

- Added the React, TypeScript, and Vite frontend scaffold.
- Added Rust app-domain DTOs for Keyboard Snapshots, Runtime Events, Backend Health, Capability Flags, Import Candidates, Profiles, Display Legends, and Source Conflicts.
- Added a Fake Backend for hardware-free live overlay validation.
- Added Rust-owned Overlay Window creation with click-through enabled by default and Positioning Mode support.
- Added EDN v1 Profile Codec save/load support behind a Rust boundary.
- Added NocFree/Vial `.vil` JSON import as a Best-Effort Preview Import Candidate.
- Added frontend surfaces for the overlay, Source Inspector, Import Review, Backend Health, fake Runtime Events, and Positioning Mode.
- Added a KeyPeek-derived packet adapter for layer-state packets, pressed-key packets, subscription keepalive messages, and Layer Stack conversion.
- Added the Rust Protocol Backend trait boundary and routed the Fake Backend command path through it.
- Added Rust-owned Source Precedence conflict resolution with User Overrides winning over imported candidates while preserving losing values.
- Added Source Inspector promotion controls backed by snapshot-level User Overrides for the current demo state.

Verification:

- `cargo fmt --check`
- `cargo test` (21 Rust tests)
- `npm test` (7 frontend tests)
- `npm run build`
- `npm run tauri build -- --debug`
