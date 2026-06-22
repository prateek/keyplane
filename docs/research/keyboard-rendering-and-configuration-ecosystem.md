# Keyboard Rendering and Configuration Ecosystem

Date: 2026-06-22

## Executive Summary

Keyboard tools answer different questions. A physical layout answers where keys are. A matrix map answers how switches connect to firmware. A keymap answers what each position does on each layer. A behavior language answers what happens when a key is tapped, held, combined, or timed. A runtime editor answers how changes get written to a device. A renderer answers how all of this becomes something a person can inspect.

Most confusion in keyboard tooling comes from treating those questions as one problem. KLE and KLE NG are visual geometry editors. QMK `keyboard.json` and `info.json` describe firmware-facing board metadata and physical positions. VIA and Vial add user-facing runtime configuration on top of QMK-family firmware and use KLE-like keyboard definitions. ZMK uses devicetree for keymaps and physical layout data, with ZMK Studio adding runtime editing. Kanata and KMonad remap keys in user space, so they define behavior well but usually do not define real board geometry. keymap-drawer, KeyPeek, keyviz, Monkeytype, and OverKeys sit closer to rendering, but they render different things: static keymap diagrams, firmware-aware live overlays, OS input events, typing-practice keymaps, and app-local practice overlays. [1][2][3][4][5][6][7][8][9][10][11][33][34][35]

The three named keyboards show why a renderer or configurator cannot stop at one ecosystem. NocFree Lite is configured through Vial. MoErgo Go60 uses the MoErgo Layout Editor for most users and ZMK config repositories for source-based workflows. Kinesis Advantage360 Pro uses Kinesis Clique, a ZMK Studio-based runtime editor, or the Adv360-Pro-ZMK repository for GitHub-based firmware builds. [12][13][14][15][16][17]

The practical model is simple: keep geometry, logical keymaps, runtime state, input events, and visual style separate. A serious keyboard renderer should ingest geometry from KLE, QMK, VIA, Vial, ZMK physical layouts, or Ergogen output; ingest behavior from QMK, ZMK, Kanata, KMonad, VIA/Vial exports, or keymap-drawer YAML; read live layer state from firmware-aware tools when available; and apply display styling from tools such as keyviz, Monkeytype, or OverKeys only at the end.

## Introduction

This report explains how keyboard tools render and configure keyboards from first principles. It covers the formats and tools in the current hobbyist and ergonomic keyboard ecosystem: OverKeys, Monkeytype theme and keymap style references, VIA, the `the-via/keyboards` repository, QMK, Kanata, Vial, ZMK, ZMK Studio, KLE, KLE NG, Ergogen, keymap-drawer, KeyPeek, keyviz, Keymap Editor, KMonad, NocFree Lite, MoErgo Go60, and Kinesis Advantage360 Pro.

The report uses "spec" broadly. Some entries are formal or semi-formal file formats, such as QMK `info.json`, VIA definition JSON, KLE JSON, Vial JSON, ZMK `.keymap`, Kanata `.kbd`, and KMonad `.kbd`. Other entries are tools or workflows over those formats, such as MoErgo Layout Editor, Kinesis Clique, ZMK Studio, Keymap Editor, Monkeytype, OverKeys, and keymap-drawer.

The first-principles distinction matters. A keyboard has at least seven layers of representation:

1. Electrical scan matrix: rows, columns, direct pins, diodes, and scan order.
2. Physical geometry: key x/y position, width, height, rotation, handedness, encoders, and layout options.
3. Logical keymap: position-to-action mappings for one or more layers.
4. Behavior semantics: tap-hold, mod-tap, combos, macros, one-shot keys, sticky keys, Unicode, pointer actions, and timing.
5. Runtime storage and protocol: EEPROM, flash, BLE, USB HID, WebHID, WebSerial, or firmware rebuilds.
6. Host input events: the keypresses and mouse actions that the operating system sees after firmware and remappers have done their work.
7. Rendering and style: labels, colors, keycap shape, themes, legends, active-layer display, and pressed-key animation.

No single ecosystem artifact covers all six layers cleanly. QMK can cover the first four, but QMK Configurator JSON is a narrower keymap format. VIA can identify a device and write dynamic keymaps, but its definition file is not enough to build firmware from scratch. KLE can draw a board, but it knows nothing about firmware behaviors unless another tool encodes metadata into legends. ZMK `.keymap` files can express rich behavior, but physical positions may live in separate devicetree files. [1][2][3][4][5]

## Main Analysis

### Finding 1: Physical geometry and logical behavior are different specs

The most useful mental model is to separate position from action. Position means a key exists at x/y coordinates, has a size, belongs to a matrix row and column, and may be rotated. Action means the firmware or remapper sends `A`, holds `Control`, switches to layer 2, runs a macro, or behaves differently on tap and hold.

The same physical key can have many logical actions. A 1u thumb key at position 42 might send Space on the base layer, momentarily activate a navigation layer when held, emit Escape on a tap dance, and do nothing on a game layer. A renderer needs both the physical key and each layer's action.

The inverse is also true: the same logical action can appear on many shapes. `KC_ESC`, `&kp ESC`, `esc`, and `Esc` may all render as `Esc`, but they come from different dialects. A renderer should keep the raw action and the display legend.

A neutral internal shape often looks like this:

```json
{
  "physicalKey": {
    "id": "k00",
    "matrix": [0, 0],
    "x": 0,
    "y": 0,
    "w": 1,
    "h": 1,
    "r": 0,
    "label": "Esc"
  },
  "logicalAction": {
    "layer": "base",
    "raw": "KC_ESC",
    "dialect": "qmk",
    "display": "Esc"
  }
}
```

Almost every tool in this report can be placed by asking two questions: does it provide geometry, and does it provide behavior?

| Tool or format | Geometry | Behavior | Runtime write | Rendering |
| --- | --- | --- | --- | --- |
| KLE / KLE NG | Yes | No | No | Yes |
| QMK `keyboard.json` / `info.json` | Yes | Some metadata | No | Used by QMK tools |
| QMK `keymap.c` / keymap JSON | No, references layout | Yes | Firmware build | No |
| VIA definition JSON | Yes | UI metadata | Paired with VIA firmware | Used by VIA |
| Vial `vial.json` | Yes | UI metadata | Paired with Vial firmware | Used by Vial |
| ZMK `.keymap` | No or partial | Yes | Firmware build; Studio can override | No |
| ZMK physical layout | Yes | No | Enables Studio | Used by Studio |
| Kanata `.kbd` | Positional list, not geometry | Yes | User-space process | No |
| KMonad `.kbd` | Positional list, not geometry | Yes | User-space process | No |
| Ergogen YAML | Parametric geometry | Board design metadata | Generates artifacts | Generates previews/outputs |
| keymap-drawer YAML | Yes or references geometry | Yes for diagrams | No | SVG |
| KeyPeek | Device-derived for Vial/ZMK; QMK needs exported info | Reads live layers/keymap from device paths | No, firmware module required | Live keyboard overlay |
| keyviz | No | OS input events only | No | Keystroke and mouse-action overlay |
| Monkeytype | Preset visual keymap styles | Practice/layout display | No | Yes |
| OverKeys | App-local row layouts | Display layers/triggers | Kanata live state only | Yes |

### Finding 2: KLE and KLE NG are the common visual geometry layer

Keyboard Layout Editor serializes a keyboard as a JSON array of rows. Rows contain key labels and property objects. Position is implicit: each row starts at x = 0, rows increment y, and x advances by the prior key's width. Per-key properties such as `x`, `y`, `w`, `h`, `x2`, `y2`, `w2`, and `h2` modify the next key. Persistent properties such as `c`, `t`, `a`, `f`, and `p` affect following keys. [18]

Minimal KLE example:

```json
[
  { "name": "Tiny 2x2" },
  ["Esc", "Q"],
  [{ "w": 1.5 }, "Tab", "A"]
]
```

This means:

- Row 1 has `Esc` at x=0 and `Q` at x=1.
- Row 2 starts at y=1.
- `Tab` is 1.5u wide.
- `A` starts after the 1.5u key.

KLE is excellent for keycap geometry and visual layout editing. It is weak as a firmware spec because it does not natively define matrix coordinates, key behaviors, layers, or device identity. VIA and Vial solve part of that by encoding matrix coordinates and layout options into KLE legends, but that is a convention layered on top of KLE, not KLE's core data model. [2][3][18]

KLE NG, the tool at `editor.keyboard-tools.xyz`, is a modern KLE-compatible editor. Its repository describes it as a reimplementation of Keyboard Layout Editor that keeps compatibility with existing layouts and adds plate and PCB generators for DIY keyboard work. That makes KLE NG important to renderers because it keeps KLE JSON relevant while adding modern editing workflows. [19]

KLE and KLE NG provide:

- Visual key positions.
- Key sizes, gaps, and some odd-shaped key support.
- Labels and colors.
- A practical copy/paste format for keyboard layout sketches.

They do not provide:

- Firmware behavior.
- Runtime remapping.
- USB device identity.
- Reliable matrix data unless a downstream convention adds it.

### Finding 3: QMK separates board metadata from keymaps

QMK has two major kinds of configuration that renderers care about. Board metadata lives in `keyboard.json` or `info.json`. User behavior lives in `keymap.c` or QMK Configurator JSON. QMK documentation says `info.json` is used by the QMK API and contains the data QMK Configurator needs to display a keyboard. The layouts section contains physical key objects with matrix positions, x/y coordinates, width, height, labels, rotation fields, handedness, and encoders. [1]

Minimal QMK geometry example:

```json
{
  "keyboard_name": "TinyPad",
  "manufacturer": "Example",
  "maintainer": "example",
  "usb": {
    "vid": "0xFEED",
    "pid": "0x0001"
  },
  "matrix_pins": {
    "rows": ["B0"],
    "cols": ["B1", "B2"]
  },
  "diode_direction": "COL2ROW",
  "layouts": {
    "LAYOUT": {
      "layout": [
        { "label": "Esc", "matrix": [0, 0], "x": 0, "y": 0 },
        { "label": "Q", "matrix": [0, 1], "x": 1, "y": 0, "w": 1.25 }
      ]
    }
  }
}
```

This file answers "what is the board?" and "where are the keys?" It does not necessarily answer "what did this user map today?"

QMK keymaps answer the behavior question. In C, QMK stores keymaps as arrays of layers and keycodes, usually wrapped in a `LAYOUT()` macro. QMK documentation describes the outer array as layers and the inner layer array as keys; QMK supports up to 32 layers, with higher active layers taking precedence over lower ones. [20]

Minimal QMK `keymap.c` example:

```c
enum layers {
    _BASE,
    _FN
};

const uint16_t PROGMEM keymaps[][MATRIX_ROWS][MATRIX_COLS] = {
    [_BASE] = LAYOUT(
        KC_ESC, KC_Q
    ),
    [_FN] = LAYOUT(
        KC_GRV, KC_1
    )
};
```

QMK Configurator uses JSON for keymaps. The required fields are `keyboard`, `keymap`, `layout`, and `layers`; `layers` is an array with one array per layer. [21]

Minimal QMK Configurator keymap example:

```json
{
  "keyboard": "example/tinypad",
  "keymap": "default",
  "layout": "LAYOUT",
  "layers": [
    ["KC_ESC", "KC_Q"],
    ["KC_GRV", "KC_1"]
  ]
}
```

For rendering, QMK is strongest when geometry and keymap files are paired. The geometry file maps physical positions to matrix slots. The keymap file maps layout macro positions to actions. The renderer then needs a normalization layer that turns `KC_ESC`, `MO(1)`, `LT(2, KC_SPC)`, and `_______` into readable legends.

### Finding 4: VIA adds device detection and runtime configuration to QMK-family boards

VIA needs a keyboard definition to configure a device. VIA documentation says this definition describes the physical layout, layout options, encoders, lighting, and other configurable elements. The definition is JSON, stored in the VIA GitHub repository and hosted by VIA for the web app to use. It includes fields such as `name`, `vendorId`, `productId`, `matrix`, and `layouts.keymap`. The VID/PID pair identifies the keyboard when it is plugged in. [2]

Minimal VIA definition example:

```json
{
  "name": "TinyPad",
  "vendorId": "0xFEED",
  "productId": "0x0001",
  "matrix": { "rows": 1, "cols": 2 },
  "layouts": {
    "keymap": [
      ["0,0", "0,1"]
    ]
  }
}
```

The `layouts.keymap` field is KLE JSON. VIA's convention places switch matrix coordinates in the top-left legend. If a keyboard supports layout options, the bottom-right legend encodes option and choice values. VIA's layout docs describe layout options for choices such as split backspace or ISO enter, and note that VIA stores the user's selected layout-option state on the device. [3]

Example VIA layout option shape:

```json
{
  "layouts": {
    "labels": ["Split Backspace"],
    "keymap": [
      [
        { "w": 2 },
        "0,0\n\n\n\n\n\n\n\n\n\n\n0,0",
        { "x": 0.25 },
        "0,0\n\n\n\n\n\n\n\n\n\n\n0,1",
        "0,1\n\n\n\n\n\n\n\n\n\n\n0,1"
      ]
    ]
  }
}
```

This example is intentionally small and illustrative. In real VIA definitions, the KLE geometry and legend placement must follow VIA's rules so the configurator can distinguish default choices from optional choices.

The `the-via/keyboards` repository is the public definition repository. Its README says adding a keyboard requires QMK source in QMK firmware, a `keymaps/via` keymap in VIA's QMK userspace, and a JSON definition submitted to the VIA keyboard repository. Definitions live under `v3/vendor/keyboardname/keyboardname.json`. [22]

VIA provides:

- Board identity through VID/PID.
- KLE-derived physical geometry.
- Matrix mapping.
- Layout options.
- Runtime keymap editing when firmware enables VIA.
- Macros, lighting, and encoder UI where supported.

VIA omits:

- A complete firmware build recipe in the definition JSON.
- A universal source format for non-QMK boards.
- Full semantics for complex custom firmware code.

VIA-compatible firmware in QMK is a separate `keymaps/via` build target. VIA's QMK guide says `VIA_ENABLE = yes` enables dynamic keymaps, raw HID message handling, and bootmagic lite, and that dynamic keymaps default to four layers unless configured otherwise. [23]

### Finding 5: Vial is VIA-like, but its definition file is not the same file

Vial is a runtime configurator for keyboards using Vial-enabled firmware. Vial porting documentation starts with `vial.json`, a fixed-format JSON file that describes the physical layout so the Vial app or web app can draw the keyboard. Vial's docs say a VIA definition can be a starting point, but `info.json` and `vial.json` are very different and cannot be copied or renamed into one another. [24]

Minimal Vial-style geometry example:

```json
{
  "name": "TinyPad",
  "vendorId": "0xFEED",
  "productId": "0x0001",
  "matrix": { "rows": 1, "cols": 2 },
  "layouts": {
    "keymap": [
      ["0,0", "0,1"]
    ]
  }
}
```

This looks close to VIA because both use KLE-derived layout data and matrix legends. The surrounding ecosystem differs. Vial expects Vial firmware support, and many details live outside `vial.json`.

For a renderer, Vial matters because many boards are configured through Vial rather than VIA. NocFree's own Lite guide tells users to go to Vial, pair the keyboard, remap keys, create macros, and use up to four layers. [12]

Vial provides:

- Physical layout data for the Vial app.
- Runtime key remapping through Vial firmware.
- Layer and macro configuration from the user interface.
- A practical path for boards that ship with Vial support.

Vial omits:

- A guarantee that the board exists in VIA's repository.
- A complete firmware source representation in `vial.json`.
- A universal export format for every current runtime state.

### Finding 6: ZMK uses devicetree for keymaps and separate physical layout data

ZMK uses devicetree syntax for keymaps. ZMK documentation says keymaps live in a `<keyboard>.keymap` file and define mappings, behaviors, and feature configuration. A ZMK keymap has a `keymap` node, one child node per layer, and bindings that refer to behaviors. [4]

Minimal ZMK keymap example:

```dts
/ {
    keymap {
        compatible = "zmk,keymap";

        base {
            display-name = "Base";
            bindings = <
                &kp ESC  &kp Q
            >;
        };

        nav {
            display-name = "Nav";
            bindings = <
                &kp GRAVE  &kp N1
            >;
        };
    };
};
```

ZMK behavior is explicit. `&kp ESC` is a key press behavior. `&mo 1` momentarily activates a layer. `&lt 1 SPACE` and hold-tap behaviors can express richer timing rules. That makes ZMK a good behavior source, but the `.keymap` file alone does not always give a renderer the physical positions.

ZMK physical layouts fill that gap. ZMK physical layout docs say every physical layout needs a matrix transform and may have a kscan. The optional `keys` property is required for ZMK Studio support and describes each key with width, height, x, y, rotation, and rotation origin, in the same order as keymap bindings and matrix transforms. [5]

Minimal ZMK physical layout example:

```dts
#include <physical_layouts.dtsi>

/ {
    physical_layout0: physical_layout_0 {
        compatible = "zmk,physical-layout";
        display-name = "TinyPad";
        transform = <&default_transform>;
        keys = <
            &key_physical_attrs 100 100   0   0 0 0 0
            &key_physical_attrs 125 100 100   0 0 0 0
        >;
    };
};
```

The dimensions use centi-keyunit style values in the ZMK docs, so `100` is a 1u key and `125` is a 1.25u key. ZMK docs also recommend converting from QMK JSON or existing layout tools when possible. [5]

ZMK provides:

- Rich behavior syntax.
- Good support for split and wireless keyboards.
- Devicetree-based source control.
- Physical layout data for Studio-compatible boards.
- Runtime editing through ZMK Studio when firmware is built for it.

ZMK omits:

- A single JSON file equivalent to VIA definitions.
- Simple parsing without include resolution.
- A guarantee that source `.keymap` files match runtime-edited devices after Studio takes over.

### Finding 7: ZMK Studio, Keymap Editor, MoErgo Layout Editor, and Kinesis Clique are workflows over ZMK

ZMK Studio adds runtime editing to ZMK. Its docs say once Studio manages a keymap, later `.keymap` source changes will not apply unless the user restores stock settings. ZMK Studio can run in Chrome or Edge at `zmk.studio` or as a native app, and Studio support requires firmware built with Studio settings plus a physical layout with the `keys` property. [6]

Minimal Studio-related keymap pattern:

```dts
/ {
    keymap {
        compatible = "zmk,keymap";

        base {
            display-name = "Base";
            bindings = <
                &studio_unlock  &kp Q
            >;
        };

        extra1 {
            status = "reserved";
        };
    };
};
```

The important idea is that Studio is a runtime editor. It is not just a file format. A renderer reading a GitHub repo may see the stock `.keymap`; a Studio-managed device may contain a different runtime keymap.

Keymap Editor is a browser-based graphical editor for ZMK source keymaps. ZMK's blog describes it as a visual editor that can render devicetree keymaps using predefined, generated, or side-loadable layouts; it can integrate with GitHub, the local file system, or the clipboard; and it can edit combos, behaviors, macros, conditional layers, and rotary encoder bindings. [25]

MoErgo Layout Editor is the user-facing editor for MoErgo keyboards. The Go60 docs say Go60 uses open-source ZMK-based firmware, the easiest workflow is MoErgo Layout Editor, and the alternative is editing the keymap file and compiling ZMK firmware. The editor can generate a `.uf2` firmware file. MoErgo also publishes an official Go60 ZMK config repository for source-based workflows. [13][14][26]

MoErgo's advanced docs also expose how close the editor remains to ZMK. Custom Defined Behaviors let users inject text into the keymap DTSI file for unsupported ZMK features such as mod-morph or tap-dance. That is a strong signal that MoErgo's product UI is a layer over ZMK concepts, not a separate universal keyboard spec. [27]

Kinesis Clique plays a similar role for Kinesis ZMK devices. Kinesis says Clique supports ZMK-powered devices such as Advantage360 Professional, uses a desktop Chromium-family browser, connects over a serial port, and does not support wireless programming. Kinesis's Clique upgrade page says Clique uses the ZMK Studio backbone and allows Advantage360 users to rearrange keys in real time from a browser without rebuilding firmware. [15][16]

The older Kinesis Advantage360 Pro GUI repository now states that it has been replaced by Clique, a runtime keymap editor based on ZMK Studio. The Adv360-Pro-ZMK repository remains the source-controlled build path for users who manage firmware through GitHub Actions or local containers. [17][28]

### Finding 8: Kanata and KMonad are behavior languages without complete physical geometry

Kanata and KMonad run on the host computer. They intercept keyboard input and emit different input. They can give ordinary keyboards QMK-like behavior without flashing firmware.

Kanata configuration uses Lisp-like forms. A `defsrc` lists the source keys. A `deflayer` lists actions in the same sequence. Kanata's configuration guide says the order of `deflayer` actions corresponds to the same sequence position in `defsrc`, and the first layer is the starting layer. [9]

Minimal Kanata example:

```lisp
(defsrc
  caps a s d
)

(deflayer base
  esc  a s d
)

(deflayer nav
  _    left down right
)
```

This is enough to remap keys. It is not enough to draw the exact physical keyboard unless the renderer already knows where `caps`, `a`, `s`, and `d` sit.

KMonad uses a similar conceptual structure: `defcfg` for input and output setup, `defsrc` for source keys, and `deflayer` for mappings. KMonad's README describes it as an advanced tool for customizing almost any keyboard, with features such as layers, multi-tap, and tap-hold, usually associated with QMK firmware. [10]

Minimal KMonad example:

```lisp
(defcfg
  input  (device-file "/dev/input/by-id/example-event-kbd")
  output (uinput-sink "KMonad output")
  fallthrough true
)

(defsrc
  caps a s d
)

(deflayer base
  esc  a s d
)
```

Kanata and KMonad provide:

- Host-level remapping.
- Layers.
- Tap-hold and advanced behavior.
- Independence from keyboard firmware.

They omit:

- Board-specific x/y geometry.
- USB firmware identity.
- A canonical visual layout.

For rendering, pair Kanata or KMonad behavior with a geometry source such as KLE, QMK, VIA, Vial, or a manual layout.

### Finding 9: Ergogen describes keyboards as parametric design objects

Ergogen is a keyboard design generator. Its docs say the heart of Ergogen is a single YAML config file, with JSON or JavaScript also accepted. The top-level keys include `meta`, `units`, `points`, `outlines`, `cases`, and `pcbs`; `points` describes the positions of keys, and downstream sections generate outlines, cases, and PCB templates. [29]

Minimal Ergogen example:

```yaml
units:
  kx: 19
  ky: 19

points:
  zones:
    main:
      columns:
        index:
        middle:
          stagger: 5
      rows:
        home:
        top:
          shift: [0, 19]

outlines:
  plate:
    - what: rectangle
      where: true
      size: [kx, ky]
```

Ergogen points carry x/y position and rotation. Its anchor system can shift, rotate, orient, mirror, and average points. [30]

Ergogen provides:

- Parametric board design.
- Column stagger and ergonomic geometry.
- Derived outlines, cases, and PCB footprints.
- A design source for custom keyboards.

Ergogen omits:

- A simple runtime keymap.
- A firmware behavior source by default.
- A safe-to-execute interchange format when configs contain JavaScript.

For renderers, Ergogen is most useful after evaluation. Import generated KLE, SVG, point data, or a safe JSON representation rather than running arbitrary JavaScript configs inside a renderer.

### Finding 10: keymap-drawer is a renderer-oriented interchange layer

keymap-drawer parses QMK and ZMK keymaps and draws SVG diagrams. Its README says it supports multiple layers, hold-tap keys, combos, human-editable YAML, automatic parsing from QMK or ZMK keymaps, arbitrary physical layouts with rotated keys, custom drawing configuration, and custom SVG icons. It also explicitly decouples physical layout from keymap definitions. [8]

Minimal keymap-drawer-style YAML example:

```yaml
layout:
  qmk_keyboard: ferris/sweep
  qmk_layout: LAYOUT_split_3x5_2

layers:
  Base:
    - [Q, W, E, R, T, Y, U, I, O, P]
    - [A, S, D, F, G, H, J, K, L, ";"]
    - [Z, X, C, V, B, N, M, ",", ".", "/"]
    - [Ctrl, Space, Enter, Shift]

combos:
  - p: [0, 1]
    k: Esc
```

The exact schema supports more than this small example. The important point is architectural: keymap-drawer is not firmware. It is a diagram source and renderer that can bootstrap from firmware files.

keymap-drawer provides:

- Static rendering to SVG.
- A YAML representation that is easier to edit than firmware code.
- QMK and ZMK parsers.
- Hold-tap and combo visualization.

It omits:

- Runtime device editing.
- Guaranteed lossless firmware semantics.
- Live layer or pressed-key state.

### Finding 11: KeyPeek and keyviz show two live overlay models

Live overlays split into two different categories. KeyPeek is firmware-aware. keyviz is input-event-aware.

KeyPeek provides a live on-screen overlay that mirrors active base and momentary layers for QMK, Vial, and ZMK keyboards. Its README says stock QMK, Vial, and ZMK firmware does not expose live layer-change events, so KeyPeek requires a small firmware module that streams those events over the device connection. For QMK and Vial, the setup adds a KeyPeek module to the keymap, enables RAW HID and VIA, and flashes the firmware. For QMK, the user also exports `keyboard_info.json` with `qmk info`; Vial does not need that export because it transmits layout data to KeyPeek when connected. For ZMK, KeyPeek uses ZMK Studio support and raw HID modules, then reads layout and keymap data directly from the device. [33]

Minimal KeyPeek QMK/Vial setup example:

```json
{
  "modules": [
    "srwi/keypeek_layer_notify"
  ]
}
```

```make
RAW_ENABLE = yes
VIA_ENABLE = yes
```

Minimal KeyPeek ZMK module example:

```yaml
manifest:
  remotes:
    - name: srwi
      url-base: https://github.com/srwi
  projects:
    - name: zmk-keypeek-layer-notifier
      remote: srwi
      revision: master
```

KeyPeek's design is an important ecosystem signal: a live keyboard overlay needs a live state channel. Static layout files can draw a board, but they cannot tell the overlay which momentary layer is active right now. KeyPeek solves that by adding firmware support and by leaning on VIA/Vial/ZMK device protocols where possible. [33]

keyviz solves a different problem. Its site describes it as a free, open-source tool for visualizing keystrokes and mouse actions in real time, with customization for keycap style, size, color, border, icon, animations, mouse tracking, privacy-local processing, and input filtering. Its repository describes the same scope and adds implementation details: keyviz can show keypresses, mouse clicks, scroll wheel events, recent input history, screen position, and filters for which keys appear. [34][35]

Minimal keyviz event example:

```ts
type EventPayload =
  | { type: "KeyEvent"; pressed: boolean; name: "MetaLeft" | "KeyK" | string }
  | { type: "MouseButtonEvent"; pressed: boolean; button: "Left" | "Right" | "Middle" | "Other" }
  | { type: "MouseMoveEvent"; x: number; y: number }
  | { type: "MouseWheelEvent"; delta_x: number; delta_y: number };
```

Minimal keyviz style export example:

```json
{
  "appearance": {
    "flexDirection": "column",
    "alignment": "bottom-center",
    "animation": "fade",
    "style": "lowprofile"
  },
  "layout": {
    "showIcon": true,
    "showSymbol": true,
    "showPressCount": true
  },
  "text": {
    "size": 32,
    "caps": "capitalize",
    "variant": "text-short"
  },
  "mouse": {
    "showClicks": true,
    "showIndicator": true
  }
}
```

keyviz is valuable for demos, tutorials, and screen sharing because it shows what the presenter is pressing. It is not a keyboard-layout renderer in the QMK/VIA/ZMK sense. It does not know that `KeyK` came from a thumb cluster, a remapped layer, a split keyboard, or a Kanata action. It sees the host-level event stream after firmware and remappers have resolved the physical key into an OS event. [34][35]

The first-principles distinction is:

- keymap-drawer renders static keymaps from source files.
- KeyPeek renders a connected keyboard's active firmware-aware layers.
- keyviz renders OS input events and mouse actions.

All three are useful. They should not be treated as interchangeable specs.

### Finding 12: Monkeytype and OverKeys show the rendering/style end of the spectrum

Monkeytype is a typing practice site, not a keyboard firmware tool. Its relevant contribution to this topic is its visual keymap feature and theme ecosystem. A current Monkeytype settings page lists keymap display modes such as off, static, react, and next; keymap styles such as staggered, matrix, split, and split matrix; and legend styles such as lowercase, uppercase, and blank. [31]

Monkeytype-style renderer settings look conceptually like this:

```json
{
  "keymap": "react",
  "keymapStyle": "split matrix",
  "keymapLegendStyle": "uppercase",
  "theme": {
    "background": "#111111",
    "main": "#88c0d0",
    "text": "#d8dee9"
  }
}
```

This is a conceptual example, not a Monkeytype export schema. Monkeytype theme repositories usually change CSS, colors, and UI feel rather than firmware behavior. One Monkeytype theme repository describes style themes as changing Monkeytype's color and feel. [32]

OverKeys is an on-screen keyboard layout visualizer. Its README describes it as a tool for practicing alternative layouts system-wide, with user-defined layouts, layer switching integration, and customization options. Its current model is app-local: row arrays, visual styles, custom aliases, triggers, and Kanata host/port settings. [11]

Minimal OverKeys-style user layout example:

```json
{
  "defaultUserLayout": "Colemak Example",
  "kanataHost": "127.0.0.1",
  "kanataPort": 4039,
  "userLayouts": [
    {
      "name": "Colemak Example",
      "keys": [
        ["Q", "W", "F", "P", "B", "J", "L", "U", "Y"],
        ["A", "R", "S", "T", "G", "M", "N", "E", "I"],
        ["Z", "X", "C", "D", "V", "K", "H", ",", "."]
      ]
    }
  ]
}
```

Monkeytype and OverKeys provide:

- Visual keymap rendering.
- Style presets and themes.
- Practice-oriented overlays.
- Pressed-key or active-layer visualization in some modes.

They omit:

- Canonical board geometry.
- Firmware source.
- Runtime firmware editing.

For ecosystem design, these tools prove that rendering is a separate product layer. They can consume keyboard data, but their visual choices should not be mistaken for firmware specs.

### Finding 13: The named keyboards span Vial and ZMK workflows

NocFree Lite belongs in the Vial family. NocFree's guide tells Lite users to use Vial, pair the keyboard, remap keys, create macros, and use up to four layers. That makes Vial the relevant spec and workflow for rendering or configuring this board. [12]

MoErgo Go60 belongs in the ZMK family. MoErgo's Go60 docs say the keyboard uses open-source ZMK-based firmware, the easiest path is MoErgo Layout Editor, and the alternative is editing a keymap file and compiling ZMK firmware. The official Go60 config repository says it is the official ZMK configuration for the Go60 wireless split keyboard and can build `go60.uf2` through GitHub Actions. [13][14][26]

Kinesis Advantage360 Pro also belongs in the ZMK family, but its end-user workflow now emphasizes Clique. Kinesis says Clique supports ZMK-powered devices, connects through Chromium-family browsers over serial, and does not support wireless programming. Kinesis's older Advantage360 Pro GUI repository says that tool was replaced by Clique, which is based on ZMK Studio. The Adv360-Pro-ZMK repository remains the GitHub and local build path. [15][16][17][28]

The device lesson is straightforward:

| Keyboard | Main user workflow | Underlying family | Best rendering sources |
| --- | --- | --- | --- |
| NocFree Lite | Vial web app | Vial / QMK-family firmware | Vial JSON, possibly KLE/VIA-like geometry |
| MoErgo Go60 | MoErgo Layout Editor or Go60 ZMK repo | ZMK / MoErgo ZMK fork | ZMK `.keymap`, ZMK physical layout or vendor preset |
| Kinesis Advantage360 Pro | Kinesis Clique or Adv360-Pro-ZMK | ZMK / ZMK Studio / Kinesis fork | ZMK source repo, Kinesis key-position assets, Studio/Clique runtime caveats |

## Synthesis & Insights

The ecosystem has four centers of gravity. The QMK/VIA/Vial center is JSON-heavy and KLE-influenced. It works well for wired custom keyboards, dynamic keymaps, and device matching through VID/PID. The ZMK center is devicetree-heavy and strong for wireless, split, and ergonomic boards. It increasingly uses runtime editing through ZMK Studio and vendor UIs such as Kinesis Clique. The design/rendering center uses KLE, KLE NG, Ergogen, keymap-drawer, Monkeytype, and OverKeys to draw, design, or practice layouts without necessarily owning firmware state. The live-overlay center splits again: KeyPeek tries to read live keyboard state, while keyviz displays the host's input event stream. [33][34][35]

The best interchange path depends on what you are building.

If you are building a board renderer, import QMK `keyboard.json` or `info.json`, KLE, VIA, Vial, ZMK physical layouts, and evaluated Ergogen output. These sources contain physical shape. Avoid starting with Kanata or KMonad unless the user only needs logical remap display.

If you are building a keymap diagram generator, import QMK keymap JSON, ZMK `.keymap`, keymap-drawer YAML, and maybe VIA/Vial backup exports. Then attach geometry from QMK, KLE, VIA, Vial, or ZMK physical layouts.

If you are building a configurator, choose a runtime target first. VIA and Vial write to compatible QMK-family firmware. ZMK Studio writes to Studio-enabled ZMK firmware. MoErgo Layout Editor and Kinesis Clique are product workflows over ZMK. Kanata and KMonad write to host-side remapping processes, not keyboard firmware.

If you are building an overlay or practice tool, decide whether the source of truth is firmware state or OS input events. KeyPeek shows the firmware-aware path: get layout and layer data from QMK/Vial/ZMK routes and add a firmware event stream when stock firmware lacks one. keyviz shows the presentation path: capture local keypress and mouse events, filter them, and render them clearly for an audience. Monkeytype and OverKeys show that users also care about staggered, matrix, split, colors, legends, and pressed-key feedback. Those visual choices should sit on top of imported geometry and behavior rather than replacing them. [11][31][33][34][35]

The most stable cross-ecosystem model has four objects:

```json
{
  "physicalLayout": {
    "keys": [
      { "id": "k00", "matrix": [0, 0], "x": 0, "y": 0, "w": 1, "h": 1 }
    ]
  },
  "logicalKeymap": {
    "layers": {
      "Base": { "k00": { "raw": "KC_ESC", "display": "Esc" } }
    }
  },
  "runtimeState": {
    "activeLayer": "Base",
    "pressedKeys": ["k00"]
  },
  "inputEvents": [
    { "type": "KeyEvent", "pressed": true, "name": "MetaLeft" }
  ],
  "style": {
    "keymapStyle": "split matrix",
    "theme": "nord"
  }
}
```

This model is simple enough to render and rich enough to preserve source-specific data. It also prevents a common failure: using a visual row layout as if it were a firmware matrix, or using a firmware keymap as if it described physical positions.

## Limitations

This report uses public documentation and repositories. Some vendor tools may have private schemas or device protocols that are not documented publicly. MoErgo Layout Editor and Kinesis Clique are important workflows, but their stable public integration surface is mostly ZMK source, generated firmware, or Studio-style runtime behavior.

The examples are minimal and illustrative. Real keyboard definitions contain more metadata, more layers, more layout choices, and more source-specific edge cases. A production parser must use each project's real schema, not the small examples in this report.

Some sources are fast moving. ZMK Studio, KeyPeek, keyviz, KLE NG, vendor editors, and VIA definitions can change. Any implementation plan should pin source versions or re-check docs when work begins.

## Recommendations

1. Model the keyboard stack explicitly.

   Use separate data objects for physical geometry, logical keymaps, behavior semantics, runtime state, host input events, and visual style.

2. Prefer canonical geometry sources.

   Use QMK `keyboard.json` or `info.json` for QMK boards, VIA definitions from `the-via/keyboards` for VIA boards, Vial JSON for Vial boards, ZMK physical layouts for ZMK Studio-class boards, and KLE/KLE NG for manual visual layouts.

3. Prefer canonical behavior sources.

   Use QMK Configurator JSON or QMK source for QMK behavior, ZMK `.keymap` for ZMK behavior, Kanata `.kbd` for Kanata behavior, KMonad `.kbd` for KMonad behavior, and keymap-drawer YAML for diagrams.

4. Treat product editors as workflows.

   MoErgo Layout Editor, Kinesis Clique, Keymap Editor, KeyPeek, keyviz, Monkeytype, and OverKeys are user experiences over data. Each may expose or consume useful data, but none should be assumed to be a universal keyboard spec.

5. Normalize display labels but preserve raw actions.

   A renderer can show `Esc` while storing `KC_ESC`, `&kp ESC`, `esc`, or a Vial runtime code. Raw data is needed for debugging, round-tripping, and source-specific behavior.

6. Build around device families.

   NocFree Lite requires Vial support. MoErgo Go60 requires ZMK and MoErgo workflow awareness. Kinesis Advantage360 Pro requires ZMK, ZMK Studio, Kinesis Clique, and Adv360-Pro-ZMK awareness.

7. Pick the right live overlay model.

   Use a KeyPeek-style approach when the goal is to show active firmware layers and current keymap state. Use a keyviz-style approach when the goal is to show the user's visible input events during demos, tutorials, or screen sharing.

## Bibliography

[1] QMK Firmware. "`info.json` Reference." https://docs.qmk.fm/reference_info_json

[2] VIA. "Specification." https://caniusevia.com/docs/specification/

[3] VIA. "Layouts." https://caniusevia.com/docs/layouts/

[4] ZMK Firmware. "Keymaps & Behaviors." https://zmk.dev/docs/keymaps

[5] ZMK Firmware. "Physical Layouts." https://zmk.dev/docs/hardware-integration/physical-layouts

[6] ZMK Firmware. "ZMK Studio." https://zmk.dev/docs/features/studio

[7] Vial. "Build support 1 - Create JSON." https://get.vial.today/docs/porting-to-via.html

[8] caksoylar. "keymap-drawer." https://github.com/caksoylar/keymap-drawer

[9] jtroo. "Kanata Configuration Guide for defsrc, deflayer, and actions." https://jtroo.github.io/config.html

[10] KMonad. "KMonad." https://github.com/kmonad/kmonad

[11] conventoangelo. "OverKeys README." https://github.com/conventoangelo/OverKeys/blob/main/README.md

[12] NocFree. "Get the most out of NocFree Lite with Vial." https://www.nocfree.com/blogs/news/get-the-most-out-of-nocfree-lite-with-vial

[13] MoErgo. "Customizing key layout and swapping keycaps." https://docs.moergo.com/go60-user-guide/customizing-key-layout/

[14] MoErgo. "Appendix: ZMK." https://docs.moergo.com/go60-user-guide/appendix-zmk/

[15] Kinesis. "Clique Help." https://kinesis-ergo.com/clique-help/

[16] Kinesis. "Upgrading your Advantage 360 Pro to Access Clique." https://kinesis-ergo.com/360p-clique-upgrade/

[17] Kinesis Corporation. "Adv360-Pro-ZMK." https://github.com/KinesisCorporation/Adv360-Pro-ZMK

[18] Ian Prest. "Keyboard Layout Editor Serialized Data Format." https://github.com/ijprest/keyboard-layout-editor/wiki/Serialized-Data-Format

[19] adamws. "KLE NG." https://github.com/adamws/kle-ng and https://editor.keyboard-tools.xyz/

[20] QMK Firmware. "QMK Keymap Overview and Layer Model." https://docs.qmk.fm/keymap

[21] QMK Firmware. "Adding Default Keymaps to QMK Configurator." https://docs.qmk.fm/configurator_default_keymaps

[22] the-via. "VIA Keyboards." https://github.com/the-via/keyboards

[23] VIA. "Configuring QMK." https://caniusevia.com/docs/configuring_qmk/

[24] Vial. "Create keyboard definition JSON." https://get.vial.today/docs/porting-to-via.html

[25] ZMK Firmware. "Community Spotlight Series #1: Keymap Editor." https://zmk.dev/blog/2023/11/09/keymap-editor

[26] MoErgo Keyboards. "go60-zmk-config." https://github.com/moergo-keyboards/go60-zmk-config

[27] MoErgo. "Advanced usage: Custom Defined Behaviors." https://docs.moergo.com/layout-editor-guide/advanced-usage-custom-defined-behaviors/

[28] Kinesis Corporation. "Adv360-Pro-GUI." https://github.com/KinesisCorporation/Adv360-Pro-GUI

[29] Ergogen. "Ergogen Config Overview." https://docs.ergogen.xyz/config-overview/

[30] Ergogen. "Points." https://docs.ergogen.xyz/points/

[31] Monkeytype. "Settings and keymap options." https://seerlite.github.io/monkeytype/

[32] refact0r. "monkeytype-themes." https://github.com/refact0r/monkeytype-themes

[33] srwi. "KeyPeek." https://github.com/srwi/keypeek

[34] Keyviz. "Visualise your keypress in real-time." https://keyviz.org/

[35] mulaRahul. "keyviz." https://github.com/mulaRahul/keyviz

## Methodology Appendix

I treated each source as answering one or more first-principles questions: physical geometry, electrical matrix, logical keymap, behavior language, runtime editing, host input events, and rendering. Primary sources were preferred: official docs, project READMEs, and vendor support pages. I used product-specific sources for NocFree Lite, MoErgo Go60, and Kinesis Advantage360 Pro because those workflows differ from the general QMK/VIA and ZMK documentation. I added KeyPeek and keyviz as separate overlay models after reviewing their project documentation and source-level configuration shapes.

The examples are small by design. They show the shape of each format without trying to cover every feature. Production implementations should validate against the upstream schema or parser for each ecosystem.
