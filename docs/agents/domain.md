# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before Exploring, Read These

- `CONTEXT.md` at the repo root, or
- `CONTEXT-MAP.md` at the repo root if it exists. It points at one `CONTEXT.md` per context. Read each one relevant to the topic.
- `docs/adr/`. Read ADRs that touch the area you're about to work in. In multi-context repos, also check `src/<context>/docs/adr/` for context-scoped decisions.

If any of these files don't exist, proceed silently. Don't flag their absence; don't suggest creating them upfront. The `domain-modeling` skill, reached via `grill-with-docs` and `improve-codebase-architecture`, creates them lazily when terms or decisions actually get resolved.

## File Structure

Single-context repo, which is the current layout for this repo:

```text
/
|-- CONTEXT.md
|-- docs/adr/
|   |-- 0001-example-decision.md
|   `-- 0002-example-decision.md
`-- src/
```

Multi-context repo, indicated by `CONTEXT-MAP.md` at the root:

```text
/
|-- CONTEXT-MAP.md
|-- docs/adr/
`-- src/
    |-- ordering/
    |   |-- CONTEXT.md
    |   `-- docs/adr/
    `-- billing/
        |-- CONTEXT.md
        `-- docs/adr/
```

## Use the Glossary's Vocabulary

When your output names a domain concept, such as in an issue title, refactor proposal, hypothesis, or test name, use the term as defined in `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal: either you're inventing language the project doesn't use, or there's a real gap to note for `domain-modeling`.

## Flag ADR Conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding it.
