# Implementation Log

Execution notes for building Keyplane against [the PRD](prd/keyplane-prd.md). The
PRD stays stable; progress and decisions live here.

## Architecture

```
keyplane/
├── Cargo.toml                 # workspace
├── crates/
│   ├── keyplane-core/         # domain core (no Tauri/HID/UI deps)
│   ├── keyplane-keypeek/      # KeyPeek-derived protocol + keycode code (GPL, vendored)
│   ├── keyplane-kanata/       # Kanata TCP backend
│   └── keyplane-sentinel/     # sentinel-key (host-event) backend
├── src-tauri/                 # Tauri v2 shell: windows, commands/events, drivers
└── src/                       # React + TypeScript + Vite frontend (app + overlay)
```

### Protocol Backend family

All implement `keyplane_core::backend::ProtocolBackend`; the app's `Backend` enum
holds whichever is active.

| Backend | Source | Live transport | Confidence |
| --- | --- | --- | --- |
| Fake | scripted demo | none (in-process) | Authoritative |
| KeyPeek | VIA/Vial firmware | HID (`qmk-via-api`), hardware-gated | Authoritative |
| Kanata | Kanata remapper | TCP (`--port`), daemon-gated | Authoritative |
| Sentinel | Host Input Events | OS key capture (not yet wired), feedable | Inferred |

`keyplane-keypeek` vendors KeyPeek's QMK/VIA keycode-label tables, `LayoutKey`
model, and VIA/Vial HID protocol code (`src/vendor/`, GPL-attributed in
`NOTICE`). A bridge maps `LayoutKey` → Keyplane Semantic Actions, so the
numeric-VIA-keycode path uses KeyPeek's curated tables instead of the
hand-rolled token parser (which still serves QMK/ZMK string tokens from `.vil`).

`keyplane-core` is the testable heart and the contract everything else depends
on. Per ADR 0036 it owns source composition, Backend Health, Layer Stack
resolution, and Effective Action derivation. The Tauri layer adds windows, the
fake-backend driver loop, and the command/event boundary; the frontend only
renders snapshots and applies runtime events.

### keyplane-core module map

| Module | Responsibility | Key ADRs |
| --- | --- | --- |
| `ids` | Stable Element IDs | 0028 |
| `geometry` | Per-key coordinate geometry | 0028 |
| `action` | Raw + Semantic actions | 0005, 0030 |
| `legend` | Structured Display Legends, collapse | 0029 |
| `model::*` | Physical Layout, Keymap, Runtime State, Visual Style | 0014, 0032, 0011 |
| `provenance` / `precedence` | Source Provenance + per-field Source Precedence | 0005, 0018, 0019 |
| `health` | Capability Flags + typed Health States | 0033, 0023 |
| `resolve::*` | Semantic derivation + Effective Action resolution | 0030, 0031, 0032 |
| `snapshot` / `event` | Frontend DTOs: Keyboard Snapshot + Runtime Events | 0014, 0036 |
| `backend::*` | Protocol Backend trait + Fake Backend | 0015, 0033, 0003 |
| `compose` | State composition seam (the primary test seam) | 0036 |
| `import::*` | Importers + Import Candidate/Review | 0008, 0034, 0046 |
| `profile::*` | EDN Profile Codec (parse/save/migrate) | 0012, 0035, 0042 |

## Decisions made during implementation

- **Own a small EDN subset rather than depend on `edn-rs`.** ADR 0042 allowed
  `edn-rs`, but deterministic save formatting is a core requirement and belongs
  to the app. A focused reader/writer for the value kinds the schema uses
  (keywords, strings, ints, floats, bools, nil, vectors, maps) keeps formatting
  fully under our control and removes a dependency. Floats always serialize with
  a decimal point so geometry round-trips as `Float`, never `Int`.
- **Raw Action is the single source of truth on disk.** Semantic Actions are
  derived from Raw Actions on load, so the EDN profile stays minimal and
  round-trips stably. (ADR 0005, 0030.)
- **Source Precedence is currently code-owned** (`precedence.rs`, ADR 0018), not
  serialized; Source Provenance is stored per element rather than as a separate
  top-level EDN section. The other ADR 0035 sections are modeled in EDN.
- **Time and I/O live above `keyplane-core`.** Backends are pull-based
  (`poll()`); the driver decides cadence and stale detection. Core resolution is
  deterministic and clock-free, which keeps the test seam pure.

## Validation

- `cargo test --workspace` — 39 domain tests green; `cargo clippy` clean.
- `pnpm test` — frontend contract tests green, run against real serde output
  dumped by `cargo run -p keyplane-core --example dump_dtos`.
- `pnpm build` — both window bundles build.
- `cargo run -p keyplane` — the app launches, creates the App + Overlay
  windows, and runs the Fake Backend driver loop without panicking. (A runtime
  smoke test caught and fixed an invalid-icon startup crash.)

## Hardware/OS-gated / deferred

- Real KeyPeek/HID and Kanata TCP live validation need a supported device and a
  running Kanata daemon; the protocol code is in place and the pure adapters are
  tested, but the live transports can't run in CI.
- Sentinel keys: the inference logic is implemented and tested, but OS-level
  host-event capture and macOS accessibility/input-monitoring permission
  detection are not yet wired (a global input hook conflicts with Tauri's
  main-thread event loop on macOS and needs real permissions).
- ZMK live (needs the `zmk-studio-api` serial/BLE git dep) and ZMK `.keymap`
  file import are not implemented.
- Fade Visibility, monitor-name-based Display Targeting placement, and
  Positioning Mode drag/resize affordances are modeled but not finished.
- Signed builds, launch-at-login, and packaging are out of scope for the local
  MVP (ADR 0038, 0039).

Done since the first pass: User Overrides are applied in resolution
(`Profile::resolved_model`), Display Targeting position/size drives the overlay
window, and KeyPeek's protocol/keycode code is reused rather than clean-roomed.

## Status

See the PR description checklist. The PRD critical-path numbering (Further
Notes) maps onto the tasks tracked there.
