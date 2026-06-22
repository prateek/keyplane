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
- Added Rust-owned Overlay Window drag and resize commands, with Positioning Mode temporarily enabling window resizability.
- Added a Rust-owned Active Profile store with Import Candidate commit, active EDN save/load commands, Source Inspector promotion against active profile state, and Best-Effort Preview state confidence.
- Added toolbar actions to save and load the Active Profile as hand-editable EDN.
- Added keyviz style JSON import as a style-only Import Candidate with visual-style Source Conflicts.
- Hardened NocFree/Vial `.vil` import for backup files whose `layout` is a layer matrix, with generated fallback geometry, numeric UID handling, and raw top-level section preservation.
- Added OverKeys companion JSON import as an import-only Best-Effort Preview path with row-array fallback geometry, raw labels, aliases, triggers, styles, and Kanata settings preserved as Source Provenance.
- Added deterministic Backend Health Runtime Events for permission-missing and recovery states so permission health is visible through the same snapshot/event path as live backends.
- Added a disconnected KeyPeek Live backend to the initial profile so firmware-backend capabilities and connection health are visible before hardware is connected.
- Added ZMK `.keymap` import as a Best-Effort Preview path that parses layer binding rows, preserves raw source text, and derives common ZMK layer-action semantics for visualization.
- Added a disconnected Kanata TCP Protocol Backend status with runtime-layer capabilities so Kanata health appears beside firmware backends before a runtime connection exists.
- Vendored a narrow GPL-3.0-only KeyPeek source slice at `third_party/keypeek` and moved KeyPeek firmware packet parsing into a named Rust contract module adapted from that source.
- Applied profile-owned Overlay Window targeting to the Rust-owned Tauri Overlay Window on initial snapshot, profile load, and import commit, with Positioning Mode updates persisted in the active Rust Profile.
- Added a Sentinel Keys Protocol Backend and public Profile binding section that maps Host Input Events to lower-confidence Layer Stack Runtime Events without requiring OS input monitoring in tests.
- Added a KeyPeek Live Raw HID session using `qmk-via-api`, with subscription keepalive, layer and pressed-key packet mapping through the app-domain Runtime Event boundary, VID/PID connection controls, and Tauri event delivery to both App Window and Overlay Window renderers.
- Added launch-at-login Settings backed by `tauri-plugin-autostart`, with explicit Tauri autostart permissions and browser-preview fallback state.
- Added native Sentinel Key global shortcut registration through `tauri-plugin-global-shortcut`, with a Rust-owned shortcut registry, Runtime Event emission, registration rollback on partial failure, and a Settings toggle backed by typed Backend Health.

Verification:

- `cargo fmt --check`
- `cargo test` (66 Rust tests passed, 1 private local `.vil` canary ignored by default)
- `KEYPLANE_LOCAL_VIL_CANDIDATE=<private .vil path> cargo test local_vil_candidate_file_imports_when_env_is_set -- --ignored`
- `npm test` (17 frontend tests)
- `npm run build`
- `npm run tauri build -- --debug`
