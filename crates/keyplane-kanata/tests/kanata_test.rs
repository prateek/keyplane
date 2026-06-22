//! Kanata message parsing, layer mapping, and the live TCP stream.

use keyplane_core::backend::{BackendUpdate, ProtocolBackend};
use keyplane_core::ids::LayerId;
use keyplane_core::model::ActivationKind;
use keyplane_core::provenance::SourceKind;
use keyplane_kanata::{parse_message, KanataBackend, KanataEvent, LayerMap};
use std::io::Write;
use std::net::TcpListener;
use std::time::{Duration, Instant};

fn demo_map() -> LayerMap {
    LayerMap::new(
        [
            ("base".to_string(), LayerId::new("layer-0")),
            ("nav".to_string(), LayerId::new("layer-1")),
        ],
        LayerId::new("layer-0"),
    )
}

#[test]
fn parses_layer_change_and_ignores_other_messages() {
    assert_eq!(
        parse_message(r#"{"LayerChange":{"new":"nav"}}"#),
        Some(KanataEvent::LayerChange { layer: "nav".into() })
    );
    assert_eq!(parse_message(r#"{"Acknowledge":{}}"#), None);
    assert_eq!(parse_message("not json"), None);
}

#[test]
fn layer_map_builds_a_base_plus_active_stack() {
    let map = demo_map();
    // The base layer alone is just the base.
    assert_eq!(map.stack_for("base").active.len(), 1);

    // A non-base layer sits on top of the base as a remapper layer.
    let stack = map.stack_for("nav");
    assert_eq!(stack.active.len(), 2);
    assert_eq!(stack.top().unwrap().layer, LayerId::new("layer-1"));
    assert_eq!(stack.top().unwrap().activation, ActivationKind::Remapper);
}

#[test]
fn unknown_layer_names_still_resolve() {
    assert_eq!(demo_map().resolve("mystery"), LayerId::new("mystery"));
}

#[test]
fn live_tcp_stream_emits_layer_stack_updates() {
    // A stand-in Kanata server that emits one LayerChange line.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut socket, _)) = listener.accept() {
            let _ = socket.write_all(b"{\"LayerChange\":{\"new\":\"nav\"}}\n");
            // Keep the socket open briefly so the client can read the line.
            std::thread::sleep(Duration::from_millis(200));
        }
    });

    let mut backend = KanataBackend::connect(addr, demo_map()).expect("connect");
    assert_eq!(backend.descriptor().kind, SourceKind::Kanata);

    // Poll until the reader thread delivers the update (or time out).
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut updates = Vec::new();
    while Instant::now() < deadline {
        updates = backend.poll();
        if !updates.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    match updates.first() {
        Some(BackendUpdate::LayerStack { stack, .. }) => {
            assert_eq!(stack.top().unwrap().layer, LayerId::new("layer-1"));
        }
        other => panic!("expected a layer-stack update, got {other:?}"),
    }
}
