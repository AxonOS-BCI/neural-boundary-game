// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
// See LICENSE and IP_NOTICE.md for details.

//! Deterministic state machine (§4, §5). The canonical per-tick step is
//! [`GameState::advance`]; the browser and CLI both drive the game through it,
//! so an ABI-played run and a CLI replay of the same `(scenario, seed, actions)`
//! produce identical grade, score, and state hash (§19.1).
//!
//! PROVISIONAL §balance: per-event metric magnitudes and the "excellent"
//! release thresholds are authored to the §4.5 / §5 intent; the spec pins the
//! directions, the §5.2 terminal overrides, the §5.3 release thresholds, and
//! the §5.4 score formula, all implemented exactly.

use crate::actions::{ActionGate, ActionResult, PlayerAction};
use crate::events::{EventKind, NeuralEvent, PermissionScope};
use crate::hash::{Fnv64, Rng, HASH_ALGORITHM, RNG_ALGORITHM};
use crate::metrics::Metrics;
use crate::scenario::{scenario_by_id, Scenario};
use crate::scoring::{compute_score, terminal_score_override, ScoreCounters};
use crate::{ABI_VERSION, CORE_VERSION, MAX_ACTIVE_EVENTS, PRODUCT_VERSION_PACKED};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RunStatus {
    Running = 0,
    Ended = 1,
}

impl RunStatus {
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "RUNNING",
            Self::Ended => "ENDED",
        }
    }
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Ended)
    }
}

/// Sovereignty grades (§5.5). `Pending` is the in-progress sentinel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SovereigntyGrade {
    Pending = 0,
    Sealed = 1,
    Reviewable = 2,
    Breached = 3,
    Unsafe = 4,
}

impl SovereigntyGrade {
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Sealed => "SEALED",
            Self::Reviewable => "REVIEWABLE",
            Self::Breached => "BREACHED",
            Self::Unsafe => "UNSAFE",
        }
    }
    pub const fn public_label(self) -> &'static str {
        match self {
            Self::Pending => "In Progress",
            Self::Sealed => "Sovereignty Sealed",
            Self::Reviewable => "Reviewable Boundary",
            Self::Breached => "Boundary Breached",
            Self::Unsafe => "Unsafe Release Blocked",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EndReason {
    None = 0,
    SuccessRelease = 1,
    BoundaryCollapse = 2,
    RawLeakLimit = 3,
    UnsafeStimulationEscape = 4,
    ConsentCollapse = 5,
    VaultFailure = 6,
    Timeout = 7,
}

impl EndReason {
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::SuccessRelease => "SUCCESS_RELEASE",
            Self::BoundaryCollapse => "BOUNDARY_COLLAPSE",
            Self::RawLeakLimit => "RAW_LEAK_LIMIT",
            Self::UnsafeStimulationEscape => "UNSAFE_STIMULATION_ESCAPE",
            Self::ConsentCollapse => "CONSENT_COLLAPSE",
            Self::VaultFailure => "VAULT_FAILURE",
            Self::Timeout => "TIMEOUT",
        }
    }
    pub fn from_label(s: &str) -> Option<Self> {
        Some(match s {
            "NONE" => Self::None,
            "SUCCESS_RELEASE" => Self::SuccessRelease,
            "BOUNDARY_COLLAPSE" => Self::BoundaryCollapse,
            "RAW_LEAK_LIMIT" => Self::RawLeakLimit,
            "UNSAFE_STIMULATION_ESCAPE" => Self::UnsafeStimulationEscape,
            "CONSENT_COLLAPSE" => Self::ConsentCollapse,
            "VAULT_FAILURE" => Self::VaultFailure,
            "TIMEOUT" => Self::Timeout,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PermissionState {
    pub active_scopes: PermissionScope,
    pub count: u8,
    pub last_change_tick: u32,
}

impl PermissionState {
    pub fn has_sensitive(&self) -> bool {
        self.active_scopes.is_sensitive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivacyVaultState {
    pub sealed: bool,
    pub capacity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct StimulationState {
    pub level: u8,
    pub throttled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct AuditState {
    pub pending: u8,
    pub resolved: u32,
}

/// PROVISIONAL §balance — starting vault seal charges.
const INITIAL_VAULT_CAPACITY: u8 = 5;

#[derive(Clone, Debug)]
pub struct GameState {
    pub version_packed: u32,
    pub abi_version: u32,
    pub scenario_id: u32,
    pub seed: u64,
    pub tick: u32,
    pub status: RunStatus,
    pub metrics: Metrics,
    pub permissions: PermissionState,
    pub vault: PrivacyVaultState,
    pub stimulation: StimulationState,
    pub audit: AuditState,
    pub score: i32,
    pub grade: SovereigntyGrade,
    pub final_reason: EndReason,
    pub state_hash: u64,
    pub counters: ScoreCounters,
    pub gate: ActionGate,
    pub last_action_result: ActionResult,
    // Active event pool (§18 MAX_ACTIVE_EVENTS).
    events: [Option<NeuralEvent>; MAX_ACTIVE_EVENTS],
    next_event_id: u32,
    schedule_cursor: usize,
    rng: Rng,
    scenario: &'static Scenario,
}

impl GameState {
    pub fn new(scenario_id: u32, seed: u64) -> Option<Self> {
        let scenario = scenario_by_id(scenario_id)?;
        let mut s = Self {
            version_packed: PRODUCT_VERSION_PACKED,
            abi_version: ABI_VERSION,
            scenario_id,
            seed,
            tick: 0,
            status: RunStatus::Running,
            metrics: scenario.initial_metrics,
            permissions: PermissionState {
                active_scopes: PermissionScope::FLOW,
                count: 1,
                last_change_tick: 0,
            },
            vault: PrivacyVaultState {
                sealed: false,
                capacity: INITIAL_VAULT_CAPACITY,
            },
            stimulation: StimulationState::default(),
            audit: AuditState::default(),
            score: 0,
            grade: SovereigntyGrade::Pending,
            final_reason: EndReason::None,
            state_hash: 0,
            counters: ScoreCounters::default(),
            gate: ActionGate::default(),
            last_action_result: ActionResult::NoOp,
            events: [None; MAX_ACTIVE_EVENTS],
            next_event_id: 1,
            schedule_cursor: 0,
            rng: Rng::new(seed),
            scenario,
        };
        s.finalize();
        Some(s)
    }

    // ── Accessors ──────────────────────────────────────────────────────────

    pub fn scenario(&self) -> &'static Scenario {
        self.scenario
    }
    pub const fn tick(&self) -> u32 {
        self.tick
    }
    pub const fn status(&self) -> RunStatus {
        self.status
    }
    pub const fn grade(&self) -> SovereigntyGrade {
        self.grade
    }
    pub const fn end_reason(&self) -> EndReason {
        self.final_reason
    }
    pub const fn score(&self) -> i32 {
        self.score
    }
    pub const fn metrics(&self) -> Metrics {
        self.metrics
    }
    pub const fn state_hash(&self) -> u64 {
        self.state_hash
    }
    pub const fn rng_state(&self) -> u64 {
        self.rng.state()
    }
    pub const fn last_action_result(&self) -> ActionResult {
        self.last_action_result
    }
    pub fn active_event_count(&self) -> u8 {
        self.events.iter().filter(|e| e.is_some()).count() as u8
    }
    /// Focused event = soonest to expire (tie: lowest id). Drives the UI card.
    pub fn focused_event(&self) -> Option<NeuralEvent> {
        self.focused_index()
            .map(|i| self.events[i].expect("focused index valid"))
    }

    fn focused_index(&self) -> Option<usize> {
        let mut best: Option<(usize, u32, u32)> = None;
        for (i, slot) in self.events.iter().enumerate() {
            if let Some(ev) = slot {
                let key = (ev.expires_at_tick, ev.id);
                if best.map(|(_, e, id)| key < (e, id)).unwrap_or(true) {
                    best = Some((i, key.0, key.1));
                }
            }
        }
        best.map(|(i, _, _)| i)
    }

    fn has_active_critical(&self) -> bool {
        self.events.iter().flatten().any(|e| e.kind.is_critical())
    }

    fn raw_exposure_active(&self) -> bool {
        self.events
            .iter()
            .flatten()
            .any(|e| e.kind == EventKind::RawSignalExposure)
    }

    // ── Release availability (§5.3 + scenario thresholds) ────────────────────

    pub fn release_available(&self) -> bool {
        if self.status.is_terminal() {
            return false;
        }
        let t = &self.scenario.success_thresholds;
        let m = &self.metrics;
        self.tick >= t.min_release_tick
            && !self.has_active_critical()
            && m.boundary_integrity >= t.boundary_min
            && m.consent_coherence >= t.consent_min
            && m.vault_integrity >= t.vault_min
            && m.raw_leak_risk <= t.raw_max
            && m.stimulation_risk <= t.stim_max
            && m.latency_pressure <= t.latency_max
    }

    /// Bit mask of unmet release conditions (for "why is release locked" UX).
    pub fn release_blockers(&self) -> u16 {
        let t = &self.scenario.success_thresholds;
        let m = &self.metrics;
        let mut b = 0u16;
        if self.tick < t.min_release_tick {
            b |= 1 << 0;
        }
        if self.has_active_critical() {
            b |= 1 << 1;
        }
        if m.boundary_integrity < t.boundary_min {
            b |= 1 << 2;
        }
        if m.consent_coherence < t.consent_min {
            b |= 1 << 3;
        }
        if m.vault_integrity < t.vault_min {
            b |= 1 << 4;
        }
        if m.raw_leak_risk > t.raw_max {
            b |= 1 << 5;
        }
        if m.stimulation_risk > t.stim_max {
            b |= 1 << 6;
        }
        if m.latency_pressure > t.latency_max {
            b |= 1 << 7;
        }
        b
    }

    // ── Canonical per-tick step ───────────────────────────────────────────────

    pub fn advance(&mut self, action: PlayerAction) -> ActionResult {
        if self.status.is_terminal() {
            self.last_action_result = ActionResult::RejectedTerminalState;
            return ActionResult::RejectedTerminalState;
        }
        self.tick += 1;
        self.gate.begin_tick();

        self.spawn_due_events();
        self.expire_events();

        let result = self.apply(action);

        self.passive_drift();
        self.check_terminal();

        if !self.status.is_terminal() && self.tick >= self.scenario.max_ticks {
            self.end(EndReason::Timeout);
        }

        self.finalize();
        self.last_action_result = result;
        result
    }

    fn end(&mut self, reason: EndReason) {
        if !self.status.is_terminal() {
            self.status = RunStatus::Ended;
            self.final_reason = reason;
        }
    }

    // ── Event spawning / expiry ───────────────────────────────────────────────

    fn spawn_due_events(&mut self) {
        let schedule = self.scenario.event_schedule;
        while self.schedule_cursor < schedule.len()
            && schedule[self.schedule_cursor].at_tick <= self.tick
        {
            let s = schedule[self.schedule_cursor];
            self.schedule_cursor += 1;
            // Seed-driven jitter keeps `seed` meaningful and deterministic.
            let jitter = self.rng.range(11) as i16 - 5;
            let hidden = (s.hidden_risk as i16 + jitter).clamp(0, 100) as u8;
            let ev = NeuralEvent {
                id: self.next_event_id,
                kind: s.kind,
                severity: s.severity,
                hidden_risk: hidden,
                visible_risk: s.visible_risk,
                permission_scope: s.scope,
                requires_audit: s.requires_audit,
                audited: false,
                created_at_tick: self.tick,
                expires_at_tick: self.tick + s.ttl_ticks,
            };
            self.next_event_id += 1;
            if let Some(slot) = self.events.iter_mut().find(|e| e.is_none()) {
                *slot = Some(ev);
                if ev.requires_audit {
                    self.audit.pending = self.audit.pending.saturating_add(1);
                }
            }
            // Pool full → event is dropped (pool pressure); deterministic.
        }
    }

    fn expire_events(&mut self) {
        for i in 0..MAX_ACTIVE_EVENTS {
            let expired = matches!(self.events[i], Some(ev) if self.tick >= ev.expires_at_tick);
            if expired {
                let ev = self.events[i].expect("expired slot present");
                self.apply_expiry_penalty(ev);
                if ev.kind.is_critical() {
                    self.counters.unresolved_critical_events =
                        self.counters.unresolved_critical_events.saturating_add(1);
                }
                if ev.requires_audit && !ev.audited && self.audit.pending > 0 {
                    self.audit.pending -= 1;
                }
                self.events[i] = None;
            }
        }
    }

    fn apply_expiry_penalty(&mut self, ev: NeuralEvent) {
        let m = &mut self.metrics;
        match ev.kind {
            EventKind::RawSignalExposure => {
                Metrics::adjust(&mut m.raw_leak_risk, 22);
                Metrics::adjust(&mut m.boundary_integrity, -10);
            }
            EventKind::UnsafeStimulation => {
                Metrics::adjust(&mut m.stimulation_risk, 22);
                Metrics::adjust(&mut m.boundary_integrity, -8);
            }
            EventKind::PermissionEscalation | EventKind::AdversarialProbe => {
                Metrics::adjust(&mut m.consent_coherence, -14);
                Metrics::adjust(&mut m.boundary_integrity, -6);
            }
            EventKind::LatencySpike => {
                Metrics::adjust(&mut m.latency_pressure, 16);
            }
            EventKind::VaultPressure => {
                Metrics::adjust(&mut m.vault_integrity, -12);
            }
            EventKind::StimulationRequest => {
                Metrics::adjust(&mut m.stimulation_risk, 8);
            }
            EventKind::ConsentRequest => {
                Metrics::adjust(&mut m.consent_coherence, -6);
            }
            EventKind::BenignFlow | EventKind::AuditCheckpoint => {}
        }
    }

    fn take_focused(&mut self) -> Option<NeuralEvent> {
        let idx = self.focused_index()?;
        self.events[idx].take()
    }

    // ── Action application ─────────────────────────────────────────────────────

    fn apply(&mut self, action: PlayerAction) -> ActionResult {
        if action == PlayerAction::None {
            return ActionResult::NoOp;
        }
        if action == PlayerAction::Release {
            if self.release_available() {
                let excellent = self.is_excellent();
                self.counters.clean_release_bonus = if excellent { 200 } else { 100 }; // PROVISIONAL §balance
                self.gate.record_accept(self.tick);
                self.end(EndReason::SuccessRelease);
                return ActionResult::Accepted;
            }
            self.gate.record_reject();
            return ActionResult::RejectedReleaseLocked;
        }
        if let Some(reject) = self.gate.gate(self.tick) {
            self.gate.record_reject();
            return reject;
        }

        let needs_event = matches!(action, PlayerAction::Authorize | PlayerAction::Quarantine);
        if needs_event && self.focused_index().is_none() {
            self.gate.record_reject();
            return ActionResult::RejectedInvalidForEvent;
        }

        match action {
            PlayerAction::Authorize => self.act_authorize(),
            PlayerAction::Revoke => self.act_revoke(),
            PlayerAction::Quarantine => self.act_quarantine(),
            PlayerAction::SealVault => self.act_seal_vault(),
            PlayerAction::Throttle => self.act_throttle(),
            PlayerAction::Audit => self.act_audit(),
            PlayerAction::None | PlayerAction::Release => unreachable!(),
        }
        self.gate.record_accept(self.tick);
        ActionResult::Accepted
    }

    fn act_authorize(&mut self) {
        let ev = self.take_focused().expect("authorize requires event");
        self.permissions.active_scopes = self.permissions.active_scopes.union(ev.permission_scope);
        self.permissions.count = self.permissions.count.saturating_add(1);
        self.permissions.last_change_tick = self.tick;
        let m = &mut self.metrics;
        let guardrail = self.stimulation.throttled || ev.audited;
        match ev.kind {
            EventKind::UnsafeStimulation => {
                if guardrail {
                    Metrics::adjust(&mut m.stimulation_risk, 8);
                    Metrics::adjust(&mut m.cognitive_flow, 6);
                } else {
                    // §5.2: unsafe stimulation authorized without guardrail.
                    self.counters.unsafe_actions = self.counters.unsafe_actions.saturating_add(1);
                    Metrics::adjust(&mut m.stimulation_risk, 60);
                    self.end(EndReason::UnsafeStimulationEscape);
                }
            }
            EventKind::RawSignalExposure => {
                if self.vault.sealed {
                    Metrics::adjust(&mut m.raw_leak_risk, 4);
                    Metrics::adjust(&mut m.cognitive_flow, 6);
                } else {
                    Metrics::adjust(&mut m.raw_leak_risk, 26);
                    Metrics::adjust(&mut m.vault_integrity, -10);
                    Metrics::adjust(&mut m.cognitive_flow, 4);
                }
            }
            EventKind::PermissionEscalation | EventKind::AdversarialProbe => {
                let penalty = if ev.audited { -10 } else { -18 };
                Metrics::adjust(&mut m.consent_coherence, penalty);
                Metrics::adjust(&mut m.boundary_integrity, -6);
                Metrics::adjust(&mut m.cognitive_flow, 6);
                if !ev.audited {
                    self.counters.unsafe_actions = self.counters.unsafe_actions.saturating_add(1);
                }
            }
            EventKind::StimulationRequest => {
                Metrics::adjust(&mut m.stimulation_risk, 8);
                Metrics::adjust(&mut m.cognitive_flow, 8);
            }
            EventKind::ConsentRequest | EventKind::BenignFlow => {
                Metrics::adjust(&mut m.consent_coherence, 4);
                Metrics::adjust(&mut m.cognitive_flow, 8);
            }
            EventKind::LatencySpike | EventKind::VaultPressure | EventKind::AuditCheckpoint => {
                Metrics::adjust(&mut m.cognitive_flow, 6);
            }
        }
    }

    fn act_revoke(&mut self) {
        let had_sensitive = self.permissions.has_sensitive();
        self.permissions.active_scopes = PermissionScope::FLOW;
        self.permissions.count = 1;
        self.permissions.last_change_tick = self.tick;
        let m = &mut self.metrics;
        if had_sensitive {
            self.counters.correct_revocations = self.counters.correct_revocations.saturating_add(1);
            Metrics::adjust(&mut m.consent_coherence, 14);
            Metrics::adjust(&mut m.cognitive_flow, -8);
            Metrics::adjust(&mut m.raw_leak_risk, -6);
        } else {
            Metrics::adjust(&mut m.consent_coherence, 6);
            Metrics::adjust(&mut m.cognitive_flow, -6);
        }
    }

    fn act_quarantine(&mut self) {
        let ev = self.take_focused().expect("quarantine requires event");
        let m = &mut self.metrics;
        Metrics::adjust(&mut m.boundary_integrity, 4);
        Metrics::adjust(&mut m.raw_leak_risk, -14);
        Metrics::adjust(&mut m.stimulation_risk, -10);
        Metrics::adjust(&mut m.latency_pressure, 12);
        if ev.kind == EventKind::BenignFlow {
            Metrics::adjust(&mut m.cognitive_flow, -10);
        }
    }

    fn act_seal_vault(&mut self) {
        let m = &mut self.metrics;
        if self.vault.capacity > 0 {
            self.vault.sealed = true;
            self.vault.capacity -= 1;
            Metrics::adjust(&mut m.vault_integrity, 16);
            Metrics::adjust(&mut m.raw_leak_risk, -20);
        }
    }

    fn act_throttle(&mut self) {
        self.stimulation.throttled = true;
        self.stimulation.level = self.stimulation.level.saturating_sub(30);
        let m = &mut self.metrics;
        Metrics::adjust(&mut m.stimulation_risk, -22);
        Metrics::adjust(&mut m.cognitive_flow, -6);
        Metrics::adjust(&mut m.latency_pressure, -4);
    }

    fn act_audit(&mut self) {
        let m_latency;
        let idx = self.focused_index();
        if let Some(i) = idx {
            let ev = self.events[i].as_mut().expect("focused slot present");
            if ev.requires_audit && !ev.audited {
                ev.reveal();
                self.counters.successful_audits = self.counters.successful_audits.saturating_add(1);
                self.audit.resolved = self.audit.resolved.saturating_add(1);
                if self.audit.pending > 0 {
                    self.audit.pending -= 1;
                }
                Metrics::adjust(&mut self.metrics.audit_confidence, 18);
                m_latency = 6;
            } else {
                if !ev.audited {
                    ev.reveal();
                }
                Metrics::adjust(&mut self.metrics.audit_confidence, 6);
                m_latency = 6;
            }
        } else {
            Metrics::adjust(&mut self.metrics.audit_confidence, 4);
            m_latency = 6;
        }
        Metrics::adjust(&mut self.metrics.latency_pressure, m_latency);
    }

    // ── Passive drift (PROVISIONAL §balance) ─────────────────────────────────

    fn passive_drift(&mut self) {
        let any_event = self.active_event_count() > 0;
        let raw_active = self.raw_exposure_active();
        let critical_active = self.has_active_critical();
        let stale_sensitive = self.permissions.has_sensitive()
            && self.tick.saturating_sub(self.permissions.last_change_tick) > 240;
        let m = &mut self.metrics;
        if !any_event {
            Metrics::adjust(&mut m.cognitive_flow, 1);
            Metrics::adjust(&mut m.latency_pressure, -1);
        }
        if stale_sensitive {
            Metrics::adjust(&mut m.consent_coherence, -1);
        }
        if raw_active && !self.vault.sealed {
            Metrics::adjust(&mut m.raw_leak_risk, 2);
            Metrics::adjust(&mut m.vault_integrity, -1);
        }
        if critical_active {
            Metrics::adjust(&mut m.boundary_integrity, -1);
        }
    }

    // ── Terminal conditions (§5.2) ────────────────────────────────────────────

    fn check_terminal(&mut self) {
        if self.status.is_terminal() {
            return;
        }
        let m = self.metrics;
        if m.boundary_integrity == 0 {
            self.end(EndReason::BoundaryCollapse);
        } else if m.raw_leak_risk >= 100 {
            self.end(EndReason::RawLeakLimit);
        } else if m.stimulation_risk >= 100 {
            self.end(EndReason::UnsafeStimulationEscape);
        } else if m.consent_coherence == 0 && self.permissions.has_sensitive() {
            self.end(EndReason::ConsentCollapse);
        } else if m.vault_integrity == 0 && self.raw_exposure_active() {
            self.end(EndReason::VaultFailure);
        }
    }

    // ── Finalize: clamp, score, grade, hash ──────────────────────────────────

    fn finalize(&mut self) {
        // Metrics are clamped at every adjust(); this is the invariant guard.
        debug_assert!(self.metrics.in_bounds());

        let formula = compute_score(
            &self.metrics,
            &self.counters,
            self.gate.rejected_actions_total,
        );
        self.score = match terminal_score_override(self.final_reason) {
            Some(over) if self.status.is_terminal() => over,
            _ => formula,
        };
        self.grade = self.derive_grade();
        self.state_hash = self.compute_hash();
    }

    fn is_excellent(&self) -> bool {
        let m = &self.metrics;
        m.boundary_integrity >= 85
            && m.consent_coherence >= 85
            && m.vault_integrity >= 85
            && m.raw_leak_risk <= 15
            && m.stimulation_risk <= 15
            && m.latency_pressure <= 40
            && m.audit_confidence >= 60
            && self.counters.unresolved_critical_events == 0
            && self.counters.unsafe_actions == 0
    }

    fn derive_grade(&self) -> SovereigntyGrade {
        if self.status == RunStatus::Running {
            return SovereigntyGrade::Pending;
        }
        match self.final_reason {
            EndReason::UnsafeStimulationEscape => SovereigntyGrade::Unsafe,
            EndReason::BoundaryCollapse
            | EndReason::RawLeakLimit
            | EndReason::ConsentCollapse
            | EndReason::VaultFailure => SovereigntyGrade::Breached,
            EndReason::SuccessRelease => {
                if self.is_excellent() {
                    SovereigntyGrade::Sealed
                } else {
                    SovereigntyGrade::Reviewable
                }
            }
            EndReason::Timeout => {
                if self.metrics.boundary_integrity >= 50
                    && !self.has_active_critical()
                    && self.counters.unresolved_critical_events == 0
                {
                    SovereigntyGrade::Reviewable
                } else {
                    SovereigntyGrade::Breached
                }
            }
            EndReason::None => SovereigntyGrade::Breached,
        }
    }

    // ── State hash (§19; fnv1a64-v1, little-endian, full pool in slot order) ──

    pub fn compute_hash(&self) -> u64 {
        let mut h = Fnv64::new();
        h.feed_str(CORE_VERSION);
        h.feed_str(HASH_ALGORITHM);
        h.feed_str(RNG_ALGORITHM);
        h.feed_u32(self.version_packed);
        h.feed_u32(self.abi_version);
        h.feed_u32(self.scenario_id);
        h.feed_u64(self.seed);
        h.feed_u32(self.tick);
        h.feed_u8(self.status.code());
        self.metrics.feed_hash(&mut h);
        for slot in &self.events {
            match slot {
                None => h.feed_u8(0),
                Some(ev) => {
                    h.feed_u8(1);
                    ev.feed_hash(&mut h);
                }
            }
        }
        h.feed_u16(self.permissions.active_scopes.bits());
        h.feed_u8(self.permissions.count);
        h.feed_u32(self.permissions.last_change_tick);
        h.feed_bool(self.vault.sealed);
        h.feed_u8(self.vault.capacity);
        h.feed_u8(self.stimulation.level);
        h.feed_bool(self.stimulation.throttled);
        h.feed_u8(self.audit.pending);
        h.feed_u32(self.audit.resolved);
        h.feed_i32(self.score);
        h.feed_u8(self.grade.code());
        h.feed_u8(self.final_reason.code());
        h.feed_u32(self.counters.unresolved_critical_events);
        h.feed_u32(self.counters.unsafe_actions);
        h.feed_u32(self.counters.successful_audits);
        h.feed_u32(self.counters.correct_revocations);
        h.feed_i32(self.counters.clean_release_bonus);
        h.feed_u32(self.gate.rejected_actions_total);
        h.feed_u8(self.last_action_result.code());
        h.feed_u64(self.rng.state());
        h.finish()
    }
}

// Re-export for SuccessThresholds consumers.
pub use crate::scenario::SuccessThresholds as ReleaseThresholds;

#[cfg(test)]
mod state_tests {
    use super::*;
    use crate::events::EventKind;

    fn fresh(scn: u32) -> GameState {
        GameState::new(scn, 7).expect("scenario exists")
    }

    fn inject(g: &mut GameState, kind: EventKind, scope: PermissionScope, audit: bool) {
        g.events[0] = Some(NeuralEvent {
            id: 999,
            kind,
            severity: 80,
            hidden_risk: 90,
            visible_risk: 40,
            permission_scope: scope,
            requires_audit: audit,
            audited: false,
            created_at_tick: g.tick,
            expires_at_tick: g.tick + 10_000,
        });
    }

    #[test]
    fn unsafe_authorize_without_guardrail_is_unsafe() {
        let mut g = fresh(4);
        inject(
            &mut g,
            EventKind::UnsafeStimulation,
            PermissionScope::STIM,
            true,
        );
        // Not throttled, not audited → no guardrail.
        let r = g.advance(PlayerAction::Authorize);
        assert_eq!(r, ActionResult::Accepted);
        assert_eq!(g.end_reason(), EndReason::UnsafeStimulationEscape);
        assert_eq!(g.grade(), SovereigntyGrade::Unsafe);
        assert_ne!(g.grade(), SovereigntyGrade::Sealed);
        assert_eq!(g.score(), -1000);
    }

    #[test]
    fn unsafe_authorize_with_throttle_guardrail_survives() {
        let mut g = fresh(4);
        g.advance(PlayerAction::Throttle); // establish guardrail
        inject(
            &mut g,
            EventKind::UnsafeStimulation,
            PermissionScope::STIM,
            true,
        );
        let r = g.advance(PlayerAction::Authorize);
        assert_eq!(r, ActionResult::Accepted);
        assert_ne!(g.end_reason(), EndReason::UnsafeStimulationEscape);
    }

    #[test]
    fn vault_failure_during_raw_exposure() {
        let mut g = fresh(2);
        inject(
            &mut g,
            EventKind::RawSignalExposure,
            PermissionScope::RAW,
            true,
        );
        g.metrics.vault_integrity = 0;
        g.advance(PlayerAction::None);
        assert_eq!(g.end_reason(), EndReason::VaultFailure);
        assert_eq!(g.grade(), SovereigntyGrade::Breached);
        assert_eq!(g.score(), -900);
    }

    #[test]
    fn focused_event_is_soonest_to_expire() {
        let mut g = fresh(9);
        g.events[0] = Some(NeuralEvent {
            id: 1,
            kind: EventKind::BenignFlow,
            severity: 1,
            hidden_risk: 1,
            visible_risk: 1,
            permission_scope: PermissionScope::FLOW,
            requires_audit: false,
            audited: false,
            created_at_tick: 0,
            expires_at_tick: 500,
        });
        g.events[1] = Some(NeuralEvent {
            id: 2,
            kind: EventKind::RawSignalExposure,
            severity: 1,
            hidden_risk: 1,
            visible_risk: 1,
            permission_scope: PermissionScope::RAW,
            requires_audit: false,
            audited: false,
            created_at_tick: 0,
            expires_at_tick: 300,
        });
        assert_eq!(g.focused_event().map(|e| e.id), Some(2));
    }

    #[test]
    fn release_blockers_clear_when_stable() {
        let mut g = fresh(1);
        // Force comfortably-stable metrics and enough elapsed time.
        g.metrics = Metrics::new(100, 100, 100, 90, 0, 0, 0, 80);
        g.tick = 700; // > 30 s at 20 Hz
        assert_eq!(g.release_blockers(), 0);
        assert!(g.release_available());
    }
}
