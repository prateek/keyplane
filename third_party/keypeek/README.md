# Vendored KeyPeek Source Slice

This directory contains a narrow source snapshot from KeyPeek:

- upstream: https://github.com/srwi/keypeek
- commit: `9c8d4b3f7c30e088367ba361a52eb597e146a276`
- upstream license: GPL-3.0-only
- vendored on: 2026-06-22

The vendored files are intentionally limited to the protocol/domain code that
substantiates Keyplane's current KeyPeek-compatible live packet adapter:

- `src/keyboard.rs`: layer-state and pressed-key packet handling
- `src/protocols/mod.rs`: raw HID subscription markers and sender contract
- `src/protocols/via.rs`: VIA raw HID transport path
- `src/protocols/vial.rs`: Vial raw HID transport path
- `Cargo.toml` and `LICENSE`: upstream package and license metadata

Keyplane does not vendor KeyPeek's egui overlay UI. The application code adapts
the packet contract in `src-tauri/src/keypeek_contract.rs` and keeps tests that
compare the adapted constants against this vendored source slice.
