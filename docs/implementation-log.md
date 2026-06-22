# Implementation Log

Execution notes for building Keyplane against [the PRD](prd/keyplane-prd.md). The
PRD stays stable; progress and decisions live here.

## Architecture

```
keyplane/
├── Cargo.toml                 # workspace
├── crates/keyplane-core/      # domain core (no Tauri/HID/UI deps)
├── src-tauri/                 # Tauri v2 shell: windows, commands/events, drivers
└── src/                       # React + TypeScript + Vite frontend (app + overlay)
```

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

## Status

See the PR description for the live checklist. The PRD critical-path numbering
(Further Notes) maps onto the tasks tracked there.
