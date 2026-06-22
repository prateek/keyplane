# Keyplane

Keyplane is a cross-platform keyboard layer overlay built with Tauri, Rust, React, TypeScript, and Vite. It focuses on full-keyboard overlays for layered keyboards.

The current implementation includes:

- a normalized Keyboard Model DTO boundary shared by Rust and the frontend
- a Fake Backend that emits Keyboard Snapshots and Layer Stack Runtime Events
- a transparent, frameless, always-on-top Tauri Overlay Window owned by Rust
- Effective Action resolution for transparent keys and inherited legends
- EDN v1 Profile Codec save/load support with deterministic output and toolbar import/export actions
- NocFree/Vial `.vil` JSON import as a Best-Effort Preview Import Candidate, including backup-style layer matrices
- keyviz style JSON import as a style-only Import Candidate
- a Rust-owned Active Profile store with Import Review commit support
- App Window surfaces for the overlay, Import Review, Source Inspector, Backend Health, and Positioning Mode
- Rust-owned Overlay Window drag and resize controls while Positioning Mode is active

The remaining PRD scope includes KeyPeek-derived live hardware support, deeper importer coverage, permission prompts, and release packaging work beyond local debug bundles.

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
