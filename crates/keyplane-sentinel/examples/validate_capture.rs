//! Real end-to-end validation of sentinel OS capture (ADR 0016).
//!
//! Runs the actual `rdev` global key hook (on the main thread, as macOS
//! requires) feeding a `SentinelBackend`, while a worker thread injects a
//! synthetic `Function` (Fn) press/release with `rdev::simulate` and checks the backend
//! produced the expected Layer Stack change. Requires macOS Input Monitoring +
//! Accessibility (this environment has both). `Function` (Fn) is used because it is a
//! no-op on macOS and almost never present on physical keyboards.
//!
//! Run: `cargo run -p keyplane-sentinel --example validate_capture`

use keyplane_core::backend::{BackendUpdate, ProtocolBackend};
use keyplane_core::ids::LayerId;
use keyplane_sentinel::{HostEvent, SentinelAction, SentinelBackend, SentinelKey};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() {
    let backend = Arc::new(Mutex::new(SentinelBackend::new(
        vec![SentinelKey {
            host_key: "Function".into(),
            action: SentinelAction::Momentary(LayerId::new("layer-1")),
        }],
        LayerId::new("layer-0"),
    )));

    // Worker: inject events and check the captured result, then exit the process
    // (the main thread is blocked in `rdev::listen`).
    let probe = backend.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500)); // let the tap install

        let _ = rdev::simulate(&rdev::EventType::KeyPress(rdev::Key::Function));
        std::thread::sleep(Duration::from_millis(200));
        let press = probe.lock().unwrap().poll();
        let captured_press = press.iter().any(|u| {
            matches!(u, BackendUpdate::LayerStack { stack, .. }
                if stack.top().map(|a| a.layer.as_str()) == Some("layer-1"))
        });

        let _ = rdev::simulate(&rdev::EventType::KeyRelease(rdev::Key::Function));
        std::thread::sleep(Duration::from_millis(200));
        let release = probe.lock().unwrap().poll();
        let captured_release = release.iter().any(|u| {
            matches!(u, BackendUpdate::LayerStack { stack, .. } if stack.active.len() == 1)
        });

        println!("sentinel capture: Fn press   -> layer-1 = {captured_press}");
        println!("sentinel capture: Fn release -> base    = {captured_release}");
        if captured_press && captured_release {
            println!("PASS: real OS key capture drove the sentinel Layer Stack");
            std::process::exit(0);
        } else {
            println!("FAIL: did not observe the expected capture");
            std::process::exit(1);
        }
    });

    // Main thread: the real global hook. Feeds captured keys to the backend
    // (non-sentinel keys are ignored by the config).
    let feed = backend;
    let _ = rdev::listen(move |event| {
        let (key, down) = match event.event_type {
            rdev::EventType::KeyPress(k) => (format!("{k:?}"), true),
            rdev::EventType::KeyRelease(k) => (format!("{k:?}"), false),
            _ => return,
        };
        let event = if down {
            HostEvent::KeyDown(key)
        } else {
            HostEvent::KeyUp(key)
        };
        feed.lock().unwrap().feed(event);
    });
}
