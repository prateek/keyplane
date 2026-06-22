# PRD: Keyplane

## Problem Statement

Power users with layered keyboards need a reliable way to see what their keyboard will do right now. Existing tools cover parts of the problem:

- KeyPeek can read real keyboard geometry and live layer state, but it requires firmware support and ships an egui UI rather than a cross-platform overlay shell.
- OverKeys has useful overlay ideas, Kanata integration patterns, and user-facing configuration concepts, but its internal model is row-array based and Flutter-specific.
- keyviz has strong keycap styling and host-input visualization, but it is centered on recent input events rather than physical keyboard layer state.
- Stock Vial/VIA/ZMK can expose useful layout and keymap data, but they generally do not expose authoritative live layer changes without extra firmware support.

The user wants a new cross-platform app that starts from KeyPeek's Rust protocol and domain code, replaces the UI with Tauri and a web frontend, supports nice full-keyboard overlays by default, and can grow toward OverKeys-style compatibility without copying OverKeys code.

The first useful version must help with two workflows:

- A KeyPeek-backed keyboard that can stream live layer state.
- A NocFree/Vial-style workflow that can import keyboard data and render it as Best-Effort Preview when live layer state is unavailable.

## Solution

Build a GPL-3.0 Tauri desktop app that forks and reuses KeyPeek's Rust protocol/domain code, then layers a web frontend and dedicated transparent Overlay Window on top.

The app centers on a normalized Keyboard Model. Rust owns source composition, Protocol Backend health, Layer Stack resolution, Effective Action derivation, import processing, profile persistence, and Runtime Events. The web frontend consumes a Keyboard Snapshot plus Runtime Events and focuses on rendering, settings, Import Review, Source Inspector, and overlay controls.

The first product surface is a full-keyboard layer overlay. It renders a Physical Layout with structured Display Legends, resolves transparent keys into Effective Actions through the active Layer Stack, shows lightweight State Confidence or Backend Health status, and stays out of the user's way through Click-Through Mode.

Profiles are app-native EDN files. They are public, hand-editable, versioned, deterministic on save, and preserve Source Provenance. External formats enter through Importers. KeyPeek, Vial/VIA, ZMK, OverKeys, keyviz style JSON, Kanata companion data, and sentinel-key configuration can feed the model, but none of those formats becomes canonical.

## User Stories

1. As a keyboard user, I want to see my full keyboard layout on screen, so that I can understand what each Physical Key does on the active layer.
2. As a layered-keyboard user, I want the overlay to update when my layer changes, so that I do not have to remember hidden layer state.
3. As a KeyPeek firmware user, I want the app to connect to my supported keyboard, so that I can get authoritative live Layer Stack updates.
4. As a NocFree/Vial user, I want to import my keyboard data, so that I can render my board without rewriting my keymap.
5. As a stock Vial/VIA user, I want the app to label my imported layout as Best-Effort Preview, so that I know the overlay may not reflect live layer changes.
6. As a Kanata user, I want Kanata treated as a Protocol Backend, so that runtime layer changes can drive the same overlay path as firmware backends.
7. As a Kanata user, I want the MVP to support Kanata through a companion OverKeys-style profile, so that the app has the keymap and layout data needed to render layers.
8. As an OverKeys user, I want to import existing layouts, aliases, triggers, styles, and Kanata settings, so that migration starts from my current configuration.
9. As an OverKeys user, I want import-only compatibility at first, so that the new app can use a richer model without corrupting old OverKeys configs.
10. As a keyviz user, I want to import keyviz style JSON, so that I can reuse keycap visual preferences where they fit the full-keyboard overlay.
11. As a keyboard user, I want each keycap to support structured Legend Slots, so that primary, shifted, hold, tap, layer, action, and icon meanings can render cleanly.
12. As a user of compact visual styles, I want the renderer to collapse structured legends into simple labels, so that the overlay can stay readable.
13. As a user of rich keycap styles, I want the renderer to preserve structured meaning, so that the overlay can show more than a flat key label.
14. As a firmware-aware user, I want Raw Actions preserved, so that exact source data is available for debugging and future importer improvements.
15. As a normal user, I want Semantic Actions derived from Raw Actions, so that the overlay can explain key, modifier, layer, tap-hold, macro, transparent, mouse, none, and unknown actions.
16. As a user with transparent layer entries, I want the overlay to show the Effective Action I will get, so that transparent keys are useful while typing.
17. As a user inspecting my profile, I want transparent entries preserved, so that I can see where an Inherited Value came from.
18. As a user with momentary and toggled layers, I want Runtime State represented as a Layer Stack, so that Effective Actions resolve through the same precedence I expect from firmware.
19. As a user with a default layer and temporary layer, I want the overlay to highlight the top active layer, so that I can see the current layer focus.
20. As a user with uncertain layer state, I want State Confidence shown, so that I can tell authoritative live state from inferred state.
21. As a user without firmware support, I want sentinel keys as a lower-confidence Protocol Backend, so that I can still map host input events to layer changes.
22. As a user who starts the app mid-layer, I want sentinel-key state to show lower confidence, so that the app does not pretend it knows more than it does.
23. As a user, I want Backend Health surfaced in the UI, so that permission and connection problems do not disappear into logs.
24. As a macOS user, I want accessibility and input-monitoring permission problems to appear as persistent Backend Health, so that I know what to grant.
25. As a user with a disconnected keyboard, I want the app to show a disconnected Health State, so that I know why the overlay stopped updating.
26. As a user with stale polling data, I want the app to show stale Backend Health, so that I can trust the overlay appropriately.
27. As a user importing data, I want an Import Review before my Profile changes, so that I can inspect what the Importer found.
28. As a user importing multiple sources, I want Source Conflicts shown outside the overlay, so that the overlay stays clean while conflicts remain inspectable.
29. As a user resolving conflicts, I want to see Source Provenance for each candidate value, so that I can choose the source I trust.
30. As a user, I want to promote a selected value to a User Override, so that future imports do not silently replace my choice.
31. As a user, I want Source Precedence to be explicit per field, so that runtime state, layout, keymap, style, and overrides resolve predictably.
32. As a user, I want User Overrides to win over imported data, so that manual corrections stay stable.
33. As a user, I want KeyPeek or firmware-aware sources to win for Runtime State, so that authoritative live state drives the overlay.
34. As a user, I want Vial/VIA/ZMK/KeyPeek imports to win for Physical Layout and Logical Keymap, so that the app uses real geometry when available.
35. As a user, I want keyviz imports to affect only Visual Style fields, so that style data does not change keyboard meaning.
36. As a user, I want app-native EDN Profiles, so that profiles are readable and editable outside the app.
37. As a user, I want Profile Schema Version recorded, so that migrations can update old profiles safely.
38. As a user, I want Stable Element IDs for keys, layers, sources, and style references, so that imports and migrations do not break references.
39. As a user, I want deterministic EDN formatting on save, so that profile diffs are readable.
40. As a user, I want one Active Profile in the MVP, so that profile selection and live overlay behavior stay predictable.
41. As a future multi-keyboard user, I want the model to avoid blocking multiple active devices later, so that the MVP does not close that door.
42. As a user, I want a dedicated Overlay Window, so that profile/settings UI and overlay rendering have separate lifecycles.
43. As a user, I want the Overlay Window to be transparent, frameless, and always on top, so that it behaves like an overlay rather than a normal app panel.
44. As a user, I want Click-Through Mode by default, so that the overlay does not steal focus or pointer input while I type.
45. As a user, I want Positioning Mode, so that I can drag, resize, and place the overlay when needed.
46. As a user, I want Pinned Visibility as the MVP default, so that layer correctness is easy to validate.
47. As a user, I want Overlay Visibility Policy to allow manual toggle and Fade Visibility later, so that the app can support different workflows.
48. As a user with a desk setup and laptop setup, I want Display Targeting to be Profile-owned, so that each workflow can keep its monitor, placement, size, and opacity.
49. As a user without profile-specific Display Targeting, I want a Global Display Fallback, so that the overlay still appears in a useful place.
50. As a user, I want one keyboard overlay mode with Style Variants, so that the product stays focused while still allowing different looks.
51. As a user, I want Physical Layout based on per-key coordinate geometry, so that split, angled, and non-rectangular boards render correctly.
52. As an OverKeys importer user, I want row arrays imported as a Fallback Layout, so that older configs still produce something renderable.
53. As a developer, I want Protocol Backends to expose typed Capability Flags, so that frontend and profile logic can reason about backend abilities.
54. As a developer, I want typed Health States, so that permission missing, disconnected, stale, unsupported, parse error, and protocol error are handled consistently.
55. As a frontend developer, I want to receive Keyboard Snapshots and Runtime Events, so that the web UI does not need to know HID, Vial, ZMK, Kanata TCP, or KeyPeek protocol details.
56. As a backend developer, I want Rust to own Effective Action resolution, so that rendering behavior stays consistent across overlay, Source Inspector, and tests.
57. As a project maintainer, I want GPL obligations and attribution handled explicitly, so that reuse from KeyPeek and keyviz is legally clean.
58. As a project maintainer, I want a new product identity early, so that the fork does not confuse users with KeyPeek or OverKeys.
59. As a release user, I want launch-at-login designed into the app, so that the overlay can run as a daily-use tool.
60. As a release user, I want minimal permissions and visible permission health, so that the app asks for only what it needs and explains failures.

## Implementation Decisions

- License and product identity: The app is GPL-3.0 because it reuses KeyPeek and may reuse keyviz concepts or code. Use `Keyplane` as the initial product name and `keyplane` as the repo, crate, and package slug. This is an implementation default, not trademark clearance. Keep attribution explicit. Treat OverKeys as design inspiration and an import target, not a code source.

- Application architecture: Reuse and reshape KeyPeek Rust protocol/domain code. Replace KeyPeek's egui UI with a Tauri v2 shell and a React, TypeScript, and Vite web frontend.

- Rust/frontend boundary: Protocol Backends live as Rust-side traits and services. Tauri commands/events are the frontend boundary. The frontend receives serialized app-domain DTOs: Keyboard Snapshots, Runtime Events, Backend Health, Capability Flags, Import Candidates, and Import Review data.

- State ownership: Rust owns source composition, Backend Health, Layer Stack resolution, Source Precedence, Semantic Action derivation, Effective Action resolution, and EDN Profile persistence. The web frontend owns rendering, settings UI, Import Review, Source Inspector, overlay controls, and visual styling.

- Primary source of truth: Firmware State and authoritative remapper state are primary. Host Input Events are secondary and can support sentinel keys or future recent-input visualization.

- First vertical slice: Start with a fake Protocol Backend, then KeyPeek's firmware-module live-state path. The fake backend must produce a Keyboard Snapshot, stream Layer Stack Runtime Events, and render those changes in a full-keyboard Overlay Window without real hardware. The first real hardware validation should use whichever KeyPeek-supported QMK/Vial/ZMK device is identified or flashed first; do not block CI or core development on a named keyboard.

- First import-and-preview path: Support a NocFree/Vial `.vil` JSON file import as Best-Effort Preview. Stock Vial/VIA file data can provide useful layout and keymap information, but the app must not label it as authoritative live layer sync. Live Vial device import can come later.

- Keyboard Model: The normalized model combines Physical Layout, Logical Keymap, Runtime State, Host Input Events, Visual Style, Source Provenance, and Source Precedence without making any external source format canonical.

- Physical Layout: Use per-key coordinate geometry. Each Physical Key has a Stable Element ID, optional Matrix Position, coordinate geometry, optional rotation, and Source Provenance. OverKeys row arrays import only as a Fallback Layout.

- Logical Keymap: Store layer data by Stable Element IDs rather than row positions. Preserve Raw Actions and derive Semantic Actions for visualization, explanation, Display Legends, layer hints, and State Confidence warnings.

- Display Legends: Model legends as structured Legend Slots instead of a single string. Supported slots include primary, shifted/secondary, tap-role, hold-role, layer hint, action hint, and optional icon. Style Variants may collapse slots into a simple label.

- Effective Actions: Preserve transparent Raw Actions in the model. For rendering, resolve Effective Actions through the Layer Stack and show a subtle inherited indicator when a displayed value comes from an Inherited Value.

- Runtime State: Represent active layers as an ordered Layer Stack rather than a single current layer. Runtime State carries active layer IDs, Layer Precedence, Activation Kind when known, and State Confidence.

- Protocol Backends: The first backend family is KeyPeek-derived firmware/HID support. Kanata is also a Protocol Backend, but the MVP requires an OverKeys-compatible companion profile so Kanata has renderable layout/keymap data. Sentinel keys are a separate lower-confidence Protocol Backend based on Host Input Events. Polling/manual modes can exist as Best-Effort Preview.

- Backend capabilities: Each Protocol Backend declares typed Capability Flags, including whether it can discover devices, import geometry, import keymaps, stream live Layer Stack changes, stream pressed keys, poll, or only preview.

- Backend health: Permissions, connection failures, stale sources, unsupported devices, parse errors, and protocol errors are Runtime State through Backend Health. They must be visible in the overlay/app shell and Source Inspector.

- Import pipeline: Importers produce Import Candidates with normalized data and Source Provenance. They never mutate the Active Profile directly. Import Review previews, diffs, and Source Conflicts before committing an Import Candidate as a new Profile or an update.

- Source Precedence: User Overrides always win. KeyPeek/firmware-aware sources win for Runtime State. Imported Vial/VIA/ZMK/KeyPeek data wins for Physical Layout and Logical Keymap. Kanata wins only for runtime layer state unless paired with companion profile data. keyviz style JSON affects only Visual Style fields. Losing values remain inspectable through Source Provenance.

- Source Conflicts: The overlay renders selected values only. Profile, edit, import, and Source Inspector surfaces expose conflicts with Source Provenance, selected winner, and an action to promote a value to User Override.

- Profile format: Profiles use app-native EDN v1 as the canonical saved format. Profiles are public, hand-editable, versioned, deterministic on save, and migratable. Use namespaced EDN keywords for sections, fields, and enum-like values; strings for Stable Element IDs and user-authored identifiers; vectors for ordered data; and maps for keyed collections. Top-level sections include `:schema/version`, `:profile/id`, `:sources`, `:keyboard/physical-layout`, `:keyboard/keymap`, `:runtime/backends`, `:visual/style`, `:overlay/window`, `:source/precedence`, `:user/overrides`, and `:source/provenance`.

- Profile codec: Hide the EDN parser, writer, and migration machinery behind a small Rust Profile Codec boundary. The first implementation may use `edn-rs` plus serde for parsing and serialization, but deterministic saves require canonical ordering and formatting owned by the app. Migrations are pure `vN` to `vN+1` transforms over parsed profile data or a validated EDN AST.

- Profile scope: The MVP supports exactly one Active Profile at a time. The model should not block multiple active keyboards later.

- Profile editing: The MVP supports import plus hand-editable EDN. A focused Source Inspector is in scope. A full visual Profile editor is deferred.

- keyviz compatibility: MVP compatibility means importing keyviz style JSON into the Visual Style model. The full-keyboard overlay is not based on keyviz's recent-input component structure.

- Overlay surface: The first and only MVP Overlay Surface is the full-keyboard layer overlay. Recent-input visualization is deferred.

- Overlay window: Implement the overlay as a dedicated transparent, frameless, always-on-top Tauri Overlay Window separate from the App Window. Rust creates and owns this window through Tauri v2 `WebviewWindowBuilder`/`Window` APIs. Initial overlay options should include label `overlay`, transparent, decorations disabled, always on top, skip taskbar, non-focusable, hidden until the first Keyboard Snapshot, and not resizable except in Positioning Mode.

- Overlay interaction: The Overlay Window is click-through by default through `Window::set_ignore_cursor_events(true)` where the platform supports it. Positioning Mode calls `set_ignore_cursor_events(false)` and exposes drag/resize affordances through Tauri window APIs such as `start_dragging` and resize-dragging. If transparency, click-through, always-on-top, or all-workspaces behavior is unsupported on a platform, report it as typed app/window capability health instead of silently degrading.

- Overlay visibility: Pinned Visibility is the MVP default. The Profile model should allow Overlay Visibility Policy to support manual toggle and Fade Visibility later.

- Display Targeting: Display Targeting is Profile-owned with a Global Display Fallback. The MVP may store only the Active Profile's target, but the model treats targeting as part of Profile behavior.

- Packaging and permissions: MVP can start as dev/local desktop builds. Design for signed builds, launch-at-login, accessibility/input-monitoring prompts, minimal permissions, and visible permission health from day one. On macOS, transparent Tauri windows require the `macos-private-api` capability and are not App Store compatible; that is acceptable for the local/dev MVP but should remain visible in release planning.

## Testing Decisions

- Good tests assert external behavior: imported data, rendered model shape, Runtime Event handling, Backend Health behavior, Source Conflict results, EDN round trips, and overlay-visible outcomes. Tests should avoid asserting private helper structure.

- Highest test seam: the primary seam is the Rust-owned state composition boundary that turns sources, profiles, and backend events into a Keyboard Snapshot plus Runtime Events. This seam should be tested before lower-level UI tests because it validates the contract the whole app depends on.

- Backend tests: Use fake Protocol Backends to verify Capability Flags, Health States, Runtime Events, disconnect/stale behavior, and layer-stack updates. Do not require real HID hardware for core tests.

- Importer tests: Use fixture exports for KeyPeek-like data, Vial/VIA data, ZMK-like data, OverKeys-style configs, keyviz style JSON, and Kanata companion data. Assert that each Importer produces an Import Candidate with Source Provenance and no Active Profile mutation.

- Profile tests: Verify EDN parse/save, deterministic formatting, Profile Schema Version handling, Stable Element IDs, migration hooks, Source Precedence, User Overrides, and preservation of losing Source Conflict values.

- Layer resolution tests: Verify Layer Stack order, Layer Precedence, transparent key inheritance, Effective Action resolution, Activation Kind propagation, and State Confidence handling.

- Legend tests: Verify Raw Action preservation, Semantic Action derivation, structured Legend Slots, collapsed labels for minimal Style Variants, inherited indicators, and unknown-action fallback.

- Backend Health tests: Verify permission-missing, disconnected, stale, unsupported, parse-error, and protocol-error states flow into Keyboard Snapshots/Runtime Events and can be rendered by UI state.

- Frontend contract tests: Use serialized Keyboard Snapshots and Runtime Events as fixtures. Verify overlay rendering receives the selected values, source/confidence indicators, active layer highlights, and Backend Health status without directly depending on protocol APIs.

- Overlay behavior tests: Where platform automation allows, verify the Overlay Window can render transparent, pinned, click-through, and positionable states. Unit/component tests can cover state transitions even when true OS click-through behavior requires manual or platform-specific validation.

- Import Review tests: Verify previews, diffs, Source Conflicts, selected winners, and User Override promotion behavior.

- MVP acceptance tests: A passing MVP demo should show fake-backend live layer changes in the Overlay Window, one real KeyPeek-backed live layer change when supported hardware is available, one NocFree/Vial `.vil` Best-Effort Preview import, EDN save/load, visible Backend Health, and Positioning Mode.

- Prior art: Existing OverKeys docs and tests can inform expected configuration behavior and user-facing terminology. KeyPeek's protocol/domain code should be the starting point for backend tests. The new app should add its own Rust/domain tests and web frontend tests rather than trying to reuse Flutter test structure.

## Out of Scope

- Arbitrary Kanata `.kbd` parsing.
- Full visual Profile editor.
- Multiple simultaneously active keyboard profiles.
- Firmware flashing assistant.
- Write-back to Vial, ZMK, QMK, OverKeys, or Kanata configs.
- Recent-input keyviz clone.
- Exact simulation of firmware behavior.
- Guaranteeing live layer sync for stock Vial/VIA/ZMK without firmware support.
- Shipping polished signed installers before the MVP works locally.
- Multi-device routing, per-app overlay profiles, cloud sync, profile sharing marketplace, and remote backup.

## Further Notes

Critical path for the implementation agent:

1. Fork/reuse KeyPeek Rust protocol/domain code and introduce a Tauri shell with a minimal React/TypeScript/Vite frontend.
2. Define the app-domain DTOs for Keyboard Snapshot, Runtime Event, Capability Flag, Backend Health, Import Candidate, and Profile.
3. Build a Fake Backend that produces a Keyboard Snapshot and renders it in a dedicated Overlay Window.
4. Add EDN Profile parse/save with deterministic formatting and schema versioning.
5. Implement KeyPeek-backed live Layer Stack updates through a fake backend first, then the real KeyPeek-derived backend.
6. Implement Effective Action and Display Legend resolution for transparent keys and structured Legend Slots.
7. Implement NocFree/Vial `.vil` import as Best-Effort Preview through the Import Candidate and Import Review path.
8. Add Backend Health and permission/connection status to the App Window and overlay status area.
9. Add Source Inspector views for provenance, selected winners, conflicts, and User Override promotion.
10. Add Positioning Mode and Profile-owned Display Targeting.

Execution updates should live in a separate implementation log, not in this PRD. The PRD should stay stable unless scope or accepted product behavior changes.

Resolved implementation defaults:

- Product name: `Keyplane`; slug: `keyplane`.
- Frontend stack: React, TypeScript, Vite, Tauri v2.
- Profile format: app-native EDN v1 with namespaced keywords, string Stable Element IDs, ordered vectors, deterministic writer, and pure schema migrations behind a Rust Profile Codec.
- Overlay implementation: Rust-owned Tauri Overlay Window using builder/window APIs; click-through via cursor-event ignoring where supported; Positioning Mode disables click-through and uses drag/resize APIs.
- First live validation: fake backend in automated/dev flows, then whichever KeyPeek-supported real keyboard is identified or flashed first.
- First NocFree path: `.vil` JSON file import only, producing Best-Effort Preview and preserving raw Vial/VIA data as Source Provenance.
