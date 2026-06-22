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
- Added a Profile Codec migration hook that migrates legacy v0 EDN Profiles to v1 and rejects unsupported future schema versions before typed Profile parsing.
- Added keyviz style JSON import as a style-only Import Candidate with visual-style Source Conflicts.
- Hardened NocFree/Vial `.vil` import for backup files whose `layout` is a layer matrix, with generated fallback geometry, numeric UID handling, and raw top-level section preservation.
- Added OverKeys companion JSON import as an import-only Best-Effort Preview path with row-array fallback geometry, raw labels, aliases, triggers, styles, and Kanata settings preserved as Source Provenance.
- Added deterministic Backend Health Runtime Events for permission-missing and recovery states so permission health is visible through the same snapshot/event path as live backends.
- Added a disconnected KeyPeek Live backend to the initial profile so firmware-backend capabilities and connection health are visible before hardware is connected.
- Added ZMK `.keymap` import as a Best-Effort Preview path that parses layer binding rows, preserves raw source text, and derives common ZMK layer-action semantics for visualization.
- Added a disconnected Kanata TCP Protocol Backend status with runtime-layer capabilities so Kanata health appears beside firmware backends before a runtime connection exists.
- Vendored a narrow GPL-3.0-only KeyPeek source slice at `third_party/keypeek` and moved KeyPeek firmware packet parsing into a named Rust contract module adapted from that source.
- Applied profile-owned Overlay Window targeting to the Rust-owned Tauri Overlay Window on initial snapshot, profile load, and import commit, with Positioning Mode updates persisted in the active Rust Profile.
- Added a Global Display Fallback for Profiles that omit Overlay Window Display Targeting, while preserving Profile-owned targeting when it is present.
- Added Active Profile controls for Overlay Visibility Policy and current Overlay Window visible state, with EDN persistence and backward-compatible loading for Profiles that predate `:overlay/visible?`.
- Added Fade Visibility runtime behavior that shows the Overlay Window on Runtime Events, hides it after an inactivity interval, and leaves Pinned Visibility and manual toggle behavior unchanged.
- Added a Sentinel Keys Protocol Backend and public Profile binding section that maps Host Input Events to lower-confidence Layer Stack Runtime Events without requiring OS input monitoring in tests.
- Added a KeyPeek Live Raw HID session using `qmk-via-api`, with subscription keepalive, layer and pressed-key packet mapping through the app-domain Runtime Event boundary, VID/PID connection controls, and Tauri event delivery to both App Window and Overlay Window renderers.
- Added launch-at-login Settings backed by `tauri-plugin-autostart`, with explicit Tauri autostart permissions and browser-preview fallback state.
- Added native Sentinel Key global shortcut registration through `tauri-plugin-global-shortcut`, with a Rust-owned shortcut registry, Runtime Event emission, registration rollback on partial failure, and a Settings toggle backed by typed Backend Health.
- Fixed Vial layer import resolution so matrix-addressed layer cells map to Physical Keys by `MatrixPosition` even when KLE geometry order differs from row/column order.
- Added a Vial device HID import path using KeyPeek-derived Vial definition reads, XZ decompression, `qmk-via-api` layer matrix reads, and Import Review wiring as a Best-Effort Preview rather than live layer sync.
- Added a Kanata TCP runtime path with local host/port Settings controls, newline-delimited JSON protocol parsing, Layer Stack Runtime Events from `LayerChange` and `CurrentLayerName`, and typed Backend Health from `HelloOk`, errors, reloads, disconnects, and parse failures.
- Added Source Provenance and Source records to the Keyboard Snapshot contract and surfaced them in the Source Inspector with raw preserved values.
- Expanded the Source Inspector to show concrete Backend Health messages and raw Source Provenance beside matching Source Conflict candidates.
- Added State Confidence reasons and Backend Health messages to the Overlay Surface status strip so inferred, stale, and disconnected state is visible without opening Source Inspector.
- Added an Active Profile diff summary to Import Review so Import Candidates show changed keys, layers, sources, provenance records, backends, style, and fallback-layout state before commit.
- Added selected-winner and raw Source Provenance rows to Import Review Source Conflicts before Import Candidate commit.
- Added density-aware structured Display Legend rendering so compact Visual Styles collapse to primary labels, standard styles show one secondary slot, and rich styles preserve all secondary Legend Slots.
- Added derived `tap-role` and `hold-role` Display Legend Slots for QMK and ZMK layer-tap actions while preserving compact primary labels.
- Added top active layer highlighting for non-inherited Effective Actions in the full-keyboard Overlay Surface.
- Added a Visual Style density Settings control backed by the Rust Active Profile store so compact, standard, and rich density choices persist into saved EDN Profiles.
- Added first-class Visual Style color tokens to the Profile contract, keyviz style import, EDN codec, and overlay renderer so imported keyviz keycap colors affect the full-keyboard overlay.
- Added macOS Accessibility and Input Monitoring permission checks and request prompts, surfaced through persistent Sentinel Keys Backend Health and Settings controls.
- Added typed Overlay Window Backend Health for window creation, transparency/click-through configuration, focusability, drag, and resize capability failures; Positioning Mode now makes the Overlay Window focusable only while placement controls are active.
- Added a GitHub Actions desktop build workflow that runs the Rust/frontend verification gate and uploads unsigned macOS debug `.app` and `.dmg` artifacts.
- Refreshed npm dependency metadata and made the Rolldown wasm runtime peer dependencies explicit so the desktop build workflow can use `npm ci` reliably.
- Extended the desktop build workflow with Linux and Windows debug binary builds using Tauri's no-bundle path.
- Updated GitHub-managed workflow actions to Node 24-compatible majors for checkout, Node setup, and artifact upload.
- Added env-gated KeyPeek Live hardware canaries for Raw HID subscription start/stop and observed layer-change Runtime Events, plus a hardware validation checklist for manual layer-change acceptance.
- Added a signed macOS release workflow scaffold that imports Apple signing certificates from GitHub Actions secrets, runs Tauri signing/notarization, uploads signed `.app` and `.dmg` artifacts, and documents the unverified Apple-credential boundary.

Verification:

- `cargo fmt --check`
- `cargo test` (91 Rust tests passed, 3 private local hardware canaries ignored by default)
- `KEYPLANE_LOCAL_VIL_CANDIDATE=<private .vil path> cargo test local_vil_candidate_file_imports_when_env_is_set -- --ignored`
- `npm ci`
- `npm test` (28 frontend tests)
- `npm run build`
- `actionlint .github/workflows/signed-release.yml .github/workflows/desktop-build.yml`
- `ruby -e 'require "yaml"; %w[.github/workflows/signed-release.yml .github/workflows/desktop-build.yml].each { |path| YAML.load_file(path) }'`
- `npm run tauri build -- --debug`
- `npm run tauri build -- --debug --no-bundle`
