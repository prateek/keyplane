use crate::domain::{BackendStatus, HealthState};
use crate::sentinel_backend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostPermissionState {
    pub accessibility_trusted: bool,
    pub input_monitoring_trusted: bool,
}

pub fn current_host_permission_state() -> HostPermissionState {
    platform_current_host_permission_state()
}

pub fn request_host_input_permissions() -> HostPermissionState {
    platform_request_host_input_permissions()
}

pub fn host_permission_backend_status(state: HostPermissionState) -> BackendStatus {
    let missing = missing_permissions(state);
    if missing.is_empty() {
        return sentinel_backend::sentinel_backend_status(
            HealthState::Ok,
            "Host Input Event permissions are available for Sentinel Keys",
        );
    }

    sentinel_backend::sentinel_backend_status(
        HealthState::PermissionMissing,
        format!(
            "{} {} missing for Sentinel Keys",
            missing.join(" and "),
            if missing.len() == 1 {
                "permission is"
            } else {
                "permissions are"
            }
        ),
    )
}

fn missing_permissions(state: HostPermissionState) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !state.accessibility_trusted {
        missing.push("macOS Accessibility");
    }
    if !state.input_monitoring_trusted {
        missing.push("Input Monitoring");
    }
    missing
}

#[cfg(target_os = "macos")]
fn platform_current_host_permission_state() -> HostPermissionState {
    HostPermissionState {
        accessibility_trusted: macos::accessibility_trusted(),
        input_monitoring_trusted: macos::input_monitoring_trusted(),
    }
}

#[cfg(target_os = "macos")]
fn platform_request_host_input_permissions() -> HostPermissionState {
    HostPermissionState {
        accessibility_trusted: macos::request_accessibility_trust(),
        input_monitoring_trusted: macos::request_input_monitoring_trust(),
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_current_host_permission_state() -> HostPermissionState {
    HostPermissionState {
        accessibility_trusted: true,
        input_monitoring_trusted: true,
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_request_host_input_permissions() -> HostPermissionState {
    platform_current_host_permission_state()
}

#[cfg(target_os = "macos")]
mod macos {
    use core::ffi::c_void;
    use std::ptr;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        static kAXTrustedCheckOptionPrompt: *const c_void;

        fn AXIsProcessTrusted() -> u8;
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFBooleanTrue: *const c_void;

        fn CFDictionaryCreate(
            allocator: *const c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> *const c_void;
        fn CFRelease(cf: *const c_void);
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightListenEventAccess() -> bool;
        fn CGRequestListenEventAccess() -> bool;
    }

    pub fn accessibility_trusted() -> bool {
        unsafe { AXIsProcessTrusted() != 0 }
    }

    pub fn request_accessibility_trust() -> bool {
        unsafe {
            let keys = [kAXTrustedCheckOptionPrompt];
            let values = [kCFBooleanTrue];
            let options = CFDictionaryCreate(
                ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                ptr::null(),
                ptr::null(),
            );
            let trusted = AXIsProcessTrustedWithOptions(options) != 0;
            if !options.is_null() {
                CFRelease(options);
            }
            trusted
        }
    }

    pub fn input_monitoring_trusted() -> bool {
        unsafe { CGPreflightListenEventAccess() }
    }

    pub fn request_input_monitoring_trust() -> bool {
        unsafe { CGRequestListenEventAccess() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_backend_status_is_ok_when_host_permissions_are_available() {
        let status = host_permission_backend_status(HostPermissionState {
            accessibility_trusted: true,
            input_monitoring_trusted: true,
        });

        assert_eq!(status.id, sentinel_backend::SENTINEL_BACKEND_ID);
        assert_eq!(status.health.state, HealthState::Ok);
        assert!(status.health.message.contains("Host Input Event"));
    }

    #[test]
    fn permission_backend_status_lists_missing_macos_permissions() {
        let status = host_permission_backend_status(HostPermissionState {
            accessibility_trusted: false,
            input_monitoring_trusted: false,
        });

        assert_eq!(status.health.state, HealthState::PermissionMissing);
        assert!(status.health.message.contains("macOS Accessibility"));
        assert!(status.health.message.contains("Input Monitoring"));
    }
}
