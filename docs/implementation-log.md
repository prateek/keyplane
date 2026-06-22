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

Verification:

- `cargo fmt --check`
- `cargo test`
- `npm test`
- `npm run build`
- `npm run tauri build -- --debug`
