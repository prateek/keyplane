# Hardware Validation Runbook

The MVP acceptance criteria (PRD line 178, ADR 0037) are met in code and tested,
with one criterion gated on a physical device by the PRD itself: *"one real
KeyPeek-backed live layer change **when supported hardware is available**."* ADR
0045 is explicit that core development must not block on a specific keyboard and
that the Fake Backend is the CI/dev validation path.

This runbook is the turnkey step to close that criterion the moment a
KeyPeek-supported board is attached. Nothing here needs new code.

## Prerequisites

- A QMK/Vial keyboard flashed with the KeyPeek firmware module (raw HID, usage
  page `0xff60`), or another KeyPeek-supported VIA/Vial/ZMK device.
- Its USB `vid:pid` (find it with the bundled probe below).

## Steps

1. List HID devices and find the keyboard's `vid:pid`:

   ```sh
   cargo run -p keyplane-keypeek --example probe_env
   ```

   Look for an interface with `up=0xff60` (VIA raw-HID) or a keyboard
   (`up=0x0001 usage=0x06`).

2. Stream live layer changes from the device:

   ```sh
   cargo run -p keyplane-keypeek --example live_keypeek -- <vid_hex> <pid_hex>
   ```

   It connects, prints the model built from the device's geometry + keymap, then
   prints a line on every live layer change. **Press a layer key on the keyboard
   and confirm the active layer updates here** — that is the 6th acceptance
   criterion.

3. End-to-end in the app: `pnpm tauri dev`, then in the App Window enter the
   `vid`/`pid` and click **Connect Vial**. The transparent Overlay Window
   re-renders the device's keymap and updates as you change layers.

## What is already validated without hardware

- The full backend pipeline — connect → model build → the threaded reader
  decoding real KeyPeek `0xff` layer packets → streamed Layer Stack updates — is
  integration-tested with an injected fake transport
  (`keypeek_live_test.rs`).
- The hidapi USB open/read transport is exercised against real HID hardware in
  `probe_env`.
- Sentinel OS capture is validated with a real macOS key hook
  (`validate_capture` example).

So the only thing this runbook adds is the specific KeyPeek-firmware device
emitting real layer packets, which is the hardware the PRD conditions the
criterion on.
