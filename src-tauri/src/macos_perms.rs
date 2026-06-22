//! macOS input-monitoring permission detection (ADR 0023, stories 23–24).
//!
//! Sentinel-key capture needs the Input Monitoring permission. macOS exposes
//! `CGPreflightListenEventAccess` to check the current grant without capturing
//! anything, and `CGRequestListenEventAccess` to prompt for it. Keyplane uses
//! the preflight check to report a persistent `PermissionMissing` Backend Health
//! proactively, instead of only discovering the problem when a global hook
//! fails.

#[cfg(target_os = "macos")]
mod imp {
    // These symbols live in the CoreGraphics framework, which the app already
    // links transitively through the webview stack.
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightListenEventAccess() -> bool;
        fn CGRequestListenEventAccess() -> bool;
    }

    /// Whether Input Monitoring is currently granted to this process.
    pub fn input_monitoring_granted() -> bool {
        unsafe { CGPreflightListenEventAccess() }
    }

    /// Prompt for Input Monitoring (shows the system dialog the first time).
    pub fn request_input_monitoring() -> bool {
        unsafe { CGRequestListenEventAccess() }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    /// Non-macOS platforms don't gate global key capture this way; report
    /// granted so the sentinel path is not blocked.
    pub fn input_monitoring_granted() -> bool {
        true
    }

    pub fn request_input_monitoring() -> bool {
        true
    }
}

pub use imp::{input_monitoring_granted, request_input_monitoring};

#[cfg(test)]
mod tests {
    use super::*;

    /// The FFI links and returns a real boolean without panicking. (The value
    /// reflects this test process's actual permission state.)
    #[test]
    fn preflight_check_runs() {
        let _granted: bool = input_monitoring_granted();
    }
}
