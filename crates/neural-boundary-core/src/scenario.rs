// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.0.1).
// See LICENSE and IP_NOTICE.md for details.

//! Scenario system (§7). Nine scenarios, each a static deterministic schedule
//! plus per-scenario initial metrics, success thresholds, and tick budget.
//!
//! PROVISIONAL §balance: the event-schedule CONTENTS (which event fires when,
//! with what risk) are authored to the §7 difficulty/purpose intent. The spec
//! pins the STRUCTURE (ScheduledEvent, SuccessThresholds) and the §5.3 default
//! thresholds; the per-scenario tuning here is the single thing that changes
//! when balance tables arrive. Structure, scoring, and invariants do not.

use crate::events::{EventKind, PermissionScope};
use crate::metrics::Metrics;

pub const TICK_RATE_HZ: u32 = 20;
const SEC: u32 = TICK_RATE_HZ;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Difficulty {
    Easy = 0,
    Medium = 1,
    Hard = 2,
    Elite = 3,
}

impl Difficulty {
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Easy => "EASY",
            Self::Medium => "MEDIUM",
            Self::Hard => "HARD",
            Self::Elite => "ELITE",
        }
    }
}

/// One scheduled event in a scenario timeline.
#[derive(Clone, Copy, Debug)]
pub struct ScheduledEvent {
    pub at_tick: u32,
    pub kind: EventKind,
    pub severity: u8,
    pub hidden_risk: u8,
    pub visible_risk: u8,
    pub scope: PermissionScope,
    pub requires_audit: bool,
    pub ttl_ticks: u32,
}

/// Release-availability thresholds (§5.3). Defaults match the spec; scenarios
/// may set stricter values.
#[derive(Clone, Copy, Debug)]
pub struct SuccessThresholds {
    pub boundary_min: u8,
    pub consent_min: u8,
    pub vault_min: u8,
    pub raw_max: u8,
    pub stim_max: u8,
    pub latency_max: u8,
    pub min_release_tick: u32,
}

impl SuccessThresholds {
    /// §5.3 defaults: 70 / 65 / 65 / 35 / 35 / 70 / 30 s.
    pub const DEFAULT: Self = Self {
        boundary_min: 70,
        consent_min: 65,
        vault_min: 65,
        raw_max: 35,
        stim_max: 35,
        latency_max: 70,
        min_release_tick: 30 * SEC,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct Scenario {
    pub id: u32,
    pub name: &'static str,
    pub difficulty: Difficulty,
    pub objective: &'static str,
    pub initial_metrics: Metrics,
    pub event_schedule: &'static [ScheduledEvent],
    pub success_thresholds: SuccessThresholds,
    pub max_ticks: u32,
}

pub const SCENARIO_COUNT: u32 = 9;

pub fn scenario_by_id(id: u32) -> Option<&'static Scenario> {
    SCENARIOS.iter().find(|s| s.id == id)
}

// Convenience constructors for compact schedule tables.
#[allow(clippy::too_many_arguments)]
const fn ev(
    at: u32,
    kind: EventKind,
    sev: u8,
    hidden: u8,
    visible: u8,
    scope: PermissionScope,
    audit: bool,
    ttl: u32,
) -> ScheduledEvent {
    ScheduledEvent {
        at_tick: at,
        kind,
        severity: sev,
        hidden_risk: hidden,
        visible_risk: visible,
        scope,
        requires_audit: audit,
        ttl_ticks: ttl,
    }
}

use EventKind::*;
const FLOW: PermissionScope = PermissionScope::FLOW;
const STIM: PermissionScope = PermissionScope::STIM;
const RAW: PermissionScope = PermissionScope::RAW;
const ADMIN: PermissionScope = PermissionScope::ADMIN;

// ── Schedules (PROVISIONAL content, authored to §7 intent) ───────────────────

static S1_CLEAN: [ScheduledEvent; 8] = [
    ev(40, BenignFlow, 10, 5, 5, FLOW, false, 200),
    ev(150, ConsentRequest, 20, 18, 18, FLOW, false, 220),
    ev(320, AuditCheckpoint, 10, 10, 10, FLOW, false, 220),
    ev(520, BenignFlow, 12, 6, 6, FLOW, false, 200),
    ev(720, ConsentRequest, 22, 20, 20, FLOW, false, 220),
    ev(980, AuditCheckpoint, 10, 8, 8, FLOW, false, 220),
    ev(1240, BenignFlow, 12, 6, 6, FLOW, false, 200),
    ev(1500, ConsentRequest, 20, 16, 16, FLOW, false, 220),
];

static S2_RAW_LEAK: [ScheduledEvent; 9] = [
    ev(60, BenignFlow, 12, 6, 6, FLOW, false, 200),
    ev(220, RawSignalExposure, 60, 70, 35, RAW, true, 200),
    ev(460, VaultPressure, 45, 50, 50, RAW, false, 220),
    ev(720, RawSignalExposure, 65, 78, 38, RAW, true, 200),
    ev(1000, AuditCheckpoint, 12, 10, 10, FLOW, false, 220),
    ev(1240, RawSignalExposure, 70, 82, 40, RAW, true, 200),
    ev(1520, VaultPressure, 50, 55, 55, RAW, false, 220),
    ev(1780, RawSignalExposure, 68, 80, 38, RAW, true, 200),
    ev(2040, ConsentRequest, 20, 16, 16, FLOW, false, 220),
];

static S3_CONSENT_DRIFT: [ScheduledEvent; 9] = [
    ev(60, ConsentRequest, 20, 18, 18, ADMIN, false, 220),
    ev(260, PermissionEscalation, 50, 62, 28, ADMIN, true, 220),
    ev(520, ConsentRequest, 24, 22, 22, ADMIN, false, 220),
    ev(760, PermissionEscalation, 55, 68, 30, ADMIN, true, 220),
    ev(1020, AuditCheckpoint, 12, 10, 10, FLOW, false, 220),
    ev(1280, PermissionEscalation, 58, 72, 32, ADMIN, true, 220),
    ev(1540, AdversarialProbe, 60, 75, 30, ADMIN, true, 220),
    ev(1800, ConsentRequest, 22, 20, 20, ADMIN, false, 220),
    ev(2060, PermissionEscalation, 55, 66, 30, ADMIN, true, 220),
];

static S4_UNSAFE_STIM: [ScheduledEvent; 10] = [
    ev(60, StimulationRequest, 30, 30, 30, STIM, false, 200),
    ev(240, UnsafeStimulation, 70, 85, 40, STIM, true, 200),
    ev(480, StimulationRequest, 35, 35, 35, STIM, false, 200),
    ev(720, UnsafeStimulation, 75, 90, 42, STIM, true, 200),
    ev(980, AuditCheckpoint, 12, 10, 10, FLOW, false, 220),
    ev(1220, UnsafeStimulation, 78, 92, 44, STIM, true, 200),
    ev(1480, StimulationRequest, 32, 32, 32, STIM, false, 200),
    ev(1720, UnsafeStimulation, 80, 94, 46, STIM, true, 200),
    ev(1980, LatencySpike, 40, 45, 45, FLOW, false, 200),
    ev(2240, StimulationRequest, 30, 30, 30, STIM, false, 200),
];

static S5_LATENCY_STORM: [ScheduledEvent; 14] = [
    ev(40, LatencySpike, 40, 44, 44, FLOW, false, 160),
    ev(180, LatencySpike, 45, 50, 50, FLOW, false, 160),
    ev(320, BenignFlow, 12, 6, 6, FLOW, false, 160),
    ev(460, LatencySpike, 50, 55, 55, FLOW, false, 160),
    ev(600, ConsentRequest, 22, 20, 20, FLOW, false, 200),
    ev(760, LatencySpike, 52, 58, 58, FLOW, false, 160),
    ev(920, VaultPressure, 40, 44, 44, RAW, false, 200),
    ev(1080, LatencySpike, 55, 60, 60, FLOW, false, 160),
    ev(1240, AuditCheckpoint, 12, 10, 10, FLOW, false, 200),
    ev(1400, LatencySpike, 58, 64, 64, FLOW, false, 160),
    ev(1560, BenignFlow, 12, 6, 6, FLOW, false, 160),
    ev(1720, LatencySpike, 60, 66, 66, FLOW, false, 160),
    ev(1880, ConsentRequest, 22, 20, 20, FLOW, false, 200),
    ev(2040, LatencySpike, 55, 60, 60, FLOW, false, 160),
];

static S6_ADVERSARIAL: [ScheduledEvent; 10] = [
    ev(60, BenignFlow, 12, 6, 6, FLOW, false, 220),
    ev(240, AdversarialProbe, 55, 72, 22, ADMIN, true, 220),
    ev(480, PermissionEscalation, 55, 68, 26, ADMIN, true, 220),
    ev(720, AdversarialProbe, 60, 78, 24, ADMIN, true, 220),
    ev(980, AuditCheckpoint, 12, 10, 10, FLOW, false, 220),
    ev(1220, AdversarialProbe, 62, 80, 24, ADMIN, true, 220),
    ev(1460, PermissionEscalation, 58, 72, 28, ADMIN, true, 220),
    ev(1700, AdversarialProbe, 64, 82, 22, ADMIN, true, 220),
    ev(1940, ConsentRequest, 20, 18, 18, ADMIN, false, 220),
    ev(2180, AdversarialProbe, 60, 76, 24, ADMIN, true, 220),
];

static S7_VAULT_SIEGE: [ScheduledEvent; 12] = [
    ev(50, RawSignalExposure, 60, 70, 36, RAW, true, 180),
    ev(220, VaultPressure, 55, 60, 60, RAW, false, 200),
    ev(400, RawSignalExposure, 65, 78, 38, RAW, true, 180),
    ev(580, VaultPressure, 58, 64, 64, RAW, false, 200),
    ev(760, RawSignalExposure, 68, 82, 40, RAW, true, 180),
    ev(940, VaultPressure, 60, 66, 66, RAW, false, 200),
    ev(1120, RawSignalExposure, 70, 84, 40, RAW, true, 180),
    ev(1320, AuditCheckpoint, 12, 10, 10, FLOW, false, 200),
    ev(1520, RawSignalExposure, 72, 86, 42, RAW, true, 180),
    ev(1720, VaultPressure, 62, 68, 68, RAW, false, 200),
    ev(1920, RawSignalExposure, 70, 84, 40, RAW, true, 180),
    ev(2120, VaultPressure, 60, 66, 66, RAW, false, 200),
];

static S8_SILENT_ESCALATION: [ScheduledEvent; 12] = [
    ev(60, BenignFlow, 12, 6, 6, FLOW, false, 220),
    ev(220, ConsentRequest, 20, 16, 16, FLOW, false, 220),
    // Looks benign, hidden admin escalation — only audit reveals it.
    ev(420, PermissionEscalation, 60, 82, 14, ADMIN, true, 240),
    ev(640, BenignFlow, 12, 6, 6, FLOW, false, 220),
    ev(860, AdversarialProbe, 62, 84, 16, ADMIN, true, 240),
    ev(1080, AuditCheckpoint, 12, 10, 10, FLOW, false, 220),
    ev(1300, PermissionEscalation, 64, 86, 14, ADMIN, true, 240),
    ev(1520, ConsentRequest, 20, 16, 16, FLOW, false, 220),
    ev(1740, AdversarialProbe, 66, 88, 16, ADMIN, true, 240),
    ev(1960, PermissionEscalation, 64, 86, 14, ADMIN, true, 240),
    ev(2180, BenignFlow, 12, 6, 6, FLOW, false, 220),
    ev(2400, AdversarialProbe, 64, 84, 16, ADMIN, true, 240),
];

static S9_GRAND: [ScheduledEvent; 18] = [
    ev(40, BenignFlow, 12, 6, 6, FLOW, false, 200),
    ev(180, RawSignalExposure, 62, 74, 36, RAW, true, 180),
    ev(360, PermissionEscalation, 55, 70, 26, ADMIN, true, 220),
    ev(540, UnsafeStimulation, 72, 88, 42, STIM, true, 180),
    ev(720, LatencySpike, 52, 58, 58, FLOW, false, 160),
    ev(900, AdversarialProbe, 60, 80, 22, ADMIN, true, 220),
    ev(1080, VaultPressure, 58, 64, 64, RAW, false, 200),
    ev(1260, AuditCheckpoint, 12, 10, 10, FLOW, false, 200),
    ev(1440, RawSignalExposure, 68, 82, 40, RAW, true, 180),
    ev(1620, UnsafeStimulation, 76, 90, 44, STIM, true, 180),
    ev(1800, PermissionEscalation, 58, 74, 28, ADMIN, true, 220),
    ev(1980, LatencySpike, 56, 62, 62, FLOW, false, 160),
    ev(2160, AdversarialProbe, 64, 84, 22, ADMIN, true, 220),
    ev(2340, RawSignalExposure, 70, 84, 40, RAW, true, 180),
    ev(2520, VaultPressure, 60, 66, 66, RAW, false, 200),
    ev(2700, UnsafeStimulation, 78, 92, 46, STIM, true, 180),
    ev(2880, ConsentRequest, 22, 20, 20, ADMIN, false, 220),
    ev(3060, AdversarialProbe, 62, 80, 22, ADMIN, true, 220),
];

const M_EASY: Metrics = Metrics::new(100, 100, 100, 85, 0, 0, 0, 60);
const M_STD: Metrics = Metrics::new(100, 95, 100, 80, 0, 0, 0, 50);
const M_HARD: Metrics = Metrics::new(95, 90, 95, 75, 5, 5, 5, 45);
const M_ELITE: Metrics = Metrics::new(90, 85, 90, 70, 10, 10, 10, 40);

pub static SCENARIOS: [Scenario; 9] = [
    Scenario {
        id: 1,
        name: "Clean Boundary",
        difficulty: Difficulty::Easy,
        objective: "Learn authorize, revoke, and quarantine. Seal a calm boundary.",
        initial_metrics: M_EASY,
        event_schedule: &S1_CLEAN,
        success_thresholds: SuccessThresholds::DEFAULT,
        max_ticks: 90 * SEC,
    },
    Scenario {
        id: 2,
        name: "Raw Signal Leak",
        difficulty: Difficulty::Medium,
        objective: "Seal the Privacy Vault before raw exposure leaks.",
        initial_metrics: M_STD,
        event_schedule: &S2_RAW_LEAK,
        success_thresholds: SuccessThresholds::DEFAULT,
        max_ticks: 120 * SEC,
    },
    Scenario {
        id: 3,
        name: "Consent Drift",
        difficulty: Difficulty::Medium,
        objective: "Revoke or re-scope stale permissions before consent collapses.",
        initial_metrics: M_STD,
        event_schedule: &S3_CONSENT_DRIFT,
        success_thresholds: SuccessThresholds::DEFAULT,
        max_ticks: 120 * SEC,
    },
    Scenario {
        id: 4,
        name: "Unsafe Stimulation Escape",
        difficulty: Difficulty::Hard,
        objective: "Audit and throttle unsafe stimulation. Never authorize blind.",
        initial_metrics: M_HARD,
        event_schedule: &S4_UNSAFE_STIM,
        success_thresholds: SuccessThresholds {
            stim_max: 30,
            ..SuccessThresholds::DEFAULT
        },
        max_ticks: 130 * SEC,
    },
    Scenario {
        id: 5,
        name: "Latency Storm",
        difficulty: Difficulty::Hard,
        objective: "Balance throughput and safety under real-time pressure.",
        initial_metrics: M_HARD,
        event_schedule: &S5_LATENCY_STORM,
        success_thresholds: SuccessThresholds {
            latency_max: 65,
            ..SuccessThresholds::DEFAULT
        },
        max_ticks: 130 * SEC,
    },
    Scenario {
        id: 6,
        name: "Adversarial Permission Probe",
        difficulty: Difficulty::Hard,
        objective: "Apply least privilege. Audit before authorizing escalations.",
        initial_metrics: M_HARD,
        event_schedule: &S6_ADVERSARIAL,
        success_thresholds: SuccessThresholds {
            consent_min: 70,
            ..SuccessThresholds::DEFAULT
        },
        max_ticks: 140 * SEC,
    },
    Scenario {
        id: 7,
        name: "Privacy Vault Siege",
        difficulty: Difficulty::Hard,
        objective: "Manage limited vault capacity against relentless raw exposure.",
        initial_metrics: M_HARD,
        event_schedule: &S7_VAULT_SIEGE,
        success_thresholds: SuccessThresholds {
            vault_min: 70,
            ..SuccessThresholds::DEFAULT
        },
        max_ticks: 140 * SEC,
    },
    Scenario {
        id: 8,
        name: "Silent Escalation",
        difficulty: Difficulty::Elite,
        objective: "Hidden escalations look benign. Only auditing reveals the truth.",
        initial_metrics: M_ELITE,
        event_schedule: &S8_SILENT_ESCALATION,
        success_thresholds: SuccessThresholds {
            consent_min: 70,
            ..SuccessThresholds::DEFAULT
        },
        max_ticks: 150 * SEC,
    },
    Scenario {
        id: 9,
        name: "Grand Sovereignty Trial",
        difficulty: Difficulty::Elite,
        objective: "Every mechanic at once. Hold sovereignty to a clean release.",
        initial_metrics: M_ELITE,
        event_schedule: &S9_GRAND,
        success_thresholds: SuccessThresholds {
            boundary_min: 72,
            ..SuccessThresholds::DEFAULT
        },
        max_ticks: 180 * SEC,
    },
];
