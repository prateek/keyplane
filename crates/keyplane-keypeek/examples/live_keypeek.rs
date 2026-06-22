//! Turnkey validator for the 6th MVP acceptance criterion (PRD line 178):
//! "one real KeyPeek-backed live layer change when supported hardware is
//! available."
//!
//! Connects to a real KeyPeek/Vial device by VID/PID, prints the model it built
//! from the device, then streams live Layer Stack changes — press a layer key on
//! the keyboard and the active layer prints here. This is the exact hardware
//! validation step; it needs a connected, KeyPeek-supported board (none is
//! attached in CI/headless environments, where the fake backend stands in per
//! ADR 0045).
//!
//! Usage:
//!   cargo run -p keyplane-keypeek --example live_keypeek -- <vid_hex> <pid_hex>
//!   # e.g.  cargo run -p keyplane-keypeek --example live_keypeek -- feed 0001

use keyplane_core::backend::{BackendUpdate, ProtocolBackend};
use keyplane_keypeek::{ConnectionSpec, KeyPeekBackend};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: live_keypeek <vid_hex> <pid_hex>");
        std::process::exit(2);
    }
    let vid = u16::from_str_radix(args[1].trim_start_matches("0x"), 16).expect("vid hex");
    let pid = u16::from_str_radix(args[2].trim_start_matches("0x"), 16).expect("pid hex");

    println!("connecting to Vial device {vid:04x}:{pid:04x} …");
    let (mut backend, model) = match KeyPeekBackend::connect(ConnectionSpec::Vial { vid, pid }, None)
    {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("connect failed: {e}");
            eprintln!("(no KeyPeek-supported device attached? see ADR 0045)");
            std::process::exit(1);
        }
    };

    println!(
        "built model: {} — {} keys, {} layers",
        model.name.as_deref().unwrap_or("?"),
        model.physical_layout.keys.len(),
        model.keymap.layers.len()
    );
    println!("streaming live layer changes (press a layer key; Ctrl-C to stop)…");

    loop {
        for update in backend.poll() {
            if let BackendUpdate::LayerStack { stack, confidence } = update {
                let top = stack.top().map(|a| a.layer.to_string()).unwrap_or_default();
                println!("  layer change -> top={top} ({confidence:?})");
            }
        }
        if !backend.health().is_ok() {
            println!("  backend health: {}", backend.health().tag());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
