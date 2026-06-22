# Keyboard Overlay App

This context describes a cross-platform keyboard overlay app that renders real keyboard layouts and layer state from firmware, remapper, and input-event sources.

## Language

**Firmware State**:
The keyboard-side view of active layers, key positions, and layer behavior as reported by firmware-aware channels such as KeyPeek firmware modules, Vial/VIA, or ZMK Studio extensions.
_Avoid_: Device state, real state

**Host Input Event**:
An operating-system input event after firmware and remappers have resolved a physical action into a keypress, mouse action, or scroll event.
_Avoid_: Key event when referring to firmware position state

**Sentinel Key**:
A user-configured Host Input Event that the app treats as a Runtime State signal for layer changes when no authoritative source reports them.
_Avoid_: Trigger key when referring specifically to layer-state inference

**Physical Layout**:
The geometry and identity of physical keys, including position, size, rotation, and matrix or position identity when available.
_Avoid_: Layout when the meaning could be logical keymap

**Physical Key**:
An individual key in a Physical Layout, identified independently from its displayed label or assigned action.
_Avoid_: Keycap when referring to geometry or identity

**Matrix Position**:
The firmware or hardware row and column identity associated with a Physical Key when the source exposes matrix coordinates.
_Avoid_: Screen position

**Fallback Layout**:
A lower-fidelity Physical Layout inferred from a source that lacks full per-key coordinate geometry.
_Avoid_: Canonical layout

**Logical Keymap**:
The mapping from physical key positions to actions across layers.
_Avoid_: Layout

**Layer Stack**:
The ordered set of active layers used to resolve Effective Actions for the Active Profile.
_Avoid_: Current layer

**Layer Precedence**:
The ordering rule that determines which active layer supplies the first non-transparent action for a Physical Key.
_Avoid_: Layer order when referring only to display order

**Activation Kind**:
The known reason a layer is active, such as default, momentary, toggle, tap-hold, lock, remapper state, or unknown.
_Avoid_: Layer type when referring to runtime state

**Raw Action**:
The original source-specific action representation, such as a QMK keycode, Vial/VIA numeric code, ZMK behavior ID with parameters, Kanata action string, or host input event name.
_Avoid_: Key label

**Semantic Action**:
A normalized interpretation of a Raw Action used for visualization, explanation, layer hints, and confidence warnings, not for executing firmware behavior.
_Avoid_: Firmware behavior

**Effective Action**:
The action a user will get after resolving layer precedence, transparent entries, and inheritance for a Physical Key.
_Avoid_: Raw action

**Inherited Value**:
A rendered value supplied by a lower-precedence layer or source because the current layer entry does not define its own effective behavior.
_Avoid_: Copied value

**Display Legend**:
The structured human-facing label, icon, or multi-part legend rendered on a keycap.
_Avoid_: Keycode

**Legend Slot**:
A named part of a Display Legend, such as primary, shifted, hold-role, tap-role, layer hint, action hint, or icon.
_Avoid_: Label when the keycap position carries meaning

**Runtime State**:
The live values that change while the keyboard is being used, such as active layers, pressed physical keys, locked layers, and source health.
_Avoid_: Current layout

**Runtime Event**:
An incremental update to Runtime State emitted after a Keyboard Snapshot has been loaded.
_Avoid_: Host input event when the update can come from firmware or a remapper

**State Confidence**:
The app's assessment of how trustworthy a Runtime State value is based on source authority, startup context, event completeness, and known backend limitations.
_Avoid_: Health when referring to correctness rather than availability

**Backend Health**:
The availability and failure state of a Protocol Backend, including required permissions, connections, stale data, and recoverable or fatal backend errors.
_Avoid_: Toast or log entry

**Health State**:
A typed Backend Health value such as ok, permission missing, disconnected, stale, unsupported, parse error, or protocol error.
_Avoid_: Error string

**Best-Effort Preview**:
A mode that can render known layers or infer simple state but cannot guarantee live layer correctness because the source does not expose authoritative runtime state.
_Avoid_: Live sync

**Importer**:
A component that reads an external keyboard, style, or configuration source and produces an Import Candidate.
_Avoid_: Protocol Backend when the component only performs offline import

**Vial File Import**:
An Importer path that reads a Vial `.vil` JSON export and produces a Best-Effort Preview Import Candidate.
_Avoid_: Vial live sync

**Import Candidate**:
Normalized keyboard or Profile data produced by an Importer before the user accepts it into a Profile.
_Avoid_: Active profile

**Import Review**:
The workflow or product surface where an Import Candidate is previewed, compared, and checked for Source Conflicts before it becomes a Profile.
_Avoid_: Import wizard when no step-by-step flow is implied

**Profile**:
A saved app configuration that combines imported keyboard data, selected protocol backends, visual style, and user overrides for one keyboard or workflow.
_Avoid_: OverKeys config

**Active Profile**:
The Profile currently selected to drive the overlay, Protocol Backends, source precedence, and runtime state.
_Avoid_: Current keyboard when referring to app configuration rather than the physical device

**Profile Schema Version**:
The declared version of the Profile contract that determines how a saved profile should be interpreted and migrated.
_Avoid_: App version

**Profile Codec**:
The Rust boundary responsible for parsing, validating, writing, formatting, and migrating app-native EDN Profiles.
_Avoid_: EDN parser when referring to the full persistence contract

**Stable Element ID**:
A persistent identifier for a Profile element such as a key, layer, source, or visual style reference that must survive imports, edits, and migrations.
_Avoid_: Label or display name

**Source Provenance**:
The record of which source supplied a piece of keyboard or profile data and what raw representation was preserved from that source.
_Avoid_: Metadata when the source relationship matters

**Source Precedence**:
A Profile rule that chooses which source supplies the rendered value when multiple sources provide conflicting data for the same field.
_Avoid_: Merge order

**Source Conflict**:
A disagreement between two or more imported, backend-supplied, or user-authored values for the same Profile or Keyboard Model field.
_Avoid_: Error when the app can still choose a value through Source Precedence

**Source Inspector**:
A focused product surface for reviewing Source Provenance, Source Conflicts, Source Precedence, and the values selected for rendering.
_Avoid_: Profile editor when it does not author the full Profile

**User Override**:
A user-authored Profile value that intentionally replaces data supplied by an importer or Protocol Backend.
_Avoid_: Edit when the source priority matters

**Visual Style**:
The presentation choices applied after keyboard data is known, including keycap style, colors, legends, animation, placement, and overlay behavior.
_Avoid_: Theme when referring to the full rendering configuration

**Overlay Surface**:
A user-facing visualization shape, such as a full-keyboard layer overlay or a recent-input visualizer.
_Avoid_: Theme

**Overlay Window**:
The dedicated transparent app window that renders the active Overlay Surface over other applications.
_Avoid_: App window

**Display Targeting**:
The Profile-owned placement configuration for which display the Overlay Window uses and where it appears on that display.
_Avoid_: Visual style

**Global Display Fallback**:
The app-level Display Targeting used when the Active Profile does not define its own target.
_Avoid_: Default profile

**Overlay Visibility Policy**:
A Profile setting that determines when the Overlay Window is shown, hidden, pinned, manually toggled, or faded after activity.
_Avoid_: Animation style

**Pinned Visibility**:
An Overlay Visibility Policy where the Overlay Window remains visible until the user or app explicitly hides it.
_Avoid_: Always-on when the distinction from launch-at-startup matters

**Fade Visibility**:
An Overlay Visibility Policy where the Overlay Window appears in response to activity and then fades or hides after an inactivity interval.
_Avoid_: Animation when referring to visibility behavior

**Click-Through Mode**:
An Overlay Window interaction state where pointer input passes through to the application underneath the overlay.
_Avoid_: Disabled overlay

**Positioning Mode**:
An Overlay Window interaction state where click-through behavior is temporarily disabled so the user can move, resize, or configure the overlay placement.
_Avoid_: Edit mode when the user is only adjusting placement

**App Window**:
The normal interactive app window used for profiles, imports, settings, and source inspection.
_Avoid_: Overlay window

**Recent Input Visualizer**:
An Overlay Surface that renders a rolling history of Host Input Events rather than the full Physical Layout and active layer state of a keyboard.
_Avoid_: Keyboard overlay

**Style Variant**:
A reusable visual treatment applied to an Overlay Surface without changing the keyboard data, runtime state, or source integrations behind it.
_Avoid_: Mode when referring only to presentation

**Protocol Backend**:
A backend that discovers, imports, or streams keyboard information from a concrete source such as KeyPeek firmware modules, Vial/VIA, ZMK Studio, Kanata, sentinel keys, polling, or host input events.
_Avoid_: Driver when the component does not own a hardware driver

**Fake Backend**:
A deterministic Protocol Backend used for development, automated tests, and demos when no supported live hardware is available.
_Avoid_: Mock when referring to a supported app-facing backend implementation

**Composite Source**:
A configuration where multiple sources together provide one complete keyboard model, such as Kanata for runtime layer changes plus an OverKeys-style profile for keymap and layout data.
_Avoid_: Backend when one source alone is insufficient

**Keyboard Model**:
The normalized in-app representation that combines physical layout, logical keymap, runtime state, host input events, and visual style without treating any one source format as canonical.
_Avoid_: Layout model

**Keyboard Snapshot**:
A point-in-time view of the Keyboard Model that the frontend can render before applying subsequent Runtime State changes.
_Avoid_: Backend response when referring to keyboard meaning

**Capability Flag**:
A declared ability or limitation of a Protocol Backend, such as whether it can provide authoritative layer state, import keymap data, or only offer Best-Effort Preview behavior.
_Avoid_: Feature flag
