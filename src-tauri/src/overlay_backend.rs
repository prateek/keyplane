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
        ],
        health: BackendHealth {
            backend_id: OVERLAY_WINDOW_BACKEND_ID.to_string(),
            state: health,
            message: message.into(),
        },
    }
}
