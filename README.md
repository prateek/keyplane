# Keyplane

Keyplane is a cross-platform keyboard layer overlay built with Tauri, Rust, React, TypeScript, and Vite. It focuses on full-keyboard overlays for layered keyboards.

The current implementation includes:

- a normalized Keyboard Model DTO boundary shared by Rust and the frontend, including stable Profile, keyboard/workflow, and Visual Style reference IDs
- a Fake Backend that emits Keyboard Snapshots and Layer Stack Runtime Events
- a transparent, frameless, always-on-top Tauri Overlay Window owned by Rust
- Effective Action resolution for transparent keys and inherited legends
- density-aware structured Display Legend rendering, including shifted symbol, action-hint, layer-tap, QMK mod-tap, and optional icon slots, plus Settings controls for compact, standard, and rich Visual Styles
- EDN v1 Profile Codec save/load support with deterministic output, schema migration gating, stable style references, and toolbar import/export actions
- NocFree/Vial `.vil` JSON import as a Best-Effort Preview Import Candidate, including backup-style layer matrices
- Vial device HID import as a Best-Effort Preview Import Candidate, using KeyPeek-derived Vial definition and raw-matrix reads
- KeyPeek/QMK `keyboard_info.json` import as a Best-Effort Preview Import Candidate, using KeyPeek-derived per-key matrix geometry parsing
- ZMK `.keymap` import as a Best-Effort Preview Import Candidate with fallback geometry
- keyviz style JSON import as a style-only Import Candidate with keycap color tokens and field-level Visual Style Source Conflicts
- OverKeys companion JSON import as a Kanata-ready Best-Effort Preview Import Candidate with row-array fallback geometry, display aliases, custom alias hints, custom shifted legends, Visual Style colors/opacity, Sentinel Key trigger bindings, and profile-owned Kanata TCP settings
- a vendored KeyPeek protocol/domain source slice for live firmware packet and QMK info geometry attribution and drift checks
- a KeyPeek Live Raw HID path using `qmk-via-api`, with VIA Raw HID discovery, VID/PID connection controls, and Tauri Runtime Event streaming
- a Kanata TCP runtime path with Settings host/port controls, newline JSON event parsing, and Layer Stack Runtime Event streaming
- a Rust-owned Active Profile store with Import Review commit support
- visible Backend Health and State Confidence reasons for fake, Overlay Window, KeyPeek Live, Kanata TCP, and Sentinel Keys runtime backends
- App Window surfaces for the overlay, Import Review with Active Profile diffs, provenance-backed Source Conflicts, and User Override promotion, Source Inspector, Source Precedence, Layer Stack precedence, Source Provenance, transparent-entry inheritance, Backend Health, and Positioning Mode
- active Layer Stack rendering that highlights non-inherited key actions supplied by the top active layer
- Rust-owned Overlay Window drag and resize controls while Positioning Mode is active, with unsupported click-through, focusability, positioning, and all-workspaces behavior surfaced as typed Backend Health
- Profile-owned Overlay Window placement, size, click-through, visibility policy, visible state, Fade Visibility inactivity behavior, and renderer opacity application with a Global Display Fallback for profiles without saved targeting
- Sentinel Key bindings in the public Profile contract, with lower-confidence Host Input Event ingestion through a Rust Protocol Backend
- native Sentinel Key global shortcut registration through a Rust-owned Tauri backend and Settings toggle
- macOS Accessibility and Input Monitoring permission checks and prompts surfaced as persistent Sentinel Keys Backend Health
- launch-at-login Settings backed by the Tauri autostart plugin and scoped autostart permissions
- a static Tauri capability check that keeps unused plugin permissions out of the default app/overlay window capability
- a GitHub Actions desktop build workflow that verifies the app, validates the signed-release workflow shape, uploads unsigned macOS debug bundles, and uploads Linux/Windows debug binaries
- a signed macOS release workflow scaffold for Apple certificate import, Tauri signing/notarization, signed `.app`/`.dmg` artifact upload, and sanitized signed-run evidence reports
- env-gated KeyPeek Live hardware canaries, a sanitized validation report runner, and a validation checklist for compatible firmware devices

The remaining PRD scope includes observed real KeyPeek-supported layer-change validation and first signed release execution with real Apple credentials.

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

The `Desktop Build` GitHub Actions workflow runs the same verification path on macOS, uploads unsigned debug `.app` and `.dmg` artifacts, and builds Linux/Windows debug binaries without installer bundling.
It also runs `npm run check:workflows` so signed-release workflow and Tauri capability drift are caught in PRs before real Apple credentials are available.

Validate KeyPeek Live hardware canaries:

```sh
KEYPLANE_KEYPEEK_LIVE_VID=feed \
KEYPLANE_KEYPEEK_LIVE_PID=cafe \
KEYPLANE_KEYPEEK_LIVE_WAIT_MS=10000 \
npm run validate:keypeek-live
```

The runner writes a sanitized report to `target/keyplane-validation/`.
Use `npm run validate:keypeek-live:dry` only to check report generation without
opening hardware.

Build a signed macOS release in CI:

```sh
git tag v0.1.0
git push origin v0.1.0
```

See [Signed Release Workflow](docs/release/signing.md) for required Apple secrets and the current verification boundary.

After the signed workflow has run with real Apple credentials, collect the
sanitized release evidence report:

```sh
KEYPLANE_SIGNED_RELEASE_RUN_ID=123456789 npm run validate:signed-release
```

Use `npm run validate:signed-release:dry` only to check report generation
without querying GitHub.

## Documentation Map

- [PRD](docs/prd/keyplane-prd.md): product scope, acceptance criteria, implementation defaults
- [CONTEXT.md](CONTEXT.md): glossary and domain vocabulary
- [ADRs](docs/adr/): accepted architectural and product decisions
- [Ecosystem report](docs/research/keyboard-rendering-and-configuration-ecosystem.md): source research on keyboard rendering/configuration tools
- [Attribution](docs/attribution.md): GPL/source reuse notes
- [Implementation log](docs/implementation-log.md): execution notes and verification history
- [Signed Release Workflow](docs/release/signing.md): macOS signing/notarization workflow and required CI secrets
- [KeyPeek Live hardware validation](docs/validation/keypeek-live-hardware.md): opt-in hardware canary and manual layer-change checklist
- [Agent instructions](AGENTS.md): how implementation agents should read the docs

## Source Boundaries

Do not copy OverKeys implementation code. OverKeys is design inspiration and an import target.
