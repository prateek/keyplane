use crate::domain::{BackendHealth, BackendStatus, CapabilityFlag, HealthState};

pub const KANATA_BACKEND_ID: &str = "kanata-tcp";

pub fn kanata_backend_status(health: HealthState, message: impl Into<String>) -> BackendStatus {
    BackendStatus {
        id: KANATA_BACKEND_ID.to_string(),
        name: "Kanata TCP".to_string(),
        capabilities: vec![CapabilityFlag::StreamLayerStack, CapabilityFlag::PollState],
        health: BackendHealth {
            backend_id: KANATA_BACKEND_ID.to_string(),
            state: health,
            message: message.into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kanata_backend_status_reports_runtime_capabilities() {
        let status = kanata_backend_status(HealthState::Disconnected, "Kanata TCP unavailable");

        assert_eq!(status.id, KANATA_BACKEND_ID);
        assert!(status
            .capabilities
            .contains(&CapabilityFlag::StreamLayerStack));
        assert!(status.capabilities.contains(&CapabilityFlag::PollState));
        assert_eq!(status.health.state, HealthState::Disconnected);
    }
}
