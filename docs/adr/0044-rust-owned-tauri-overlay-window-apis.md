# Rust-owned Tauri overlay window APIs

Create and control the Overlay Window from Rust using Tauri v2 window APIs. The overlay should be transparent, frameless, always on top, skipped from the taskbar, non-focusable during normal operation, and hidden until the first Keyboard Snapshot. Use cursor-event ignoring for Click-Through Mode where supported. Positioning Mode disables click-through and uses drag/resize APIs. Unsupported platform behavior becomes visible app/window capability health.
