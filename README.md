# Keyplane

Keyplane is a cross-platform keyboard layer overlay built with Tauri, Rust, React, TypeScript, and Vite. It focuses on full-keyboard overlays for layered keyboards.

The current implementation includes:

- a normalized Keyboard Model DTO boundary shared by Rust and the frontend
- a Fake Backend that emits Keyboard Snapshots and Layer Stack Runtime Events
- a transparent, frameless, always-on-top Tauri Overlay Window owned by Rust
- Effective Action resolution for transparent keys and inherited legends
- EDN v1 Profile Codec save/load support with deterministic output and toolbar import/export actions
- NocFree/Vial `.vil` JSON import as a Best-Effort Preview Import Candidate, including backup-style layer matrices
- Vial device HID import as a Best-Effort Preview Import Candidate, using KeyPeek-derived Vial definition and raw-matrix reads
- ZMK `.keymap` import as a Best-Effort Preview Import Candidate with fallback geometry
- keyviz style JSON import as a style-only Import Candidate
- OverKeys companion JSON import as a Best-Effort Preview Import Candidate with row-array fallback geometry
- a vendored KeyPeek protocol/domain source slice for live firmware packet attribution and drift checks
- a KeyPeek Live Raw HID path using `qmk-via-api`, with VID/PID connection controls and Tauri Runtime Event streaming
- a Rust-owned Active Profile store with Import Review commit support
- visible Backend Health for fake, KeyPeek Live, Kanata TCP, and Sentinel Keys runtime backends
- App Window surfaces for the overlay, Import Review, Source Inspector, Backend Health, and Positioning Mode
- Rust-owned Overlay Window drag and resize controls while Positioning Mode is active
- Profile-owned Overlay Window placement, size, click-through, visibility, and renderer opacity application
- Sentinel Key bindings in the public Profile contract, with lower-confidence Host Input Event ingestion through a Rust Protocol Backend
- native Sentinel Key global shortcut registration through a Rust-owned Tauri backend and Settings toggle
- launch-at-login Settings backed by the Tauri autostart plugin and scoped autostart permissions

The remaining PRD scope includes real KeyPeek-supported hardware validation, deeper native input-monitoring permission prompts beyond shortcut registration, and release packaging work beyond local debug bundles.

## Decisions

- Product name: `Keyplane`
- Repo and package slug: `keyplane`
- License: GPL-3.0
- Desktop shell: Tauri v2
- Frontend: React, TypeScript, Vite
- Backend/domain: Rust, reusing and reshaping KeyPeek protocol/domain code
- Canonical profile format: app-native EDN v1

## Development

Install dependencies:

```sh
npm install
```

Run the web frontend:

```sh
npm run dev
```

Run the Tauri app:

```sh
npm run tauri dev
```

Run checks:

```sh
npm test
npm run build
(cd src-tauri && cargo fmt --check && cargo test)
```

Build a local debug desktop bundle:

```sh
npm run tauri build -- --debug
```

## Documentation Map

- [PRD](docs/prd/keyplane-prd.md): product scope, acceptance criteria, implementation defaults
- [CONTEXT.md](CONTEXT.md): glossary and domain vocabulary
- [ADRs](docs/adr/): accepted architectural and product decisions
- [Ecosystem report](docs/research/keyboard-rendering-and-configuration-ecosystem.md): source research on keyboard rendering/configuration tools
- [Attribution](docs/attribution.md): GPL/source reuse notes
- [Implementation log](docs/implementation-log.md): execution notes and verification history
- [Agent instructions](AGENTS.md): how implementation agents should read the docs

## Source Boundaries

Do not copy OverKeys implementation code. OverKeys is design inspiration and an import target.
