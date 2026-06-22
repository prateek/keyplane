use crate::domain::{BackendStatus, KeyboardSnapshot, RuntimeEvent};
use crate::{fake_backend, keypeek_backend};

pub trait ProtocolBackend {
    fn status(&self) -> BackendStatus;
    fn initial_snapshot(&self) -> Option<KeyboardSnapshot>;
    fn demo_events(&self) -> Vec<RuntimeEvent> {
        Vec::new()
    }
    fn ingest_packet(&self, _packet: &[u8]) -> Option<RuntimeEvent> {
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
}
