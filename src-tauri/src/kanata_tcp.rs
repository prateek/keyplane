use crate::domain::{
    ActivationKind, HealthState, Layer, LayerActivation, RuntimeEvent, StateConfidence,
    StateConfidenceLevel,
};
use crate::kanata_backend;
use serde_json::Value as JsonValue;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Emitter;
use thiserror::Error;

const READ_TIMEOUT: Duration = Duration::from_millis(250);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const KANATA_HELLO_COMMAND: &str = r#"{"Hello":{}}"#;
const KANATA_CURRENT_LAYER_COMMAND: &str = r#"{"RequestCurrentLayerName":{}}"#;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum KanataTcpError {
    #[error("{0}")]
    Transport(String),
    #[error("Kanata TCP parse failed: {0}")]
    Parse(String),
    #[error("Kanata TCP protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug, Clone)]
pub struct KanataLayerMap {
    base_layer_id: String,
    layers: Vec<KanataLayerRef>,
}

#[derive(Debug, Clone)]
struct KanataLayerRef {
    id: String,
    names: Vec<String>,
}

pub trait KanataLineTransport: Send + 'static {
    fn write_line(&mut self, line: &str) -> Result<(), KanataTcpError>;
    fn read_line(&mut self) -> Result<Option<String>, KanataTcpError>;
}

pub struct TcpKanataTransport {
    reader: BufReader<TcpStream>,
}

impl TcpKanataTransport {
    pub fn connect(host: &str, port: u16) -> Result<Self, KanataTcpError> {
        let address = (host, port)
            .to_socket_addrs()
            .map_err(|err| {
                KanataTcpError::Transport(format!("Invalid address {host}:{port}: {err}"))
            })?
            .next()
            .ok_or_else(|| KanataTcpError::Transport(format!("No address for {host}:{port}")))?;
        let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
            .map_err(|err| KanataTcpError::Transport(format!("Connect failed: {err}")))?;
        stream.set_read_timeout(Some(READ_TIMEOUT)).map_err(|err| {
            KanataTcpError::Transport(format!("Could not set read timeout: {err}"))
        })?;

        Ok(Self {
            reader: BufReader::new(stream),
        })
    }
}

impl KanataLineTransport for TcpKanataTransport {
    fn write_line(&mut self, line: &str) -> Result<(), KanataTcpError> {
        let stream = self.reader.get_mut();
        stream
            .write_all(line.as_bytes())
            .and_then(|_| stream.write_all(b"\n"))
            .and_then(|_| stream.flush())
            .map_err(|err| KanataTcpError::Transport(format!("Write failed: {err}")))
    }

    fn read_line(&mut self) -> Result<Option<String>, KanataTcpError> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => Err(KanataTcpError::Transport(
                "Kanata TCP connection closed".to_string(),
            )),
            Ok(_) => Ok(Some(line)),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                Ok(None)
            }
            Err(err) => Err(KanataTcpError::Transport(format!("Read failed: {err}"))),
        }
    }
}

pub struct KanataTcpSession<T> {
    transport: T,
    layer_map: KanataLayerMap,
}

impl<T: KanataLineTransport> KanataTcpSession<T> {
    pub fn new(transport: T, layer_map: KanataLayerMap) -> Self {
        Self {
            transport,
            layer_map,
        }
    }

    pub fn start(&mut self) -> Result<(), KanataTcpError> {
        self.transport.write_line(KANATA_HELLO_COMMAND)?;
        self.transport.write_line(KANATA_CURRENT_LAYER_COMMAND)
    }

    pub fn poll_next_event(&mut self) -> Result<Option<RuntimeEvent>, KanataTcpError> {
        let Some(line) = self.transport.read_line()? else {
            return Ok(None);
        };
        kanata_event_from_line(&line, &self.layer_map)
    }
}

impl KanataLayerMap {
    pub fn from_layers(layers: &[Layer]) -> Self {
        let base_layer_id = layers
            .first()
            .map(|layer| layer.id.clone())
            .unwrap_or_else(|| "layer-0".to_string());
        let layers = layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let mut names = vec![
                    layer.id.clone(),
                    layer.name.clone(),
                    normalize_layer_name(&layer.id),
                    normalize_layer_name(&layer.name),
                    format!("layer-{index}"),
                    index.to_string(),
                ];
                names.sort();
                names.dedup();
                KanataLayerRef {
                    id: layer.id.clone(),
                    names,
                }
            })
            .collect();

        Self {
            base_layer_id,
            layers,
        }
    }

    fn resolve(&self, name: &str) -> Option<&str> {
        let normalized = normalize_layer_name(name);
        self.layers
            .iter()
            .find(|layer| {
                layer
                    .names
                    .iter()
                    .any(|candidate| candidate == name || candidate == &normalized)
            })
            .map(|layer| layer.id.as_str())
    }
}

pub fn kanata_event_from_line(
    line: &str,
    layer_map: &KanataLayerMap,
) -> Result<Option<RuntimeEvent>, KanataTcpError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let json: JsonValue =
        serde_json::from_str(trimmed).map_err(|err| KanataTcpError::Parse(err.to_string()))?;

    if let Some(name) = layer_name_message(&json, "LayerChange", "new")
        .or_else(|| layer_name_message(&json, "CurrentLayerName", "name"))
    {
        return Ok(Some(RuntimeEvent::LayerStackChanged {
            layer_stack: layer_stack_for_kanata_layer(name, layer_map),
            source_id: Some(kanata_backend::KANATA_BACKEND_ID.to_string()),
        }));
    }

    if let Some(hello) = json.get("HelloOk").and_then(JsonValue::as_object) {
        let version = hello
            .get("version")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        let protocol = hello
            .get("protocol")
            .map(JsonValue::to_string)
            .unwrap_or_else(|| "unknown".to_string());
        return Ok(Some(RuntimeEvent::BackendHealthChanged {
            health: kanata_backend::kanata_backend_status(
                HealthState::Ok,
                format!("Connected to Kanata TCP {version} protocol {protocol}"),
            )
            .health,
        }));
    }

    if let Some(error) = json.get("Error").and_then(JsonValue::as_object) {
        let message = error
            .get("msg")
            .and_then(JsonValue::as_str)
            .unwrap_or("Kanata TCP reported an error");
        return Ok(Some(RuntimeEvent::BackendHealthChanged {
            health: kanata_backend::kanata_backend_status(HealthState::ProtocolError, message)
                .health,
        }));
    }

    if json.get("ConfigFileReload").is_some() {
        return Ok(Some(RuntimeEvent::BackendHealthChanged {
            health: kanata_backend::kanata_backend_status(
                HealthState::Stale,
                "Kanata reloaded its configuration; current layer will update on the next event",
            )
            .health,
        }));
    }

    Ok(None)
}

fn layer_stack_for_kanata_layer(name: &str, layer_map: &KanataLayerMap) -> Vec<LayerActivation> {
    let resolved = layer_map.resolve(name);
    let active_layer_id = resolved.unwrap_or(name);
    let known_layer = resolved.is_some();
    let base = LayerActivation {
        layer_id: layer_map.base_layer_id.clone(),
        kind: ActivationKind::Default,
        confidence: StateConfidence {
            level: StateConfidenceLevel::High,
            reason: "Base layer retained below Kanata remapper state".to_string(),
        },
    };

    if active_layer_id == layer_map.base_layer_id {
        return vec![LayerActivation {
            confidence: StateConfidence {
                level: StateConfidenceLevel::High,
                reason: "Kanata TCP reported the base layer".to_string(),
            },
            ..base
        }];
    }

    vec![
        LayerActivation {
            layer_id: active_layer_id.to_string(),
            kind: ActivationKind::RemapperState,
            confidence: StateConfidence {
                level: if known_layer {
                    StateConfidenceLevel::High
                } else {
                    StateConfidenceLevel::Medium
                },
                reason: if known_layer {
                    "Kanata TCP layer-change event".to_string()
                } else {
                    format!("Kanata TCP reported unknown layer {name}")
                },
            },
        },
        base,
    ]
}

fn layer_name_message<'a>(json: &'a JsonValue, key: &str, field: &str) -> Option<&'a str> {
    json.get(key)
        .and_then(JsonValue::as_object)?
        .get(field)
        .and_then(JsonValue::as_str)
}

fn normalize_layer_name(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}

#[derive(Default)]
pub struct KanataTcpRuntime {
    worker: Mutex<Option<KanataTcpWorker>>,
}

struct KanataTcpWorker {
    stop: Arc<AtomicBool>,
    _join: thread::JoinHandle<()>,
}

impl KanataTcpRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start<T: KanataLineTransport>(
        &self,
        app: tauri::AppHandle,
        session: KanataTcpSession<T>,
    ) -> Result<(), String> {
        self.stop();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let join = thread::spawn(move || run_kanata_loop(app, session, worker_stop));
        let mut worker = self
            .worker
            .lock()
            .map_err(|_| "Kanata TCP runtime is unavailable".to_string())?;
        *worker = Some(KanataTcpWorker { stop, _join: join });
        Ok(())
    }

    pub fn stop(&self) {
        let Ok(mut worker) = self.worker.lock() else {
            return;
        };
        if let Some(worker) = worker.take() {
            worker.stop.store(true, Ordering::Relaxed);
        }
    }
}

impl Drop for KanataTcpRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_kanata_loop<T: KanataLineTransport>(
    app: tauri::AppHandle,
    mut session: KanataTcpSession<T>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Relaxed) {
        match session.poll_next_event() {
            Ok(Some(event)) => {
                let _ = app.emit(crate::keypeek_live::RUNTIME_EVENT_NAME, event);
            }
            Ok(None) => {}
            Err(err) => {
                let state = match err {
                    KanataTcpError::Parse(_) => HealthState::ParseError,
                    KanataTcpError::Protocol(_) => HealthState::ProtocolError,
                    KanataTcpError::Transport(_) => HealthState::Disconnected,
                };
                let _ = app.emit(
                    crate::keypeek_live::RUNTIME_EVENT_NAME,
                    RuntimeEvent::BackendHealthChanged {
                        health: kanata_backend::kanata_backend_status(
                            state,
                            format!("Kanata TCP stream stopped: {err}"),
                        )
                        .health,
                    },
                );
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct FakeKanataTransport {
        writes: Vec<String>,
        reads: VecDeque<Option<String>>,
    }

    impl KanataLineTransport for FakeKanataTransport {
        fn write_line(&mut self, line: &str) -> Result<(), KanataTcpError> {
            self.writes.push(line.to_string());
            Ok(())
        }

        fn read_line(&mut self) -> Result<Option<String>, KanataTcpError> {
            self.reads
                .pop_front()
                .ok_or_else(|| KanataTcpError::Transport("fake read queue is empty".to_string()))
        }
    }

    fn layer_map() -> KanataLayerMap {
        KanataLayerMap::from_layers(&crate::fake_backend::fake_profile().keymap.layers)
    }

    #[test]
    fn maps_layer_change_messages_to_layer_stack_events() {
        let event = kanata_event_from_line(r#"{"LayerChange":{"new":"Navigation"}}"#, &layer_map())
            .expect("message parses")
            .expect("event is emitted");

        match event {
            RuntimeEvent::LayerStackChanged {
                layer_stack,
                source_id,
            } => {
                assert_eq!(
                    source_id.as_deref(),
                    Some(kanata_backend::KANATA_BACKEND_ID)
                );
                assert_eq!(layer_stack[0].layer_id, "layer-1");
                assert_eq!(layer_stack[0].kind, ActivationKind::RemapperState);
                assert_eq!(layer_stack[0].confidence.level, StateConfidenceLevel::High);
                assert_eq!(layer_stack[1].layer_id, "layer-0");
            }
            _ => panic!("expected layer stack event"),
        }
    }

    #[test]
    fn maps_current_base_layer_to_default_only_stack() {
        let event = kanata_event_from_line(r#"{"CurrentLayerName":{"name":"Base"}}"#, &layer_map())
            .expect("message parses")
            .expect("event is emitted");

        match event {
            RuntimeEvent::LayerStackChanged { layer_stack, .. } => {
                assert_eq!(layer_stack.len(), 1);
                assert_eq!(layer_stack[0].layer_id, "layer-0");
                assert_eq!(layer_stack[0].kind, ActivationKind::Default);
            }
            _ => panic!("expected layer stack event"),
        }
    }

    #[test]
    fn reports_unknown_layers_with_medium_confidence() {
        let event = kanata_event_from_line(r#"{"LayerChange":{"new":"media"}}"#, &layer_map())
            .expect("message parses")
            .expect("event is emitted");

        match event {
            RuntimeEvent::LayerStackChanged { layer_stack, .. } => {
                assert_eq!(layer_stack[0].layer_id, "media");
                assert_eq!(
                    layer_stack[0].confidence.level,
                    StateConfidenceLevel::Medium
                );
            }
            _ => panic!("expected layer stack event"),
        }
    }

    #[test]
    fn maps_hello_and_error_messages_to_backend_health() {
        let hello = kanata_event_from_line(
            r#"{"HelloOk":{"version":"1.11.0","protocol":1,"capabilities":["LayerChange"]}}"#,
            &layer_map(),
        )
        .expect("message parses")
        .expect("event is emitted");
        let error = kanata_event_from_line(r#"{"Error":{"msg":"bad layer"}}"#, &layer_map())
            .expect("message parses")
            .expect("event is emitted");

        match hello {
            RuntimeEvent::BackendHealthChanged { health } => {
                assert_eq!(health.backend_id, kanata_backend::KANATA_BACKEND_ID);
                assert_eq!(health.state, HealthState::Ok);
                assert!(health.message.contains("1.11.0"));
            }
            _ => panic!("expected health event"),
        }
        match error {
            RuntimeEvent::BackendHealthChanged { health } => {
                assert_eq!(health.state, HealthState::ProtocolError);
                assert_eq!(health.message, "bad layer");
            }
            _ => panic!("expected health event"),
        }
    }

    #[test]
    fn session_requests_hello_and_current_layer_then_polls_events() {
        let transport = FakeKanataTransport {
            writes: Vec::new(),
            reads: VecDeque::from([Some(r#"{"LayerChange":{"new":"Navigation"}}"#.to_string())]),
        };
        let mut session = KanataTcpSession::new(transport, layer_map());

        session.start().expect("session starts");
        let event = session
            .poll_next_event()
            .expect("poll succeeds")
            .expect("event is emitted");

        assert_eq!(
            session.transport.writes,
            vec![KANATA_HELLO_COMMAND, KANATA_CURRENT_LAYER_COMMAND]
        );
        assert!(matches!(event, RuntimeEvent::LayerStackChanged { .. }));
    }

    #[test]
    fn malformed_messages_return_parse_errors() {
        let error = kanata_event_from_line("{not-json", &layer_map()).expect_err("parse fails");

        assert!(matches!(error, KanataTcpError::Parse(_)));
    }
}
