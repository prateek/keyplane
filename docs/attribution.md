# Attribution

Keyplane is GPL-3.0-only.

## KeyPeek

Keyplane reuses and adapts protocol/domain code from [KeyPeek](https://github.com/srwi/keypeek), which is GPL-3.0-only.

`third_party/keypeek` vendors a narrow KeyPeek source snapshot from commit `9c8d4b3f7c30e088367ba361a52eb597e146a276`. The vendored slice includes the keyboard packet loop, Raw HID subscription contract, and VIA/Vial transport paths. Keyplane adapts that source in `src-tauri/src/keypeek_contract.rs`, `src-tauri/src/keypeek_live.rs`, and `src-tauri/src/vial_device.rs` for live layer updates and Vial device imports:

- layer-state packets start with `0xff`
- pressed-key packets start with `0xf1`
- Raw HID subscription keepalive uses marker `0xc0` with active value `0xa1` and inactive value `0xa0`
- Vial device imports use Raw HID prefix `0xfe` with keyboard-id, definition-size, definition-block, XZ definition decoding, and `qmk-via-api` raw matrix reads

KeyPeek's egui UI code has not been copied into Keyplane.

## keyviz

keyviz may inform future Visual Style import work under its GPL-3.0 terms. The current implementation does not copy keyviz code.
