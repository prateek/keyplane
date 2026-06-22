# Keyplane

Keyplane is a planned cross-platform keyboard layer overlay built with Tauri. It starts from KeyPeek's Rust protocol and domain code, replaces the egui UI with a Tauri shell and web frontend, and focuses on full-keyboard overlays for layered keyboards.

The first product target is narrow:

- render a full keyboard overlay from a normalized Keyboard Model
- stream live Layer Stack changes from a KeyPeek-derived backend
- import NocFree/Vial `.vil` files as Best-Effort Preview
- save hand-editable app-native EDN profiles
- make backend health and permission failures visible

The MVP is being built against [the PRD](docs/prd/keyplane-prd.md) and the domain language in [CONTEXT.md](CONTEXT.md). See [the implementation log](docs/implementation-log.md) for architecture and progress.

## Decisions

- Product name: `Keyplane`
- Repo and package slug: `keyplane`
- License: GPL-3.0
- Desktop shell: Tauri v2
- Frontend: React, TypeScript, Vite
- Backend/domain: Rust, reusing and reshaping KeyPeek protocol/domain code
- Canonical profile format: app-native EDN v1

## Documentation Map

- [PRD](docs/prd/keyplane-prd.md): product scope, acceptance criteria, implementation defaults
- [CONTEXT.md](CONTEXT.md): glossary and domain vocabulary
- [ADRs](docs/adr/): accepted architectural and product decisions
- [Ecosystem report](docs/research/keyboard-rendering-and-configuration-ecosystem.md): source research on keyboard rendering/configuration tools
- [Agent instructions](AGENTS.md): how implementation agents should read the docs

## Layout

- `crates/keyplane-core/` — the domain core (no Tauri/HID/UI deps): Keyboard Model, Layer Stack resolution, Effective Actions, EDN Profile Codec, Protocol Backends, importers. Fully unit-tested.
- `src-tauri/` — the Tauri v2 shell: App + Overlay windows, command/event boundary, Fake Backend driver loop.
- `src/` — the React + TypeScript + Vite frontend: overlay surface and App Window.

## Building

```sh
pnpm install          # frontend deps
cargo test --workspace  # Rust domain tests
pnpm test             # frontend contract tests
pnpm build            # build the web frontend into dist/
cargo run -p keyplane   # run the desktop app (needs the dist/ build first)
```

Do not copy OverKeys implementation code. OverKeys is design inspiration and an import target.
