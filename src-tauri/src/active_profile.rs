use crate::domain::{
    compose_snapshot, promote_conflict_to_override, ActivationKind, BackendStatus, CapabilityFlag,
    HealthState, HostInputEvent, ImportCandidate, KeyboardSnapshot, LayerActivation, Profile,
    RuntimeEvent, RuntimeState, SourceConflict, StateConfidence, StateConfidenceLevel,
    StyleDensity, VisibilityPolicy,
};
use crate::profile_codec;
use crate::sentinel_backend;
use std::sync::{Mutex, MutexGuard};
use thiserror::Error;

#[derive(Debug)]
pub struct ActiveProfileStore {
    profile: Mutex<Profile>,
    source_conflicts: Mutex<Vec<SourceConflict>>,
    sentinel_active_layers: Mutex<Vec<String>>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActiveProfileError {
    #[error("active profile state is unavailable")]
    StateUnavailable,
    #[error("source candidate {source_id} is not present for {field_path}")]
    MissingSourceCandidate {
        field_path: String,
        source_id: String,
    },
}

impl ActiveProfileStore {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile: Mutex::new(profile),
            source_conflicts: Mutex::new(Vec::new()),
            sentinel_active_layers: Mutex::new(Vec::new()),
        }
    }

    pub fn snapshot(&self) -> Result<KeyboardSnapshot, ActiveProfileError> {
        let profile = self.profile()?.clone();
        let source_conflicts = self.source_conflicts()?.clone();
        Ok(snapshot_from_profile(&profile, source_conflicts))
    }

    pub fn profile_snapshot(&self) -> Result<Profile, ActiveProfileError> {
        Ok(self.profile()?.clone())
    }

    pub fn commit_import_candidate(
        &self,
        candidate: ImportCandidate,
    ) -> Result<KeyboardSnapshot, ActiveProfileError> {
        self.replace_profile(candidate.preview_profile, candidate.conflicts)
    }

    pub fn load_profile(&self, profile: Profile) -> Result<KeyboardSnapshot, ActiveProfileError> {
        self.replace_profile(profile, Vec::new())
    }

    pub fn set_overlay_positioning_mode(
        &self,
        enabled: bool,
    ) -> Result<KeyboardSnapshot, ActiveProfileError> {
        let profile = {
            let mut profile = self.profile()?;
            profile.overlay_window.positioning_mode = enabled;
            profile.overlay_window.click_through = !enabled;
            profile.clone()
        };
        let source_conflicts = self.source_conflicts()?.clone();

        Ok(snapshot_from_profile(&profile, source_conflicts))
    }

    pub fn set_overlay_visibility_policy(
        &self,
        visibility: VisibilityPolicy,
    ) -> Result<KeyboardSnapshot, ActiveProfileError> {
        let profile = {
            let mut profile = self.profile()?;
            profile.overlay_window.visibility = visibility.clone();
            if matches!(
                visibility,
                VisibilityPolicy::Pinned | VisibilityPolicy::Fade
            ) {
                profile.overlay_window.visible = true;
            }
            profile.clone()
        };
        let source_conflicts = self.source_conflicts()?.clone();

        Ok(snapshot_from_profile(&profile, source_conflicts))
    }

    pub fn set_overlay_visible(
        &self,
        visible: bool,
    ) -> Result<KeyboardSnapshot, ActiveProfileError> {
        let profile = {
            let mut profile = self.profile()?;
            profile.overlay_window.visible = visible;
            if !visible {
                profile.overlay_window.positioning_mode = false;
                profile.overlay_window.click_through = true;
            }
            profile.clone()
        };
        let source_conflicts = self.source_conflicts()?.clone();

        Ok(snapshot_from_profile(&profile, source_conflicts))
    }

    pub fn set_visual_style_density(
        &self,
        density: StyleDensity,
    ) -> Result<KeyboardSnapshot, ActiveProfileError> {
        let profile = {
            let mut profile = self.profile()?;
            profile.visual_style.density = density;
            profile.clone()
        };
        let source_conflicts = self.source_conflicts()?.clone();

        Ok(snapshot_from_profile(&profile, source_conflicts))
    }

    pub fn ingest_sentinel_host_input_event(
        &self,
        event: HostInputEvent,
    ) -> Result<Option<RuntimeEvent>, ActiveProfileError> {
        let profile = self.profile()?.clone();
        let base_layer_id = profile
            .keymap
            .layers
            .first()
            .map(|layer| layer.id.as_str())
            .unwrap_or("layer-0");
        let mut active_layers = self.sentinel_active_layers()?;

        Ok(sentinel_backend::runtime_event_from_host_input_event(
            &mut active_layers,
            &profile.sentinel_keys,
            base_layer_id,
            &event,
        ))
    }

    pub fn save_profile_edn(&self) -> Result<String, ActiveProfileError> {
        let profile = self.profile()?.clone();
        Ok(profile_codec::save_profile(&profile))
    }

    pub fn set_runtime_backend_status(
        &self,
        mut status: BackendStatus,
    ) -> Result<KeyboardSnapshot, ActiveProfileError> {
        let profile = {
            let mut profile = self.profile()?;
            if let Some(existing) = profile
                .runtime_backends
                .iter_mut()
                .find(|backend| backend.id == status.id)
            {
                if status.config.is_none() {
                    status.config = existing.config.clone();
                }
                *existing = status;
            } else {
                profile.runtime_backends.push(status);
            }
            profile.clone()
        };
        let source_conflicts = self.source_conflicts()?.clone();

        Ok(snapshot_from_profile(&profile, source_conflicts))
    }

    pub fn promote_source_candidate(
        &self,
        conflict: SourceConflict,
        source_id: &str,
    ) -> Result<KeyboardSnapshot, ActiveProfileError> {
        let profile = {
            let mut profile = self.profile()?;
            promote_conflict_to_override(&mut profile, &conflict, source_id).ok_or_else(|| {
                ActiveProfileError::MissingSourceCandidate {
                    field_path: conflict.field_path.clone(),
                    source_id: source_id.to_string(),
                }
            })?;
            profile.clone()
        };
        let source_conflicts = {
            let mut source_conflicts = self.source_conflicts()?;
            upsert_source_conflict(&mut source_conflicts, conflict);
            source_conflicts.clone()
        };

        Ok(snapshot_from_profile(&profile, source_conflicts))
    }

    fn replace_profile(
        &self,
        profile: Profile,
        source_conflicts: Vec<SourceConflict>,
    ) -> Result<KeyboardSnapshot, ActiveProfileError> {
        {
            let mut active_profile = self.profile()?;
            *active_profile = profile.clone();
        }
        {
            let mut active_conflicts = self.source_conflicts()?;
            *active_conflicts = source_conflicts.clone();
        }
        {
            let mut sentinel_active_layers = self.sentinel_active_layers()?;
            sentinel_active_layers.clear();
        }

        Ok(snapshot_from_profile(&profile, source_conflicts))
    }

    fn profile(&self) -> Result<MutexGuard<'_, Profile>, ActiveProfileError> {
        self.profile
            .lock()
            .map_err(|_| ActiveProfileError::StateUnavailable)
    }

    fn source_conflicts(&self) -> Result<MutexGuard<'_, Vec<SourceConflict>>, ActiveProfileError> {
        self.source_conflicts
            .lock()
            .map_err(|_| ActiveProfileError::StateUnavailable)
    }

    fn sentinel_active_layers(&self) -> Result<MutexGuard<'_, Vec<String>>, ActiveProfileError> {
        self.sentinel_active_layers
            .lock()
            .map_err(|_| ActiveProfileError::StateUnavailable)
    }
}

fn upsert_source_conflict(source_conflicts: &mut Vec<SourceConflict>, conflict: SourceConflict) {
    if let Some(existing) = source_conflicts
        .iter_mut()
        .find(|candidate| candidate.field_path == conflict.field_path)
    {
        *existing = conflict;
    } else {
        source_conflicts.push(conflict);
    }
}

fn snapshot_from_profile(
    profile: &Profile,
    source_conflicts: Vec<SourceConflict>,
) -> KeyboardSnapshot {
    compose_snapshot(
        profile,
        runtime_state_for_profile(profile),
        source_conflicts,
    )
}

fn runtime_state_for_profile(profile: &Profile) -> RuntimeState {
    RuntimeState {
        layer_stack: profile
            .keymap
            .layers
            .first()
            .map(|layer| LayerActivation {
                layer_id: layer.id.clone(),
                kind: ActivationKind::Default,
                confidence: state_confidence_for_profile(profile),
            })
            .into_iter()
            .collect(),
        layer_stack_source_id: None,
        pressed_keys: Vec::new(),
        pressed_keys_source_id: None,
        backend_health: profile
            .runtime_backends
            .iter()
            .map(|backend| backend.health.clone())
            .collect(),
    }
}

fn state_confidence_for_profile(profile: &Profile) -> StateConfidence {
    if profile.runtime_backends.is_empty() {
        return StateConfidence {
            level: StateConfidenceLevel::Low,
            reason: "Active Profile has no runtime backend".to_string(),
        };
    }

    let has_authoritative_runtime = profile.runtime_backends.iter().any(|backend| {
        backend.health.state == HealthState::Ok
            && !backend.capabilities.contains(&CapabilityFlag::PreviewOnly)
            && (backend
                .capabilities
                .contains(&CapabilityFlag::StreamLayerStack)
                || backend.capabilities.contains(&CapabilityFlag::PollState))
    });

    if has_authoritative_runtime {
        StateConfidence {
            level: StateConfidenceLevel::High,
            reason: "Active Profile default layer".to_string(),
        }
    } else {
        StateConfidence {
            level: StateConfidenceLevel::Medium,
            reason: "Best-Effort Preview default layer".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BackendConfig, SourceCandidate, SourceConflict};

    fn visual_style_conflict(selected_source_id: &str) -> SourceConflict {
        SourceConflict {
            field_path: ":visual/style :style/variant-id".to_string(),
            selected_source_id: selected_source_id.to_string(),
            candidates: vec![
                SourceCandidate {
                    source_id: "fake-backend".to_string(),
                    value: "keyplane-default".to_string(),
                    selected: selected_source_id == "fake-backend",
                },
                SourceCandidate {
                    source_id: "keyviz-import".to_string(),
                    value: "keyviz-minimal".to_string(),
                    selected: selected_source_id == "keyviz-import",
                },
            ],
        }
    }

    #[test]
    fn active_profile_snapshot_composes_from_the_current_profile() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        let snapshot = store.snapshot().expect("snapshot composes");

        assert_eq!(snapshot.profile_name, "Keyplane Demo");
        assert_eq!(snapshot.keyboard_id, "keyboard-keyplane-demo");
        assert_eq!(snapshot.runtime_state.layer_stack[0].layer_id, "layer-0");
    }

    #[test]
    fn committing_import_candidate_replaces_the_active_profile() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        let mut imported_profile = crate::fake_backend::fake_profile();
        imported_profile.id = "profile-imported".to_string();
        imported_profile.keyboard_id = "keyboard-imported".to_string();
        imported_profile.name = "Imported Profile".to_string();
        let candidate = crate::fake_backend::import_candidate_from_profile(imported_profile);

        let snapshot = store
            .commit_import_candidate(candidate)
            .expect("candidate commits");

        assert_eq!(snapshot.profile_id, "profile-imported");
        assert_eq!(snapshot.keyboard_id, "keyboard-imported");
        assert_eq!(snapshot.profile_name, "Imported Profile");
        assert_eq!(store.snapshot().unwrap().profile_id, "profile-imported");
    }

    #[test]
    fn committed_preview_only_profile_uses_medium_state_confidence() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        let mut imported_profile = crate::fake_backend::fake_profile();
        imported_profile.runtime_backends[0].capabilities = vec![CapabilityFlag::PreviewOnly];
        imported_profile.runtime_backends[0].health.state = HealthState::Stale;
        let candidate = crate::fake_backend::import_candidate_from_profile(imported_profile);

        let snapshot = store
            .commit_import_candidate(candidate)
            .expect("candidate commits");

        assert_eq!(
            snapshot.runtime_state.layer_stack[0].confidence.level,
            StateConfidenceLevel::Medium
        );
        assert_eq!(
            snapshot.runtime_state.layer_stack[0].confidence.reason,
            "Best-Effort Preview default layer"
        );
    }

    #[test]
    fn promoted_source_candidate_updates_active_profile_overrides() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());

        let snapshot = store
            .promote_source_candidate(visual_style_conflict("fake-backend"), "keyviz-import")
            .expect("candidate promotes");

        assert_eq!(snapshot.user_overrides[0].value, "keyviz-minimal");
        assert_eq!(
            snapshot.source_conflicts[0].selected_source_id,
            "user-overrides"
        );
        assert_eq!(snapshot.visual_style.variant_id, "keyviz-minimal");
    }

    #[test]
    fn positioning_mode_updates_the_active_profile_overlay_state() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());

        let positioned = store
            .set_overlay_positioning_mode(true)
            .expect("positioning mode enables");

        assert!(positioned.overlay_window.positioning_mode);
        assert!(!positioned.overlay_window.click_through);

        let saved = store.save_profile_edn().expect("profile saves");
        let loaded = profile_codec::load_profile(&saved).expect("saved profile loads");
        assert!(loaded.overlay_window.positioning_mode);
        assert!(!loaded.overlay_window.click_through);

        let locked = store
            .set_overlay_positioning_mode(false)
            .expect("positioning mode disables");
        assert!(!locked.overlay_window.positioning_mode);
        assert!(locked.overlay_window.click_through);
    }

    #[test]
    fn visual_style_density_updates_the_active_profile() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());

        let compact = store
            .set_visual_style_density(StyleDensity::Compact)
            .expect("density updates");

        assert_eq!(compact.visual_style.density, StyleDensity::Compact);

        let saved = store.save_profile_edn().expect("profile saves");
        let loaded = profile_codec::load_profile(&saved).expect("saved profile loads");
        assert_eq!(loaded.visual_style.density, StyleDensity::Compact);
    }

    #[test]
    fn overlay_visibility_policy_updates_the_active_profile() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());

        let manual = store
            .set_overlay_visibility_policy(VisibilityPolicy::ManualToggle)
            .expect("visibility policy updates");

        assert_eq!(
            manual.overlay_window.visibility,
            VisibilityPolicy::ManualToggle
        );
        assert!(manual.overlay_window.visible);

        let saved = store.save_profile_edn().expect("profile saves");
        let loaded = profile_codec::load_profile(&saved).expect("saved profile loads");
        assert_eq!(
            loaded.overlay_window.visibility,
            VisibilityPolicy::ManualToggle
        );
        assert!(loaded.overlay_window.visible);
    }

    #[test]
    fn overlay_visible_state_updates_the_active_profile() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        store
            .set_overlay_positioning_mode(true)
            .expect("positioning mode enables");

        let hidden = store
            .set_overlay_visible(false)
            .expect("overlay visible state updates");

        assert!(!hidden.overlay_window.visible);
        assert!(!hidden.overlay_window.positioning_mode);
        assert!(hidden.overlay_window.click_through);

        let saved = store.save_profile_edn().expect("profile saves");
        let loaded = profile_codec::load_profile(&saved).expect("saved profile loads");
        assert!(!loaded.overlay_window.visible);
    }

    #[test]
    fn sentinel_host_input_event_uses_profile_bindings_with_low_confidence() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());

        let event = store
            .ingest_sentinel_host_input_event(HostInputEvent {
                code: "F24".to_string(),
                pressed: true,
            })
            .expect("sentinel event ingests")
            .expect("binding matches");

        match event {
            RuntimeEvent::LayerStackChanged { layer_stack, .. } => {
                assert_eq!(layer_stack[0].layer_id, "layer-1");
                assert_eq!(layer_stack[0].kind, ActivationKind::Momentary);
                assert_eq!(layer_stack[0].confidence.level, StateConfidenceLevel::Low);
                assert_eq!(layer_stack[1].layer_id, "layer-0");
            }
            _ => panic!("expected layer stack event"),
        }
    }

    #[test]
    fn runtime_backend_status_updates_the_active_profile_snapshot() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        let status = crate::keypeek_backend::keypeek_backend_status(
            HealthState::Ok,
            "Connected to KeyPeek-compatible HID feed:cafe",
        );

        let snapshot = store
            .set_runtime_backend_status(status)
            .expect("backend status updates");

        let health = snapshot
            .runtime_state
            .backend_health
            .iter()
            .find(|candidate| candidate.backend_id == "keypeek-live")
            .expect("keypeek health exists");
        assert_eq!(health.state, HealthState::Ok);
        assert_eq!(
            snapshot.runtime_state.layer_stack[0].confidence.level,
            StateConfidenceLevel::High
        );
        assert_eq!(
            store
                .profile_snapshot()
                .unwrap()
                .runtime_backends
                .iter()
                .find(|backend| backend.id == "keypeek-live")
                .unwrap()
                .health
                .state,
            HealthState::Ok
        );
    }

    #[test]
    fn runtime_backend_status_preserves_profile_owned_backend_config() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        let status = crate::kanata_backend::kanata_backend_status(
            HealthState::Ok,
            "Connected to Kanata TCP",
        );

        let snapshot = store
            .set_runtime_backend_status(status)
            .expect("backend status updates");

        assert_eq!(
            snapshot
                .backends
                .iter()
                .find(|backend| backend.id == crate::kanata_backend::KANATA_BACKEND_ID)
                .and_then(|backend| backend.config.clone()),
            Some(BackendConfig::KanataTcp {
                host: "127.0.0.1".to_string(),
                port: 7070,
            })
        );
    }

    #[test]
    fn replacing_active_profile_clears_sentinel_runtime_state() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        assert!(store
            .ingest_sentinel_host_input_event(HostInputEvent {
                code: "F24".to_string(),
                pressed: true,
            })
            .expect("sentinel event ingests")
            .is_some());

        let mut imported_profile = crate::fake_backend::fake_profile();
        imported_profile.id = "profile-replaced".to_string();
        store
            .load_profile(imported_profile)
            .expect("profile replacement succeeds");

        let release_after_reset = store
            .ingest_sentinel_host_input_event(HostInputEvent {
                code: "F24".to_string(),
                pressed: false,
            })
            .expect("sentinel event ingests")
            .expect("binding still matches");

        match release_after_reset {
            RuntimeEvent::LayerStackChanged { layer_stack, .. } => {
                assert_eq!(layer_stack.len(), 1);
                assert_eq!(layer_stack[0].layer_id, "layer-0");
            }
            _ => panic!("expected layer stack event"),
        }
    }

    #[test]
    fn active_profile_save_uses_the_profile_codec() {
        let store = ActiveProfileStore::new(crate::fake_backend::fake_profile());
        let saved = store.save_profile_edn().expect("profile saves");

        assert!(saved.contains(":schema/version 1"));
        assert!(saved.contains(":profile/id \"profile-keyplane-demo\""));
        assert!(saved.contains(":keyboard/id \"keyboard-keyplane-demo\""));
    }
}
