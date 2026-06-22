use crate::domain::{
    BackendStatus, HealthState, HostInputEvent, KeyboardSnapshot, RuntimeEvent, SentinelKeyBinding,
};
use crate::{fake_backend, keypeek_backend, sentinel_backend};
use std::sync::Mutex;

pub trait ProtocolBackend {
    fn status(&self) -> BackendStatus;
    fn initial_snapshot(&self) -> Option<KeyboardSnapshot>;
    fn demo_events(&self) -> Vec<RuntimeEvent> {
        Vec::new()
    }
    fn ingest_packet(&self, _packet: &[u8]) -> Option<RuntimeEvent> {
        None
    }
    fn ingest_host_input_event(&self, _event: &HostInputEvent) -> Option<RuntimeEvent> {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct FakeProtocolBackend;

impl ProtocolBackend for FakeProtocolBackend {
    fn status(&self) -> BackendStatus {
        fake_backend::fake_profile().runtime_backends[0].clone()
    }

    fn initial_snapshot(&self) -> Option<KeyboardSnapshot> {
        Some(fake_backend::initial_snapshot())
    }

    fn demo_events(&self) -> Vec<RuntimeEvent> {
        fake_backend::demo_runtime_events()
    }
}

#[derive(Debug, Clone)]
pub struct KeyPeekPacketBackend {
    layer_count: usize,
    status: BackendStatus,
}

impl KeyPeekPacketBackend {
    pub fn disconnected(layer_count: usize) -> Self {
        Self {
            layer_count,
            status: keypeek_backend::keypeek_backend_status(
                crate::domain::HealthState::Disconnected,
                "No KeyPeek-compatible device is connected",
            ),
        }
    }
}

impl ProtocolBackend for KeyPeekPacketBackend {
    fn status(&self) -> BackendStatus {
        self.status.clone()
    }

    fn initial_snapshot(&self) -> Option<KeyboardSnapshot> {
        None
    }

    fn ingest_packet(&self, packet: &[u8]) -> Option<RuntimeEvent> {
        keypeek_backend::runtime_event_from_keypeek_layer_packet(packet, self.layer_count)
    }
}

pub struct SentinelKeyProtocolBackend {
    base_layer_id: String,
    bindings: Vec<SentinelKeyBinding>,
    active_layers: Mutex<Vec<String>>,
    status: BackendStatus,
}

impl SentinelKeyProtocolBackend {
    pub fn permission_missing(
        base_layer_id: impl Into<String>,
        bindings: Vec<SentinelKeyBinding>,
    ) -> Self {
        Self {
            base_layer_id: base_layer_id.into(),
            bindings,
            active_layers: Mutex::new(Vec::new()),
            status: sentinel_backend::sentinel_backend_status(
                HealthState::PermissionMissing,
                "Input monitoring permission is required before Sentinel Keys can infer layers",
            ),
        }
    }
}

impl ProtocolBackend for SentinelKeyProtocolBackend {
    fn status(&self) -> BackendStatus {
        self.status.clone()
    }

    fn initial_snapshot(&self) -> Option<KeyboardSnapshot> {
        None
    }

    fn ingest_host_input_event(&self, event: &HostInputEvent) -> Option<RuntimeEvent> {
        let mut active_layers = self.active_layers.lock().ok()?;
        sentinel_backend::runtime_event_from_host_input_event(
            &mut active_layers,
            &self.bindings,
            &self.base_layer_id,
            event,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypeek_backend::LAYER_STATE_PACKET_MARKER;

    #[test]
    fn fake_backend_uses_the_protocol_backend_boundary() {
        let backend = FakeProtocolBackend;
        let snapshot = backend.initial_snapshot().expect("fake snapshot exists");

        assert_eq!(backend.status().id, "fake-backend");
        assert_eq!(snapshot.profile_id, "profile-keyplane-demo");
        assert!(!backend.demo_events().is_empty());
    }

    #[test]
    fn keypeek_packet_backend_emits_runtime_events_without_hardware() {
        let backend = KeyPeekPacketBackend::disconnected(4);
        let event = backend
            .ingest_packet(&[LAYER_STATE_PACKET_MARKER, 1, 1, 0b0000_0100])
            .expect("layer event");

        match event {
            RuntimeEvent::LayerStackChanged { layer_stack } => {
                assert_eq!(layer_stack[0].layer_id, "layer-2");
                assert_eq!(layer_stack[1].layer_id, "layer-0");
            }
            _ => panic!("expected layer stack event"),
        }
    }

    #[test]
    fn sentinel_key_protocol_backend_maps_host_input_events_without_os_input() {
        let backend = SentinelKeyProtocolBackend::permission_missing(
            "layer-0",
            vec![SentinelKeyBinding {
                host_input_code: "F24".to_string(),
                layer_id: "layer-1".to_string(),
                activation: crate::domain::ActivationKind::Momentary,
            }],
        );

        let event = backend
            .ingest_host_input_event(&HostInputEvent {
                code: "F24".to_string(),
                pressed: true,
            })
            .expect("host input maps to layer event");

        assert_eq!(backend.status().id, sentinel_backend::SENTINEL_BACKEND_ID);
        match event {
            RuntimeEvent::LayerStackChanged { layer_stack } => {
                assert_eq!(layer_stack[0].layer_id, "layer-1");
                assert_eq!(layer_stack[1].layer_id, "layer-0");
            }
            _ => panic!("expected layer stack event"),
        }
    }
}
