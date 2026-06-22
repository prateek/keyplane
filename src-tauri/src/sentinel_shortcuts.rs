use crate::domain::{HealthState, SentinelKeyBinding};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::{Mutex, MutexGuard};
use tauri_plugin_global_shortcut::Shortcut;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentinelShortcutRegistration {
    pub accelerator: String,
    pub host_input_code: String,
    pub shortcut_id: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SentinelShortcutError {
    #[error("no Sentinel Key bindings are configured")]
    NoBindings,
    #[error("invalid Sentinel Key host input code {code}: {reason}")]
    InvalidHostInputCode { code: String, reason: String },
    #[error("Sentinel Key shortcut registry is unavailable")]
    RegistryUnavailable,
}

#[derive(Debug, Default)]
pub struct SentinelShortcutRuntime {
    registrations: Mutex<Vec<SentinelShortcutRegistration>>,
}

impl SentinelShortcutRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace_registered(
        &self,
        registrations: Vec<SentinelShortcutRegistration>,
    ) -> Result<(), SentinelShortcutError> {
        *self.registrations()? = registrations;
        Ok(())
    }

    pub fn clear_registered(&self) -> Result<(), SentinelShortcutError> {
        self.registrations()?.clear();
        Ok(())
    }

    pub fn registered_accelerators(&self) -> Result<Vec<String>, SentinelShortcutError> {
        Ok(self
            .registrations()?
            .iter()
            .map(|registration| registration.accelerator.clone())
            .collect())
    }

    pub fn host_input_code_for_shortcut(
        &self,
        shortcut_id: u32,
    ) -> Result<Option<String>, SentinelShortcutError> {
        Ok(self
            .registrations()?
            .iter()
            .find(|registration| registration.shortcut_id == shortcut_id)
            .map(|registration| registration.host_input_code.clone()))
    }

    fn registrations(
        &self,
    ) -> Result<MutexGuard<'_, Vec<SentinelShortcutRegistration>>, SentinelShortcutError> {
        self.registrations
            .lock()
            .map_err(|_| SentinelShortcutError::RegistryUnavailable)
    }
}

pub fn shortcut_registrations_from_bindings(
    bindings: &[SentinelKeyBinding],
) -> Result<Vec<SentinelShortcutRegistration>, SentinelShortcutError> {
    let mut registrations = BTreeMap::<String, SentinelShortcutRegistration>::new();

    for binding in bindings {
        let accelerator = accelerator_from_host_input_code(&binding.host_input_code)?;
        let shortcut = Shortcut::from_str(&accelerator).map_err(|err| {
            SentinelShortcutError::InvalidHostInputCode {
                code: binding.host_input_code.clone(),
                reason: err.to_string(),
            }
        })?;

        registrations
            .entry(accelerator.clone())
            .or_insert(SentinelShortcutRegistration {
                accelerator,
                host_input_code: binding.host_input_code.clone(),
                shortcut_id: shortcut.id(),
            });
    }

    let registrations: Vec<SentinelShortcutRegistration> = registrations.into_values().collect();
    if registrations.is_empty() {
        Err(SentinelShortcutError::NoBindings)
    } else {
        Ok(registrations)
    }
}

pub fn registration_health_state(error_message: &str) -> HealthState {
    let lower = error_message.to_ascii_lowercase();
    if lower.contains("accessibility")
        || lower.contains("input monitoring")
        || lower.contains("permission")
        || lower.contains("trusted")
    {
        HealthState::PermissionMissing
    } else {
        HealthState::ProtocolError
    }
}

pub fn registration_index(registrations: &[SentinelShortcutRegistration]) -> HashMap<u32, String> {
    registrations
        .iter()
        .map(|registration| {
            (
                registration.shortcut_id,
                registration.host_input_code.clone(),
            )
        })
        .collect()
}

fn accelerator_from_host_input_code(code: &str) -> Result<String, SentinelShortcutError> {
    let accelerator = code.trim();
    if accelerator.is_empty() {
        return Err(SentinelShortcutError::InvalidHostInputCode {
            code: code.to_string(),
            reason: "empty host input code".to_string(),
        });
    }

    Ok(accelerator.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ActivationKind;

    fn binding(code: &str) -> SentinelKeyBinding {
        SentinelKeyBinding {
            host_input_code: code.to_string(),
            layer_id: "layer-1".to_string(),
            activation: ActivationKind::Momentary,
        }
    }

    #[test]
    fn builds_shortcut_registrations_from_profile_bindings() {
        let registrations =
            shortcut_registrations_from_bindings(&[binding("F24")]).expect("valid binding");

        assert_eq!(registrations.len(), 1);
        assert_eq!(registrations[0].accelerator, "F24");
        assert_eq!(registrations[0].host_input_code, "F24");
        assert!(registration_index(&registrations).contains_key(&registrations[0].shortcut_id));
    }

    #[test]
    fn rejects_empty_sentinel_host_input_codes() {
        let error = shortcut_registrations_from_bindings(&[binding("  ")]).unwrap_err();

        assert_eq!(
            error,
            SentinelShortcutError::InvalidHostInputCode {
                code: "  ".to_string(),
                reason: "empty host input code".to_string()
            }
        );
    }

    #[test]
    fn collapses_duplicate_shortcut_registrations() {
        let registrations = shortcut_registrations_from_bindings(&[binding("F24"), binding("F24")])
            .expect("duplicate binding is registered once");

        assert_eq!(registrations.len(), 1);
    }

    #[test]
    fn maps_permission_like_errors_to_permission_missing_health() {
        assert_eq!(
            registration_health_state("Accessibility permission was denied"),
            HealthState::PermissionMissing
        );
        assert_eq!(
            registration_health_state("shortcut already registered"),
            HealthState::ProtocolError
        );
    }
}
