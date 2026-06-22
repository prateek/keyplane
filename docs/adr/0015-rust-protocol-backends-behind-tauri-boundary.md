# Rust protocol backends behind the Tauri boundary

Protocol backends will live as Rust-side traits and services, with Tauri commands and events as the frontend boundary. KeyPeek-derived protocol and domain code can be reused in Rust, but the web frontend receives only serialized app-domain snapshots and runtime events, not direct Vial, ZMK, HID, Kanata TCP, or KeyPeek protocol APIs.
