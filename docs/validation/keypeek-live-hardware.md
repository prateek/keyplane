# KeyPeek Live Hardware Validation

Use this checklist when a QMK/Vial/ZMK device with the KeyPeek firmware module is connected.

## Raw HID Subscription Canary

Run the ignored hardware canary with the device VID/PID:

```sh
cd src-tauri
KEYPLANE_KEYPEEK_LIVE_VID=feed \
KEYPLANE_KEYPEEK_LIVE_PID=cafe \
cargo test local_keypeek_live_device_accepts_subscription_when_env_is_set -- --ignored
```

This validates that Keyplane can open the KeyPeek-compatible Raw HID endpoint and send the KeyPeek live subscription start/stop messages. It does not wait for a layer event, because the current `qmk-via-api` public read path is blocking.

## Layer-Change Acceptance Check

1. Start the app with `npm run tauri dev`.
2. Enter the same VID/PID in the KeyPeek Live controls.
3. Click `Connect`.
4. Change layers on the keyboard.
5. Confirm the Overlay Window updates the Layer Stack, active layer highlight, and effective legends.
6. Confirm Backend Health shows `ok` for `KeyPeek Live`.

Record the keyboard, firmware commit or build identifier, VID/PID, and observed layer-change result in the PR or release notes.
