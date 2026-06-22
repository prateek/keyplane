# Attribution

Keyplane is GPL-3.0-only.

## KeyPeek

Keyplane is intended to fork and reuse protocol/domain ideas from [KeyPeek](https://github.com/srwi/keypeek), which is GPL-3.0-only.

Current Keyplane code adapts the KeyPeek firmware-module packet contract for live layer updates:

- layer-state packets start with `0xff`
- pressed-key packets start with `0xf1`
- Raw HID subscription keepalive uses marker `0xc0` with active value `0xa1` and inactive value `0xa0`

KeyPeek's egui UI code has not been copied into Keyplane.

## keyviz

keyviz may inform future Visual Style import work under its GPL-3.0 terms. The current implementation does not copy keyviz code.
