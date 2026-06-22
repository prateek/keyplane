use crate::domain::{BackendHealth, BackendStatus, CapabilityFlag, HealthState};

pub const OVERLAY_WINDOW_BACKEND_ID: &str = "overlay-window";

pub fn overlay_window_backend_status(
    health: HealthState,
    message: impl Into<String>,
) -> BackendStatus {
    BackendStatus {
        id: OVERLAY_WINDOW_BACKEND_ID.to_string(),
        name: "Overlay Window".to_string(),
        capabilities: vec![
            CapabilityFlag::RenderOverlayWindow,
            CapabilityFlag::TransparentOverlayWindow,
            CapabilityFlag::ClickThroughOverlayWindow,
            CapabilityFlag::PositionOverlayWindow,
            CapabilityFlag::AllWorkspacesOverlayWindow,
        ],
        health: BackendHealth {
            backend_id: OVERLAY_WINDOW_BACKEND_ID.to_string(),
            state: health,
            message: message.into(),
        },
        config: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_window_backend_status_reports_window_capabilities() {
        let status = overlay_window_backend_status(HealthState::Ok, "ready");

        assert!(status
            .capabilities
            .contains(&CapabilityFlag::RenderOverlayWindow));
        assert!(status
            .capabilities
            .contains(&CapabilityFlag::TransparentOverlayWindow));
        assert!(status
            .capabilities
            .contains(&CapabilityFlag::ClickThroughOverlayWindow));
        assert!(status
            .capabilities
            .contains(&CapabilityFlag::PositionOverlayWindow));
        assert!(status
            .capabilities
            .contains(&CapabilityFlag::AllWorkspacesOverlayWindow));
    }
}
