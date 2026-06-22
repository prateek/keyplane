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

The repo is intentionally docs-first. Do not assume a Tauri scaffold, KeyPeek fork, or Rust module layout exists until it has been created in this repo.

## Source Boundaries

- KeyPeek code may be reused under GPL-3.0 obligations.
- keyviz may inform keycap style import and rendering ideas under its license terms.
- OverKeys should be treated as design inspiration and a configuration-import target, not a code source.
- Do not commit private local keyboard exports such as personal `.vil` files unless they have been sanitized and intentionally added as fixtures.
