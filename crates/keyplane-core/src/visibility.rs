//! Fade Visibility timing (ADR 0026).
//!
//! Pinned Visibility is the MVP default; Fade Visibility shows the overlay on
//! activity and hides it after an inactivity interval. The timing is pure and
//! clock-injected here (the caller passes a monotonic millisecond clock), so it
//! is deterministic and testable; the driver supplies real time and toggles the
//! window.

/// Tracks the last activity time and decides overlay visibility for the Fade
/// policy.
#[derive(Clone, Copy, Debug)]
pub struct FadeController {
    timeout_ms: u64,
    last_activity_ms: u64,
}

impl FadeController {
    /// Create a controller that hides the overlay `timeout_ms` after the last
    /// activity. Starts active at time 0.
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            timeout_ms,
            last_activity_ms: 0,
        }
    }

    /// Record activity (e.g. a Runtime Event) at `now_ms`.
    pub fn on_activity(&mut self, now_ms: u64) {
        self.last_activity_ms = now_ms;
    }

    /// Whether the overlay should be visible at `now_ms`.
    pub fn visible(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.last_activity_ms) <= self.timeout_ms
    }
}
