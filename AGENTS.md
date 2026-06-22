# Agent Instructions

## Domain Docs

This repo uses a single-context domain-doc layout. Before changing product behavior, model code, profile formats, backends, importers, or UI concepts, read:

- `CONTEXT.md`
- relevant ADRs in `docs/adr/`
- `docs/prd/keyplane-prd.md` for product scope

See `docs/agents/domain.md` for the domain-doc workflow.

## Agent skills

### Issue tracker

Issues and PRDs are tracked in GitHub Issues for `prateek/keyplane`; external PRs are not a triage request surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage uses the default label vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

This repo uses a single-context domain-doc layout. See `docs/agents/domain.md`.

## Current State

The MVP is implemented. Layout:

- `crates/keyplane-core/` — domain core (model, resolution, EDN Profile Codec, importers), no Tauri/HID/UI deps, fully unit-tested.
- `crates/keyplane-keypeek/` — KeyPeek-derived protocol + keycode-label code, vendored in `src/vendor/` (GPL, attributed in `NOTICE`). The live KeyPeek/VIA/Vial HID backend lives here.
- `crates/keyplane-kanata/` — Kanata TCP backend. `crates/keyplane-sentinel/` — sentinel-key (Host Input Event) backend.
- `src-tauri/` — Tauri v2 shell: App + Overlay windows, command/event boundary, backend driver.
- `src/` — React/TypeScript/Vite frontend (overlay + App Window).

The Fake Backend is the automated/dev source (ADR 0045). Live HID (KeyPeek) and Kanata TCP transports are gated on real hardware/daemons; OS-level host-event capture for sentinel keys is not yet wired. See `docs/implementation-log.md` for architecture, the backend family, and the deferred list.

## Source Boundaries

- KeyPeek code may be reused under GPL-3.0 obligations.
- keyviz may inform keycap style import and rendering ideas under its license terms.
- OverKeys should be treated as design inspiration and a configuration-import target, not a code source.
- Do not commit private local keyboard exports such as personal `.vil` files unless they have been sanitized and intentionally added as fixtures.
