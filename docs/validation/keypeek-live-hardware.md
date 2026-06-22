# KeyPeek Live Hardware Validation

Use this checklist when a QMK/Vial/ZMK device with the KeyPeek firmware module is connected.

## VIA Raw HID Discovery

1. Start the app with `npm run tauri dev`.
2. Click `Scan` in the KeyPeek Live controls.
3. Confirm Backend Health reports the discovered device count or a typed failure state.
4. Select the discovered device from the menu and confirm the VID/PID fields are populated.

Discovery finds VIA Raw HID endpoints. It does not prove the keyboard has the
KeyPeek firmware module until the subscription or layer-change checks pass.

## Validation Runner

The preferred validation path runs both ignored canaries and writes a sanitized
local report under `target/keyplane-validation/`:

```sh
KEYPLANE_KEYPEEK_LIVE_VID=feed \
KEYPLANE_KEYPEEK_LIVE_PID=cafe \
KEYPLANE_KEYPEEK_LIVE_WAIT_MS=10000 \
npm run validate:keypeek-live
```

The generated report redacts USB IDs. Add
`KEYPLANE_KEYPEEK_LIVE_DEVICE_LABEL` and
`KEYPLANE_KEYPEEK_LIVE_FIRMWARE_REF` when a public hardware or firmware label
should be recorded with the evidence.

To check report generation without opening hardware:

```sh
npm run validate:keypeek-live:dry
```

The dry run is not acceptance evidence. The layer-change Runtime Event canary
must pass against a real KeyPeek-compatible device before the KeyPeek live MVP
acceptance gate is satisfied.

## Raw HID Subscription Canary

The validation runner invokes this test. Run it directly when isolating Raw HID
open or subscription failures:

```sh
cd src-tauri
KEYPLANE_KEYPEEK_LIVE_VID=feed \
KEYPLANE_KEYPEEK_LIVE_PID=cafe \
cargo test local_keypeek_live_device_accepts_subscription_when_env_is_set -- --ignored
```

This validates that Keyplane can open the KeyPeek-compatible Raw HID endpoint
and send the KeyPeek live subscription start/stop messages.

## Layer-Change Canary

The validation runner invokes this test after the subscription canary. Run it
directly while manually activating a non-base layer on the keyboard within the
wait window:

```sh
cd src-tauri
KEYPLANE_KEYPEEK_LIVE_VID=feed \
KEYPLANE_KEYPEEK_LIVE_PID=cafe \
KEYPLANE_KEYPEEK_LIVE_WAIT_MS=10000 \
cargo test local_keypeek_live_device_emits_layer_change_when_env_is_set -- --ignored
```

This validates that Keyplane receives a KeyPeek firmware-module Layer Stack
Runtime Event from the real Raw HID stream. Increase
`KEYPLANE_KEYPEEK_LIVE_WAIT_MS` if the keyboard's layer chord is hard to trigger
while the test is running.

## Layer-Change Acceptance Check

1. Start the app with `npm run tauri dev`.
2. Enter the same VID/PID in the KeyPeek Live controls.
3. Click `Connect`.
4. Change layers on the keyboard.
5. Confirm the Overlay Window updates the Layer Stack, active layer highlight, and effective legends.
6. Confirm Backend Health shows `ok` for `KeyPeek Live`.

Record the generated validation report result, keyboard, firmware commit or
build identifier, and observed layer-change result in the PR or release notes.
Do not paste private local paths or unsanitized device identifiers unless they
are intentionally public.
