// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Tactical Rust/WASM Boundary Defense.
// See LICENSE and IP_NOTICE.md for details.

//! Threat taxonomy (TZ §3.4) — the tactical identity of every boundary event.
//!
//! Each scheduled `NeuralEvent` projects onto a `ThreatKind` that carries its
//! meaning, primary risk, and the deterministic correct counterplay. The core
//! is authoritative: counterplay correctness is decided here, never in JS.

use crate::actions::PlayerAction;
use crate::events::EventKind;
use crate::hash::Fnv64;

/// The twelve tactical threats plus the `None` sentinel (TZ §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ThreatKind {
    None = 0,
    RawSignalExposure = 1,
    ConsentDrift = 2,
    PermissionEscalation = 3,
    VaultLeak = 4,
    StimulationSpike = 5,
    LatencyStorm = 6,
    ShadowAppProbe = 7,
    ReplayTamper = 8,
    SensorNoiseBurst = 9,
    PolicyConflict = 10,
    CognitiveOverload = 11,
    EmergencyOverride = 12,
}

impl ThreatKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::None,
            1 => Self::RawSignalExposure,
            2 => Self::ConsentDrift,
            3 => Self::PermissionEscalation,
            4 => Self::VaultLeak,
            5 => Self::StimulationSpike,
            6 => Self::LatencyStorm,
            7 => Self::ShadowAppProbe,
            8 => Self::ReplayTamper,
            9 => Self::SensorNoiseBurst,
            10 => Self::PolicyConflict,
            11 => Self::CognitiveOverload,
            12 => Self::EmergencyOverride,
            _ => return None,
        })
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Stable machine label (SCREAMING_SNAKE), used in vectors and logs.
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::RawSignalExposure => "RAW_SIGNAL_EXPOSURE",
            Self::ConsentDrift => "CONSENT_DRIFT",
            Self::PermissionEscalation => "PERMISSION_ESCALATION",
            Self::VaultLeak => "VAULT_LEAK",
            Self::StimulationSpike => "STIMULATION_SPIKE",
            Self::LatencyStorm => "LATENCY_STORM",
            Self::ShadowAppProbe => "SHADOW_APP_PROBE",
            Self::ReplayTamper => "REPLAY_TAMPER",
            Self::SensorNoiseBurst => "SENSOR_NOISE_BURST",
            Self::PolicyConflict => "POLICY_CONFLICT",
            Self::CognitiveOverload => "COGNITIVE_OVERLOAD",
            Self::EmergencyOverride => "EMERGENCY_OVERRIDE",
        }
    }

    /// Player-facing title for the threat card (TZ §3.4 / §4.1).
    pub const fn title(self) -> &'static str {
        match self {
            Self::None => "Boundary Stable",
            Self::RawSignalExposure => "Raw Signal Exposure",
            Self::ConsentDrift => "Consent Drift",
            Self::PermissionEscalation => "Permission Escalation",
            Self::VaultLeak => "Vault Leak",
            Self::StimulationSpike => "Stimulation Spike",
            Self::LatencyStorm => "Latency Storm",
            Self::ShadowAppProbe => "Shadow App Probe",
            Self::ReplayTamper => "Replay Tamper",
            Self::SensorNoiseBurst => "Sensor Noise Burst",
            Self::PolicyConflict => "Policy Conflict",
            Self::CognitiveOverload => "Cognitive Overload",
            Self::EmergencyOverride => "Emergency Override",
        }
    }

    /// Primary risk descriptor shown under the threat card (TZ §3.4).
    pub const fn primary_risk(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::RawSignalExposure => "raw leak",
            Self::ConsentDrift => "consent collapse",
            Self::PermissionEscalation => "policy breach",
            Self::VaultLeak => "vault failure",
            Self::StimulationSpike => "stimulation escape",
            Self::LatencyStorm => "delayed mitigation",
            Self::ShadowAppProbe => "permission breach",
            Self::ReplayTamper => "integrity failure",
            Self::SensorNoiseBurst => "accuracy loss",
            Self::PolicyConflict => "trust loss",
            Self::CognitiveOverload => "performance collapse",
            Self::EmergencyOverride => "multi-risk",
        }
    }

    /// The single best counter action (TZ §3.4, left side of each combo).
    pub const fn best_action(self) -> PlayerAction {
        match self {
            Self::None => PlayerAction::None,
            Self::RawSignalExposure => PlayerAction::Quarantine,
            Self::ConsentDrift => PlayerAction::Audit,
            Self::PermissionEscalation => PlayerAction::Audit,
            Self::VaultLeak => PlayerAction::SealVault,
            Self::StimulationSpike => PlayerAction::Throttle,
            Self::LatencyStorm => PlayerAction::Throttle,
            Self::ShadowAppProbe => PlayerAction::Revoke,
            Self::ReplayTamper => PlayerAction::Audit,
            Self::SensorNoiseBurst => PlayerAction::Audit,
            Self::PolicyConflict => PlayerAction::Audit,
            Self::CognitiveOverload => PlayerAction::Throttle,
            Self::EmergencyOverride => PlayerAction::Quarantine,
        }
    }

    /// The follow-up action that completes the counter combo, if any (TZ §3.4).
    pub const fn follow_action(self) -> Option<PlayerAction> {
        match self {
            Self::RawSignalExposure => Some(PlayerAction::SealVault),
            Self::ConsentDrift => Some(PlayerAction::Revoke),
            Self::PermissionEscalation => Some(PlayerAction::Authorize),
            Self::ShadowAppProbe => Some(PlayerAction::Quarantine),
            Self::PolicyConflict => Some(PlayerAction::Revoke),
            Self::EmergencyOverride => Some(PlayerAction::SealVault),
            _ => None,
        }
    }

    /// Classify an action against this threat (TZ §3.5: good / partial / bad).
    /// Drives `ActionReason` and tactical scoring; fully deterministic.
    pub fn rate_action(self, action: PlayerAction) -> CounterRating {
        if action == PlayerAction::None {
            return CounterRating::Neutral;
        }
        if action == self.best_action() {
            return CounterRating::Correct;
        }
        if let Some(follow) = self.follow_action() {
            if action == follow {
                return CounterRating::Partial;
            }
        }
        CounterRating::Wrong
    }

    /// Critical threats end the run badly if they leak past expiry (TZ §3.4).
    pub const fn is_critical(self) -> bool {
        matches!(
            self,
            Self::RawSignalExposure
                | Self::VaultLeak
                | Self::StimulationSpike
                | Self::PermissionEscalation
                | Self::EmergencyOverride
        )
    }

    /// Map a scheduled `NeuralEvent` kind to its tactical threat identity.
    /// Threats with no current event source (e.g. `CognitiveOverload`) are
    /// introduced by scenario expansion in a later phase.
    pub const fn from_event(kind: EventKind) -> Self {
        match kind {
            EventKind::BenignFlow => Self::None,
            EventKind::ConsentRequest => Self::ConsentDrift,
            EventKind::PermissionEscalation => Self::PermissionEscalation,
            EventKind::RawSignalExposure => Self::RawSignalExposure,
            EventKind::StimulationRequest => Self::StimulationSpike,
            EventKind::UnsafeStimulation => Self::StimulationSpike,
            EventKind::LatencySpike => Self::LatencyStorm,
            EventKind::AdversarialProbe => Self::ShadowAppProbe,
            EventKind::VaultPressure => Self::VaultLeak,
            EventKind::AuditCheckpoint => Self::ReplayTamper,
        }
    }

    pub fn feed_hash(&self, h: &mut Fnv64) {
        h.feed_u8(self.code());
    }
}

/// How well an action counters the active threat (TZ §3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CounterRating {
    /// No action taken this decision.
    Neutral = 0,
    /// The single best counter for this threat.
    Correct = 1,
    /// A valid follow-up / secondary counter.
    Partial = 2,
    /// Wrong action for this threat.
    Wrong = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_roundtrips_for_all_kinds() {
        for v in 0..=12u8 {
            let k = ThreatKind::from_u8(v).expect("valid code");
            assert_eq!(k.code(), v);
        }
        assert_eq!(ThreatKind::from_u8(13), None);
    }

    #[test]
    fn counterplay_table_matches_spec_3_4() {
        use PlayerAction as A;
        // (threat, best, follow) straight from TZ §3.4.
        let table = [
            (
                ThreatKind::RawSignalExposure,
                A::Quarantine,
                Some(A::SealVault),
            ),
            (ThreatKind::ConsentDrift, A::Audit, Some(A::Revoke)),
            (
                ThreatKind::PermissionEscalation,
                A::Audit,
                Some(A::Authorize),
            ),
            (ThreatKind::VaultLeak, A::SealVault, None),
            (ThreatKind::StimulationSpike, A::Throttle, None),
            (ThreatKind::LatencyStorm, A::Throttle, None),
            (ThreatKind::ShadowAppProbe, A::Revoke, Some(A::Quarantine)),
            (ThreatKind::ReplayTamper, A::Audit, None),
            (ThreatKind::SensorNoiseBurst, A::Audit, None),
            (ThreatKind::PolicyConflict, A::Audit, Some(A::Revoke)),
            (ThreatKind::CognitiveOverload, A::Throttle, None),
            (
                ThreatKind::EmergencyOverride,
                A::Quarantine,
                Some(A::SealVault),
            ),
        ];
        for (threat, best, follow) in table {
            assert_eq!(threat.best_action(), best, "best for {:?}", threat);
            assert_eq!(threat.follow_action(), follow, "follow for {:?}", threat);
        }
    }

    #[test]
    fn rate_action_classifies_correctly() {
        let t = ThreatKind::RawSignalExposure;
        assert_eq!(
            t.rate_action(PlayerAction::Quarantine),
            CounterRating::Correct
        );
        assert_eq!(
            t.rate_action(PlayerAction::SealVault),
            CounterRating::Partial
        );
        assert_eq!(t.rate_action(PlayerAction::Throttle), CounterRating::Wrong);
        assert_eq!(t.rate_action(PlayerAction::None), CounterRating::Neutral);
    }

    #[test]
    fn event_projection_is_total() {
        // Every EventKind must map to a defined threat (no panics, const fn).
        for v in 0..=9u8 {
            let ek = EventKind::from_u8(v).unwrap();
            let _ = ThreatKind::from_event(ek);
        }
        assert_eq!(
            ThreatKind::from_event(EventKind::RawSignalExposure),
            ThreatKind::RawSignalExposure
        );
        assert_eq!(
            ThreatKind::from_event(EventKind::BenignFlow),
            ThreatKind::None
        );
    }

    #[test]
    fn critical_threats_are_the_terminal_set() {
        assert!(ThreatKind::RawSignalExposure.is_critical());
        assert!(ThreatKind::VaultLeak.is_critical());
        assert!(!ThreatKind::SensorNoiseBurst.is_critical());
        assert!(!ThreatKind::None.is_critical());
    }
}
