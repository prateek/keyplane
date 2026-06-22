# Keyplane

Keyplane is a planned cross-platform keyboard layer overlay built with Tauri. It starts from KeyPeek's Rust protocol and domain code, replaces the egui UI with a Tauri shell and web frontend, and focuses on full-keyboard overlays for layered keyboards.

The first product target is narrow:

- render a full keyboard overlay from a normalized Keyboard Model
- stream live Layer Stack changes from a KeyPeek-derived backend
- import NocFree/Vial `.vil` files as Best-Effort Preview
- save hand-editable app-native EDN profiles
- make backend health and permission failures visible

The repo is docs-first right now. The implementation should start from [the PRD](docs/prd/keyplane-prd.md) and the domain language in [CONTEXT.md](CONTEXT.md).

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

## Implementation Starting Point

1. Fork or vendor the relevant KeyPeek Rust protocol/domain code.
2. Scaffold a Tauri v2 app with React, TypeScript, and Vite.
3. Implement the Fake Backend and Keyboard Snapshot DTO first.
4. Render the full-keyboard overlay in a Rust-owned Tauri Overlay Window.
5. Add EDN Profile Codec support.
6. Add the NocFree/Vial `.vil` Import Candidate path.

Do not copy OverKeys implementation code. OverKeys is design inspiration and an import target.
