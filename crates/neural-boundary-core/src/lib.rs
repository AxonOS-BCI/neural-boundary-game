#![no_std]
#![forbid(unsafe_code)]
//! Deterministic, allocation-free simulation core for Neural Boundary Game.
//!
//! The browser is a presentation adapter. This crate owns authoritative game
//! state, seeded randomness, tick progression, policy decisions, terminal
//! conditions, and canonical state hashing.

#[cfg(test)]
extern crate std;

use core::cmp::{max, min};

/// Product version represented by this core.
pub const CORE_VERSION: &str = "3.0.0";
/// Replay schema accepted by this core.
pub const REPLAY_SCHEMA: &str = "neural-boundary-replay-v3.0.0";
/// Canonical state-hash identifier.
pub const HASH_ALGORITHM: &str = "fnv1a64-v1";
/// Fixed simulation rate.
pub const TICK_RATE: u16 = 60;
/// Number of logical lanes.
pub const LANE_COUNT: usize = 5;
/// Maximum number of simultaneously active entities.
pub const ENTITY_CAPACITY: usize = 32;
/// Logical boundary position in fixed integer field coordinates.
pub const BOUNDARY_X: u16 = 840;
/// Initial entity position.
pub const SPAWN_X: u16 = 16;

/// Review gate: at least one intent was validated and converted.
pub const GATE_TYPE_SAFETY: u8 = 1 << 0;
/// Review gate: scoped consent is currently valid.
pub const GATE_CONSENT: u8 = 1 << 1;
/// Review gate: evidence level is at least L2.
pub const GATE_EVIDENCE: u8 = 1 << 2;
/// Review gate: private signal was contained without a leak.
pub const GATE_PRIVACY: u8 = 1 << 3;
/// Review gate: checksum/CI evidence established deterministic review.
pub const GATE_DETERMINISM: u8 = 1 << 4;
/// All mandatory review gates.
pub const ALL_REVIEW_GATES: u8 = GATE_TYPE_SAFETY
    | GATE_CONSENT
    | GATE_EVIDENCE
    | GATE_PRIVACY
    | GATE_DETERMINISM;

/// Consent scope allowing validated intent conversion.
pub const CONSENT_SCOPE_CONVERT: u8 = 1 << 0;
/// Consent scope allowing final boundary release.
pub const CONSENT_SCOPE_RELEASE: u8 = 1 << 1;
/// Complete scope required by the canonical boundary policy.
pub const CONSENT_SCOPE_REQUIRED: u8 = CONSENT_SCOPE_CONVERT | CONSENT_SCOPE_RELEASE;

/// Stable run-mode identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RunMode {
    /// Scripted onboarding run.
    Guided = 1,
    /// Canonical public run.
    Standard = 2,
    /// Higher-density adversarial review.
    Audit = 3,
    /// Multi-phase extended run.
    Grand = 4,
    /// Deterministic calendar-derived run.
    Daily = 5,
}

impl RunMode {
    /// Parse a stable numeric identifier.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Guided,
            3 => Self::Audit,
            4 => Self::Grand,
            5 => Self::Daily,
            _ => Self::Standard,
        }
    }
}

/// Stable difficulty identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Difficulty {
    /// Wider decision windows and lower packet density.
    Assisted = 1,
    /// Canonical balance.
    Standard = 2,
    /// Higher speed and density.
    Expert = 3,
}

impl Difficulty {
    /// Parse a stable numeric identifier.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Assisted,
            3 => Self::Expert,
            _ => Self::Standard,
        }
    }
}

/// Stable entity identifiers used in replay schemas and the WASM ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EntityKind {
    /// Private raw signal frame. Must be quarantined.
    RawFrame = 1,
    /// Signal artifact. Must be quarantined.
    Artifact = 2,
    /// Intent that has passed classification.
    ValidatedIntent = 3,
    /// Packet whose type is not yet known.
    UnknownPacket = 4,
    /// Application-safe typed intent.
    TypedIntent = 5,
    /// Scoped consent capability.
    ConsentToken = 6,
    /// Immediate revocation event.
    RevokedConsent = 7,
    /// Basic trace or proof.
    Evidence = 8,
    /// Deterministic file/state checksum proof.
    Checksum = 9,
    /// CI-confirmed evidence.
    CiTest = 10,
    /// Claim lacking support.
    UnsupportedClaim = 11,
    /// Claim without traceability.
    UntraceableClaim = 12,
    /// Roadmap statement presented as implemented fact.
    RoadmapAsFactClaim = 13,
    /// Unsafe stimulation command. Crossing is terminal.
    StimulationCommand = 14,
}

impl EntityKind {
    /// Parse a stable numeric identifier.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::RawFrame,
            2 => Self::Artifact,
            3 => Self::ValidatedIntent,
            4 => Self::UnknownPacket,
            5 => Self::TypedIntent,
            6 => Self::ConsentToken,
            7 => Self::RevokedConsent,
            8 => Self::Evidence,
            9 => Self::Checksum,
            10 => Self::CiTest,
            11 => Self::UnsupportedClaim,
            12 => Self::UntraceableClaim,
            13 => Self::RoadmapAsFactClaim,
            14 => Self::StimulationCommand,
            _ => Self::UnknownPacket,
        }
    }

    /// Action expected by the boundary policy.
    #[must_use]
    pub const fn required_action(self) -> BoundaryAction {
        match self {
            Self::UnknownPacket => BoundaryAction::Validate,
            Self::ValidatedIntent => BoundaryAction::Convert,
            Self::ConsentToken | Self::RevokedConsent => BoundaryAction::Consent,
            Self::Evidence | Self::Checksum | Self::CiTest => BoundaryAction::Evidence,
            Self::TypedIntent => BoundaryAction::None,
            Self::RawFrame
            | Self::Artifact
            | Self::UnsupportedClaim
            | Self::UntraceableClaim
            | Self::RoadmapAsFactClaim
            | Self::StimulationCommand => BoundaryAction::Quarantine,
        }
    }

    /// Whether the entity is private signal or an unsafe boundary object.
    #[must_use]
    pub const fn is_quarantine_target(self) -> bool {
        matches!(
            self,
            Self::RawFrame
                | Self::Artifact
                | Self::UnsupportedClaim
                | Self::UntraceableClaim
                | Self::RoadmapAsFactClaim
                | Self::StimulationCommand
        )
    }
}

/// Stable action identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BoundaryAction {
    /// No action.
    None = 0,
    /// Classify an unknown packet.
    Validate = 1,
    /// Convert a validated intent to typed intent.
    Convert = 2,
    /// Contain private or unsafe objects.
    Quarantine = 3,
    /// Establish or revoke scoped consent.
    Consent = 4,
    /// Register evidence.
    Evidence = 5,
    /// Attempt to seal the boundary.
    Release = 6,
}

impl BoundaryAction {
    /// Parse a stable numeric identifier.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Validate,
            2 => Self::Convert,
            3 => Self::Quarantine,
            4 => Self::Consent,
            5 => Self::Evidence,
            6 => Self::Release,
            _ => Self::None,
        }
    }
}

/// Evidence maturity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum EvidenceLevel {
    /// No evidence.
    L0 = 0,
    /// Trace/basic proof.
    L1 = 1,
    /// Checksum and deterministic validation.
    L2 = 2,
    /// Full CI-confirmed review set.
    L3 = 3,
}

impl EvidenceLevel {
    /// Parse a stable numeric identifier.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::L1,
            2 => Self::L2,
            3 => Self::L3,
            _ => Self::L0,
        }
    }
}

/// Boundary status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BoundaryStatus {
    /// Run is active.
    Open = 0,
    /// Release was sealed successfully.
    Sealed = 1,
    /// Run ended without a seal but without a catastrophic breach.
    Degraded = 2,
    /// Raw/privacy boundary failed.
    Breached = 3,
    /// A fail-closed unsafe condition occurred.
    Unsafe = 4,
}

/// Stable terminal reasons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TerminalReason {
    /// Run remains active.
    None = 0,
    /// All release invariants passed.
    Released = 1,
    /// Raw signal leak threshold reached.
    RawLeakLimit = 2,
    /// A stimulation command crossed the boundary.
    StimulationCrossed = 3,
    /// Integrity reached zero.
    IntegrityCollapse = 4,
    /// Risk reached its hard limit.
    RiskOverflow = 5,
    /// Run duration elapsed before sealing.
    TimeExpired = 6,
    /// Internal invariant failed closed.
    InvariantViolation = 7,
}

/// Stable feedback/event identifiers for presentation adapters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FeedbackCode {
    /// No event yet.
    None = 0,
    /// Unknown packet validated as intent.
    IntentValidated = 1,
    /// Unknown packet resolved as artifact.
    FalseIntentDetected = 2,
    /// Validated intent converted to typed intent.
    IntentConverted = 3,
    /// Conversion blocked by consent/evidence policy.
    ConversionBlocked = 4,
    /// Private or unsafe entity quarantined.
    Quarantined = 5,
    /// Consent activated.
    ConsentGranted = 6,
    /// Consent revoked.
    ConsentRevoked = 7,
    /// Evidence level increased.
    EvidenceRegistered = 8,
    /// Release sealed.
    ReleaseSealed = 9,
    /// Release attempt blocked by open invariants.
    ReleaseBlocked = 10,
    /// Incorrect action for selected entity.
    IncorrectAction = 11,
    /// No target exists in selected lane.
    NoTarget = 12,
    /// Raw signal crossed the boundary.
    RawLeak = 13,
    /// Stimulation command crossed and failed closed.
    StimulationBreach = 14,
    /// Safe typed intent reached the application layer.
    TypedIntentReleased = 15,
    /// Consent expired by tick.
    ConsentExpired = 16,
    /// Run expired.
    TimeExpired = 17,
}

/// Scoped consent state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsentState {
    /// Whether consent is active at the current tick.
    pub active: bool,
    /// Capability scope bit mask.
    pub scope: u8,
    /// Tick at which the capability became active.
    pub activation_tick: u32,
    /// Tick at which it expires (exclusive).
    pub expiry_tick: u32,
}

impl ConsentState {
    const fn none() -> Self {
        Self {
            active: false,
            scope: 0,
            activation_tick: 0,
            expiry_tick: 0,
        }
    }

    /// Whether every requested capability bit is currently granted.
    #[must_use]
    pub const fn allows(self, requested_scope: u8) -> bool {
        self.active && self.scope & requested_scope == requested_scope
    }
}

/// Fixed-capacity entity representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entity {
    /// Slot is active.
    pub active: bool,
    /// Stable per-run entity sequence ID.
    pub id: u32,
    /// Entity type.
    pub kind: EntityKind,
    /// Logical lane index from 0 to 4.
    pub lane: u8,
    /// Integer fixed-coordinate position.
    pub position: u16,
    /// Integer fixed-coordinate velocity per tick.
    pub speed: u16,
    /// Entity flags; bit 0 marks an unknown packet as a true intent candidate.
    pub flags: u8,
    /// Spawn tick.
    pub spawn_tick: u32,
}

impl Entity {
    const EMPTY: Self = Self {
        active: false,
        id: 0,
        kind: EntityKind::UnknownPacket,
        lane: 0,
        position: 0,
        speed: 0,
        flags: 0,
        spawn_tick: 0,
    };
}

/// Configuration captured in replay headers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulationConfig {
    /// Run seed. Zero is normalized to a nonzero deterministic constant.
    pub seed: u64,
    /// Run mode.
    pub mode: RunMode,
    /// Difficulty.
    pub difficulty: Difficulty,
    /// Maximum run duration in ticks. Zero selects the mode default.
    pub max_ticks: u32,
    /// Raw leak limit. Zero selects the default of three.
    pub raw_leak_limit: u8,
}

impl SimulationConfig {
    /// Create a canonical configuration.
    #[must_use]
    pub const fn canonical(seed: u64, mode: RunMode, difficulty: Difficulty) -> Self {
        Self {
            seed,
            mode,
            difficulty,
            max_ticks: 0,
            raw_leak_limit: 0,
        }
    }
}

/// Compact authoritative snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateSnapshot {
    /// Current tick.
    pub tick: u32,
    /// Selected lane.
    pub selected_lane: u8,
    /// Trust score, 0-100.
    pub trust: u8,
    /// Risk score, 0-100.
    pub risk: u8,
    /// Integrity score, 0-100.
    pub integrity: u8,
    /// Current evidence level.
    pub evidence: EvidenceLevel,
    /// Mandatory review gate bitmask.
    pub review_gates: u8,
    /// Raw leak count.
    pub raw_leaks: u8,
    /// Score.
    pub score: u32,
    /// Current correct-action streak.
    pub streak: u16,
    /// Best streak in this run.
    pub best_streak: u16,
    /// Boundary status.
    pub status: BoundaryStatus,
    /// Terminal reason.
    pub terminal_reason: TerminalReason,
    /// Consent state.
    pub consent: ConsentState,
    /// Last presentation feedback code.
    pub feedback: FeedbackCode,
    /// Release blocker bit mask.
    pub release_blockers: u16,
    /// Canonical deterministic state hash.
    pub state_hash: u64,
}

/// Release blocker: trust below 90.
pub const BLOCK_TRUST: u16 = 1 << 0;
/// Release blocker: risk above 20.
pub const BLOCK_RISK: u16 = 1 << 1;
/// Release blocker: integrity below 80.
pub const BLOCK_INTEGRITY: u16 = 1 << 2;
/// Release blocker: evidence below L2.
pub const BLOCK_EVIDENCE: u16 = 1 << 3;
/// Release blocker: one or more review gates open.
pub const BLOCK_REVIEW_GATES: u16 = 1 << 4;
/// Release blocker: consent inactive.
pub const BLOCK_CONSENT: u16 = 1 << 5;
/// Release blocker: raw leak occurred.
pub const BLOCK_RAW_LEAK: u16 = 1 << 6;
/// Release blocker: run already terminal.
pub const BLOCK_TERMINAL: u16 = 1 << 7;
/// Release blocker: the mode-specific review window is not complete.
pub const BLOCK_REVIEW_WINDOW: u16 = 1 << 8;
/// Release blocker: unresolved entities remain inside the field.
pub const BLOCK_ACTIVE_ENTITIES: u16 = 1 << 9;

/// Deterministic input event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputEvent {
    /// Tick at which the event applies.
    pub tick: u32,
    /// Lane selected before applying the action.
    pub lane: u8,
    /// Action.
    pub action: BoundaryAction,
}

/// Allocation-free deterministic simulation.
#[derive(Clone, Debug)]
pub struct Simulation {
    config: SimulationConfig,
    seed: u64,
    rng_state: u64,
    tick: u32,
    next_spawn_tick: u32,
    spawn_index: u32,
    next_entity_id: u32,
    selected_lane: u8,
    trust: u8,
    risk: u8,
    integrity: u8,
    evidence: EvidenceLevel,
    review_gates: u8,
    consent: ConsentState,
    raw_leaks: u8,
    privacy_containments: u16,
    score: u32,
    streak: u16,
    best_streak: u16,
    status: BoundaryStatus,
    terminal_reason: TerminalReason,
    feedback: FeedbackCode,
    entities: [Entity; ENTITY_CAPACITY],
}

impl Simulation {
    /// Create an authoritative simulation.
    #[must_use]
    pub fn new(mut config: SimulationConfig) -> Self {
        if config.seed == 0 {
            config.seed = 0xA30D_5EED_5AFE_0001;
        }
        if config.max_ticks == 0 {
            config.max_ticks = default_max_ticks(config.mode);
        }
        if config.raw_leak_limit == 0 {
            config.raw_leak_limit = 3;
        }

        let first_spawn = if config.mode == RunMode::Guided { 60 } else { 45 };
        Self {
            config,
            seed: config.seed,
            rng_state: config.seed,
            tick: 0,
            next_spawn_tick: first_spawn,
            spawn_index: 0,
            next_entity_id: 1,
            selected_lane: 2,
            trust: 60,
            risk: 18,
            integrity: 100,
            evidence: EvidenceLevel::L0,
            review_gates: 0,
            consent: ConsentState::none(),
            raw_leaks: 0,
            privacy_containments: 0,
            score: 0,
            streak: 0,
            best_streak: 0,
            status: BoundaryStatus::Open,
            terminal_reason: TerminalReason::None,
            feedback: FeedbackCode::None,
            entities: [Entity::EMPTY; ENTITY_CAPACITY],
        }
    }

    /// Return the immutable configuration.
    #[must_use]
    pub const fn config(&self) -> SimulationConfig {
        self.config
    }

    /// Advance exactly one fixed simulation tick.
    pub fn tick(&mut self) {
        if self.is_terminal() {
            return;
        }

        self.tick = self.tick.saturating_add(1);
        self.feedback = FeedbackCode::None;

        if self.consent.active && self.tick >= self.consent.expiry_tick {
            self.consent = ConsentState::none();
            self.review_gates &= !GATE_CONSENT;
            self.feedback = FeedbackCode::ConsentExpired;
        }

        if self.tick >= self.next_spawn_tick {
            self.spawn_next();
            self.next_spawn_tick = self
                .next_spawn_tick
                .saturating_add(spawn_interval(self.config.mode, self.config.difficulty));
        }

        self.advance_entities();
        self.refresh_dynamic_gates();
        self.enforce_terminal_invariants();

        if !self.is_terminal() && self.tick >= self.config.max_ticks {
            self.status = BoundaryStatus::Degraded;
            self.terminal_reason = TerminalReason::TimeExpired;
            self.feedback = FeedbackCode::TimeExpired;
        }
    }

    /// Advance a bounded number of fixed ticks.
    pub fn tick_many(&mut self, count: u32) {
        let mut remaining = count;
        while remaining > 0 && !self.is_terminal() {
            self.tick();
            remaining -= 1;
        }
    }

    /// Apply an input event at the current tick.
    pub fn apply_action(&mut self, action: BoundaryAction) {
        if self.is_terminal() || action == BoundaryAction::None {
            return;
        }
        if action == BoundaryAction::Release {
            self.attempt_release();
            return;
        }

        let Some(index) = self.target_index(self.selected_lane) else {
            self.feedback = FeedbackCode::NoTarget;
            self.penalize_incorrect(1, 0);
            return;
        };

        let kind = self.entities[index].kind;
        match action {
            BoundaryAction::Validate => self.action_validate(index, kind),
            BoundaryAction::Convert => self.action_convert(index, kind),
            BoundaryAction::Quarantine => self.action_quarantine(index, kind),
            BoundaryAction::Consent => self.action_consent(index, kind),
            BoundaryAction::Evidence => self.action_evidence(index, kind),
            BoundaryAction::Release | BoundaryAction::None => {}
        }
        self.refresh_dynamic_gates();
        self.enforce_terminal_invariants();
    }

    /// Apply a ticked replay event, advancing first when necessary.
    pub fn apply_event(&mut self, event: InputEvent) {
        if event.tick < self.tick || self.is_terminal() {
            return;
        }
        self.tick_many(event.tick - self.tick);
        self.select_lane(event.lane);
        self.apply_action(event.action);
    }

    /// Select a lane. Values outside the field are clamped.
    pub fn select_lane(&mut self, lane: u8) {
        self.selected_lane = min(lane as usize, LANE_COUNT - 1) as u8;
    }

    /// Move selection by a signed delta with wraparound.
    pub fn move_lane(&mut self, delta: i8) {
        let current = self.selected_lane as i8;
        let lanes = LANE_COUNT as i8;
        let next = (current + delta).rem_euclid(lanes);
        self.selected_lane = next as u8;
    }

    /// Current state snapshot including canonical hash.
    #[must_use]
    pub fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            tick: self.tick,
            selected_lane: self.selected_lane,
            trust: self.trust,
            risk: self.risk,
            integrity: self.integrity,
            evidence: self.evidence,
            review_gates: self.review_gates,
            raw_leaks: self.raw_leaks,
            score: self.score,
            streak: self.streak,
            best_streak: self.best_streak,
            status: self.status,
            terminal_reason: self.terminal_reason,
            consent: self.consent,
            feedback: self.feedback,
            release_blockers: self.release_blockers(),
            state_hash: self.state_hash(),
        }
    }

    /// Entity slot by index.
    #[must_use]
    pub fn entity(&self, index: usize) -> Option<Entity> {
        self.entities.get(index).copied().filter(|entity| entity.active)
    }

    /// Number of active entities.
    #[must_use]
    pub fn active_entity_count(&self) -> usize {
        self.entities.iter().filter(|entity| entity.active).count()
    }

    /// Whether the run has reached a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.terminal_reason as u8 != TerminalReason::None as u8
    }

    /// Whether every release invariant currently passes.
    #[must_use]
    pub fn release_ready(&self) -> bool {
        self.release_blockers() == 0
    }

    /// Current release blocker mask.
    #[must_use]
    pub fn release_blockers(&self) -> u16 {
        let mut mask = 0_u16;
        if self.trust < 90 {
            mask |= BLOCK_TRUST;
        }
        if self.risk > 20 {
            mask |= BLOCK_RISK;
        }
        if self.integrity < 80 {
            mask |= BLOCK_INTEGRITY;
        }
        if self.evidence < EvidenceLevel::L2 {
            mask |= BLOCK_EVIDENCE;
        }
        if self.review_gates & ALL_REVIEW_GATES != ALL_REVIEW_GATES {
            mask |= BLOCK_REVIEW_GATES;
        }
        if !self.consent.allows(CONSENT_SCOPE_RELEASE) {
            mask |= BLOCK_CONSENT;
        }
        if self.raw_leaks > 0 {
            mask |= BLOCK_RAW_LEAK;
        }
        if self.is_terminal() {
            mask |= BLOCK_TERMINAL;
        }
        if self.tick < minimum_release_tick(self.config.mode) {
            mask |= BLOCK_REVIEW_WINDOW;
        }
        if self.active_entity_count() != 0 {
            mask |= BLOCK_ACTIVE_ENTITIES;
        }
        mask
    }

    /// Deterministic FNV-1a hash over the canonical state encoding.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        hash_u8(&mut hash, 1); // canonical encoding version
        hash_u64(&mut hash, self.seed);
        hash_u8(&mut hash, self.config.mode as u8);
        hash_u8(&mut hash, self.config.difficulty as u8);
        hash_u32(&mut hash, self.config.max_ticks);
        hash_u8(&mut hash, self.config.raw_leak_limit);
        hash_u32(&mut hash, self.tick);
        hash_u32(&mut hash, self.next_spawn_tick);
        hash_u32(&mut hash, self.spawn_index);
        hash_u32(&mut hash, self.next_entity_id);
        hash_u64(&mut hash, self.rng_state);
        hash_u8(&mut hash, self.selected_lane);
        hash_u8(&mut hash, self.trust);
        hash_u8(&mut hash, self.risk);
        hash_u8(&mut hash, self.integrity);
        hash_u8(&mut hash, self.evidence as u8);
        hash_u8(&mut hash, self.review_gates);
        hash_bool(&mut hash, self.consent.active);
        hash_u8(&mut hash, self.consent.scope);
        hash_u32(&mut hash, self.consent.activation_tick);
        hash_u32(&mut hash, self.consent.expiry_tick);
        hash_u8(&mut hash, self.raw_leaks);
        hash_u16(&mut hash, self.privacy_containments);
        hash_u32(&mut hash, self.score);
        hash_u16(&mut hash, self.streak);
        hash_u16(&mut hash, self.best_streak);
        hash_u8(&mut hash, self.status as u8);
        hash_u8(&mut hash, self.terminal_reason as u8);
        hash_u8(&mut hash, self.feedback as u8);
        for entity in &self.entities {
            hash_bool(&mut hash, entity.active);
            hash_u32(&mut hash, entity.id);
            hash_u8(&mut hash, entity.kind as u8);
            hash_u8(&mut hash, entity.lane);
            hash_u16(&mut hash, entity.position);
            hash_u16(&mut hash, entity.speed);
            hash_u8(&mut hash, entity.flags);
            hash_u32(&mut hash, entity.spawn_tick);
        }
        hash
    }

    fn action_validate(&mut self, index: usize, kind: EntityKind) {
        if kind != EntityKind::UnknownPacket {
            self.feedback = FeedbackCode::IncorrectAction;
            self.penalize_incorrect(2, 1);
            return;
        }
        if self.entities[index].flags & 1 == 1 {
            self.entities[index].kind = EntityKind::ValidatedIntent;
            self.feedback = FeedbackCode::IntentValidated;
            self.reward(4, 0, 70);
        } else {
            self.entities[index].kind = EntityKind::Artifact;
            self.feedback = FeedbackCode::FalseIntentDetected;
            self.reward(3, 0, 60);
        }
    }

    fn action_convert(&mut self, index: usize, kind: EntityKind) {
        if kind != EntityKind::ValidatedIntent {
            self.feedback = FeedbackCode::IncorrectAction;
            self.penalize_incorrect(2, 1);
            return;
        }
        if !self.consent.allows(CONSENT_SCOPE_CONVERT) || self.evidence < EvidenceLevel::L1 {
            self.feedback = FeedbackCode::ConversionBlocked;
            self.penalize_incorrect(4, 1);
            return;
        }
        self.entities[index].kind = EntityKind::TypedIntent;
        self.review_gates |= GATE_TYPE_SAFETY;
        self.feedback = FeedbackCode::IntentConverted;
        self.reward(6, 2, 130);
    }

    fn action_quarantine(&mut self, index: usize, kind: EntityKind) {
        if !kind.is_quarantine_target() {
            self.feedback = FeedbackCode::IncorrectAction;
            self.penalize_incorrect(2, 1);
            return;
        }
        self.entities[index].active = false;
        if matches!(kind, EntityKind::RawFrame | EntityKind::Artifact) {
            self.privacy_containments = self.privacy_containments.saturating_add(1);
        }
        self.feedback = FeedbackCode::Quarantined;
        let score = if kind == EntityKind::StimulationCommand { 180 } else { 100 };
        self.reward(5, 3, score);
    }

    fn action_consent(&mut self, index: usize, kind: EntityKind) {
        match kind {
            EntityKind::ConsentToken => {
                self.entities[index].active = false;
                self.consent = ConsentState {
                    active: true,
                    scope: CONSENT_SCOPE_REQUIRED,
                    activation_tick: self.tick,
                    expiry_tick: self.tick.saturating_add(1_500),
                };
                self.feedback = FeedbackCode::ConsentGranted;
                self.reward(4, 2, 90);
            }
            EntityKind::RevokedConsent => {
                self.entities[index].active = false;
                self.consent = ConsentState::none();
                self.review_gates &= !GATE_CONSENT;
                self.feedback = FeedbackCode::ConsentRevoked;
                self.reward(3, 2, 100);
            }
            _ => {
                self.feedback = FeedbackCode::IncorrectAction;
                self.penalize_incorrect(2, 1);
            }
        }
    }

    fn action_evidence(&mut self, index: usize, kind: EntityKind) {
        let next = match kind {
            EntityKind::Evidence => max(self.evidence, EvidenceLevel::L1),
            EntityKind::Checksum => max(self.evidence, EvidenceLevel::L2),
            EntityKind::CiTest => EvidenceLevel::L3,
            _ => {
                self.feedback = FeedbackCode::IncorrectAction;
                self.penalize_incorrect(2, 1);
                return;
            }
        };
        self.entities[index].active = false;
        self.evidence = next;
        self.feedback = FeedbackCode::EvidenceRegistered;
        let score = match kind {
            EntityKind::Evidence => 80,
            EntityKind::Checksum => 110,
            EntityKind::CiTest => 140,
            _ => 0,
        };
        self.reward(4, 2, score);
    }

    fn attempt_release(&mut self) {
        self.refresh_dynamic_gates();
        let blockers = self.release_blockers();
        if blockers == 0 {
            self.status = BoundaryStatus::Sealed;
            self.terminal_reason = TerminalReason::Released;
            self.feedback = FeedbackCode::ReleaseSealed;
            self.score = self.score.saturating_add(1_000);
        } else {
            self.feedback = FeedbackCode::ReleaseBlocked;
            self.risk = min(100, self.risk.saturating_add(1));
            self.streak = 0;
        }
    }

    fn reward(&mut self, trust_gain: u8, risk_reduction: u8, score: u32) {
        self.trust = min(100, self.trust.saturating_add(trust_gain));
        self.risk = self.risk.saturating_sub(risk_reduction);
        self.score = self.score.saturating_add(score);
        self.streak = self.streak.saturating_add(1);
        self.best_streak = max(self.best_streak, self.streak);
    }

    fn penalize_incorrect(&mut self, risk_gain: u8, integrity_loss: u8) {
        self.risk = min(100, self.risk.saturating_add(risk_gain));
        self.integrity = self.integrity.saturating_sub(integrity_loss);
        self.streak = 0;
    }

    fn refresh_dynamic_gates(&mut self) {
        if self.consent.allows(CONSENT_SCOPE_REQUIRED) {
            self.review_gates |= GATE_CONSENT;
        } else {
            self.review_gates &= !GATE_CONSENT;
        }
        if self.evidence >= EvidenceLevel::L2 {
            self.review_gates |= GATE_EVIDENCE;
        }
        if self.evidence >= EvidenceLevel::L3 {
            self.review_gates |= GATE_DETERMINISM;
        }
        if self.privacy_containments > 0 && self.raw_leaks == 0 {
            self.review_gates |= GATE_PRIVACY;
        } else if self.raw_leaks > 0 {
            self.review_gates &= !GATE_PRIVACY;
        }
    }

    fn enforce_terminal_invariants(&mut self) {
        if self.is_terminal() {
            return;
        }
        if self.raw_leaks >= self.config.raw_leak_limit {
            self.status = BoundaryStatus::Breached;
            self.terminal_reason = TerminalReason::RawLeakLimit;
        } else if self.integrity == 0 {
            self.status = BoundaryStatus::Unsafe;
            self.terminal_reason = TerminalReason::IntegrityCollapse;
        } else if self.risk >= 100 {
            self.status = BoundaryStatus::Unsafe;
            self.terminal_reason = TerminalReason::RiskOverflow;
        }
    }

    fn target_index(&self, lane: u8) -> Option<usize> {
        let mut best: Option<(usize, u16)> = None;
        for (index, entity) in self.entities.iter().enumerate() {
            if !entity.active || entity.lane != lane || entity.kind == EntityKind::TypedIntent {
                continue;
            }
            match best {
                Some((_, position)) if position >= entity.position => {}
                _ => best = Some((index, entity.position)),
            }
        }
        best.map(|(index, _)| index)
    }

    fn spawn_next(&mut self) {
        let kind = kind_for_spawn(self.config.mode, self.spawn_index, self.seed);
        let lane = if self.config.mode == RunMode::Guided {
            guided_lane(self.spawn_index)
        } else {
            (self.next_random() % LANE_COUNT as u64) as u8
        };
        let mut flags = 0_u8;
        if kind == EntityKind::UnknownPacket {
            let candidate = if self.config.mode == RunMode::Guided {
                guided_unknown_is_intent(self.spawn_index)
            } else {
                self.spawn_index % 3 != 1
            };
            if candidate {
                flags |= 1;
            }
        }
        let speed = entity_speed(kind, self.config.mode, self.config.difficulty);
        if let Some(slot) = self.entities.iter_mut().find(|entity| !entity.active) {
            *slot = Entity {
                active: true,
                id: self.next_entity_id,
                kind,
                lane,
                position: SPAWN_X,
                speed,
                flags,
                spawn_tick: self.tick,
            };
            self.next_entity_id = self.next_entity_id.saturating_add(1);
            self.spawn_index = self.spawn_index.saturating_add(1);
        } else {
            self.risk = min(100, self.risk.saturating_add(5));
            self.integrity = self.integrity.saturating_sub(2);
        }
    }

    fn advance_entities(&mut self) {
        let mut crossed = [Entity::EMPTY; ENTITY_CAPACITY];
        let mut crossed_count = 0_usize;
        for entity in &mut self.entities {
            if !entity.active {
                continue;
            }
            entity.position = entity.position.saturating_add(entity.speed);
            if entity.position >= BOUNDARY_X {
                crossed[crossed_count] = *entity;
                crossed_count += 1;
                entity.active = false;
            }
        }
        for entity in crossed.iter().take(crossed_count) {
            self.handle_crossing(*entity);
            if self.is_terminal() {
                break;
            }
        }
    }

    fn handle_crossing(&mut self, entity: Entity) {
        match entity.kind {
            EntityKind::TypedIntent => {
                self.feedback = FeedbackCode::TypedIntentReleased;
                self.reward(3, 1, 120);
            }
            EntityKind::RawFrame => {
                self.raw_leaks = self.raw_leaks.saturating_add(1);
                self.risk = min(100, self.risk.saturating_add(24));
                self.integrity = self.integrity.saturating_sub(14);
                self.streak = 0;
                self.feedback = FeedbackCode::RawLeak;
            }
            EntityKind::StimulationCommand => {
                self.status = BoundaryStatus::Unsafe;
                self.terminal_reason = TerminalReason::StimulationCrossed;
                self.feedback = FeedbackCode::StimulationBreach;
            }
            EntityKind::Artifact | EntityKind::UnknownPacket | EntityKind::ValidatedIntent => {
                self.risk = min(100, self.risk.saturating_add(12));
                self.integrity = self.integrity.saturating_sub(7);
                self.streak = 0;
            }
            EntityKind::UnsupportedClaim
            | EntityKind::UntraceableClaim
            | EntityKind::RoadmapAsFactClaim => {
                self.risk = min(100, self.risk.saturating_add(15));
                self.integrity = self.integrity.saturating_sub(6);
                self.streak = 0;
            }
            EntityKind::RevokedConsent => {
                self.consent = ConsentState::none();
                self.review_gates &= !GATE_CONSENT;
                self.risk = min(100, self.risk.saturating_add(10));
                self.integrity = self.integrity.saturating_sub(3);
                self.streak = 0;
                self.feedback = FeedbackCode::ConsentRevoked;
            }
            EntityKind::ConsentToken => {
                self.risk = min(100, self.risk.saturating_add(5));
                self.streak = 0;
            }
            EntityKind::Evidence | EntityKind::Checksum | EntityKind::CiTest => {
                self.risk = min(100, self.risk.saturating_add(4));
                self.streak = 0;
            }
        }
    }

    fn next_random(&mut self) -> u64 {
        // xorshift64* with a fixed algorithm and nonzero state.
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// Deterministically derive a Daily Seed from a UTC date represented as YYYYMMDD.
#[must_use]
pub fn daily_seed(yyyymmdd: u32) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in b"neural-boundary-replay-v3.0.0" {
        hash_byte(&mut hash, *byte);
    }
    hash_u32(&mut hash, yyyymmdd);
    if hash == 0 { 1 } else { hash }
}

fn minimum_release_tick(mode: RunMode) -> u32 {
    match mode {
        RunMode::Guided => 3_250,
        RunMode::Standard | RunMode::Daily => 1_500,
        RunMode::Audit => 1_800,
        RunMode::Grand => 2_400,
    }
}

fn default_max_ticks(mode: RunMode) -> u32 {
    match mode {
        RunMode::Guided => 3_600,
        RunMode::Standard | RunMode::Daily => 5_400,
        RunMode::Audit => 6_000,
        RunMode::Grand => 7_200,
    }
}

fn spawn_interval(mode: RunMode, difficulty: Difficulty) -> u32 {
    if mode == RunMode::Guided {
        return 210;
    }
    let base: u32 = match mode {
        RunMode::Audit => 105,
        RunMode::Grand => 120,
        RunMode::Standard | RunMode::Daily => 135,
        RunMode::Guided => 210,
    };
    match difficulty {
        Difficulty::Assisted => base.saturating_add(35),
        Difficulty::Standard => base,
        Difficulty::Expert => base.saturating_sub(25),
    }
}

fn entity_speed(kind: EntityKind, mode: RunMode, difficulty: Difficulty) -> u16 {
    let mut speed: u16 = match kind {
        EntityKind::StimulationCommand => 7,
        EntityKind::UnsupportedClaim
        | EntityKind::UntraceableClaim
        | EntityKind::RoadmapAsFactClaim => 6,
        EntityKind::RawFrame | EntityKind::Artifact => 5,
        _ => 4,
    };
    if matches!(mode, RunMode::Audit | RunMode::Grand) {
        speed = speed.saturating_add(1);
    }
    match difficulty {
        Difficulty::Assisted => speed.saturating_sub(1).max(2),
        Difficulty::Standard => speed,
        Difficulty::Expert => speed.saturating_add(1),
    }
}

fn guided_lane(index: u32) -> u8 {
    const LANES: [u8; 16] = [2, 1, 3, 2, 4, 0, 1, 3, 2, 1, 4, 0, 2, 4, 1, 3];
    LANES[index as usize % LANES.len()]
}

fn guided_unknown_is_intent(index: u32) -> bool {
    !matches!(index, 8)
}

fn kind_for_spawn(mode: RunMode, index: u32, seed: u64) -> EntityKind {
    const GUIDED: [EntityKind; 16] = [
        EntityKind::RawFrame,
        EntityKind::ConsentToken,
        EntityKind::Evidence,
        EntityKind::UnknownPacket,
        EntityKind::Checksum,
        EntityKind::CiTest,
        EntityKind::UnsupportedClaim,
        EntityKind::RevokedConsent,
        EntityKind::UnknownPacket,
        EntityKind::ConsentToken,
        EntityKind::Evidence,
        EntityKind::RoadmapAsFactClaim,
        EntityKind::UnknownPacket,
        EntityKind::StimulationCommand,
        EntityKind::Checksum,
        EntityKind::UntraceableClaim,
    ];
    const STANDARD: [EntityKind; 16] = [
        EntityKind::RawFrame,
        EntityKind::ConsentToken,
        EntityKind::Evidence,
        EntityKind::UnknownPacket,
        EntityKind::Artifact,
        EntityKind::Checksum,
        EntityKind::UnknownPacket,
        EntityKind::UnsupportedClaim,
        EntityKind::CiTest,
        EntityKind::ConsentToken,
        EntityKind::RawFrame,
        EntityKind::UnknownPacket,
        EntityKind::Evidence,
        EntityKind::UntraceableClaim,
        EntityKind::Checksum,
        EntityKind::UnknownPacket,
    ];
    const AUDIT: [EntityKind; 18] = [
        EntityKind::RawFrame,
        EntityKind::UnknownPacket,
        EntityKind::ConsentToken,
        EntityKind::Evidence,
        EntityKind::RevokedConsent,
        EntityKind::UnsupportedClaim,
        EntityKind::Checksum,
        EntityKind::UnknownPacket,
        EntityKind::StimulationCommand,
        EntityKind::ConsentToken,
        EntityKind::CiTest,
        EntityKind::RoadmapAsFactClaim,
        EntityKind::UnknownPacket,
        EntityKind::RawFrame,
        EntityKind::UntraceableClaim,
        EntityKind::Evidence,
        EntityKind::Artifact,
        EntityKind::UnknownPacket,
    ];
    const GRAND: [EntityKind; 20] = [
        EntityKind::RawFrame,
        EntityKind::Artifact,
        EntityKind::UnknownPacket,
        EntityKind::ConsentToken,
        EntityKind::Evidence,
        EntityKind::Checksum,
        EntityKind::UnknownPacket,
        EntityKind::RevokedConsent,
        EntityKind::UnsupportedClaim,
        EntityKind::ConsentToken,
        EntityKind::CiTest,
        EntityKind::UnknownPacket,
        EntityKind::StimulationCommand,
        EntityKind::RoadmapAsFactClaim,
        EntityKind::RawFrame,
        EntityKind::Evidence,
        EntityKind::UnknownPacket,
        EntityKind::UntraceableClaim,
        EntityKind::Checksum,
        EntityKind::UnknownPacket,
    ];

    match mode {
        RunMode::Guided => GUIDED[index as usize % GUIDED.len()],
        RunMode::Standard => STANDARD[index as usize % STANDARD.len()],
        RunMode::Audit => AUDIT[index as usize % AUDIT.len()],
        RunMode::Grand => GRAND[index as usize % GRAND.len()],
        RunMode::Daily => {
            let offset = (seed % STANDARD.len() as u64) as usize;
            STANDARD[(index as usize + offset) % STANDARD.len()]
        }
    }
}

fn hash_byte(hash: &mut u64, byte: u8) {
    *hash ^= u64::from(byte);
    *hash = (*hash).wrapping_mul(0x0000_0100_0000_01B3);
}

fn hash_bool(hash: &mut u64, value: bool) {
    hash_byte(hash, u8::from(value));
}

fn hash_u8(hash: &mut u64, value: u8) {
    hash_byte(hash, value);
}

fn hash_u16(hash: &mut u64, value: u16) {
    for byte in value.to_le_bytes() {
        hash_byte(hash, byte);
    }
}

fn hash_u32(hash: &mut u64, value: u32) {
    for byte in value.to_le_bytes() {
        hash_byte(hash, byte);
    }
}

fn hash_u64(hash: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        hash_byte(hash, byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guided_event(tick: u32, lane: u8, action: BoundaryAction) -> InputEvent {
        InputEvent { tick, lane, action }
    }

    #[test]
    fn same_seed_and_inputs_produce_same_hash() {
        let config = SimulationConfig::canonical(58, RunMode::Guided, Difficulty::Standard);
        let events = [
            guided_event(70, 2, BoundaryAction::Quarantine),
            guided_event(280, 1, BoundaryAction::Consent),
            guided_event(490, 3, BoundaryAction::Evidence),
            guided_event(700, 2, BoundaryAction::Validate),
            guided_event(701, 2, BoundaryAction::Convert),
        ];
        let mut left = Simulation::new(config);
        let mut right = Simulation::new(config);
        for event in events {
            left.apply_event(event);
            right.apply_event(event);
        }
        left.tick_many(100);
        right.tick_many(100);
        assert_eq!(left.snapshot(), right.snapshot());
        assert_eq!(left.state_hash(), right.state_hash());
    }

    #[test]
    fn stimulation_crossing_fails_closed() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            9,
            RunMode::Guided,
            Difficulty::Standard,
        ));
        simulation.tick_many(3_100);
        assert_eq!(simulation.snapshot().status, BoundaryStatus::Unsafe);
        assert_eq!(
            simulation.snapshot().terminal_reason,
            TerminalReason::StimulationCrossed
        );
    }

    #[test]
    fn raw_leak_limit_is_terminal() {
        let mut config = SimulationConfig::canonical(58, RunMode::Guided, Difficulty::Standard);
        config.raw_leak_limit = 1;
        let mut simulation = Simulation::new(config);
        simulation.tick_many(400);
        assert_eq!(simulation.snapshot().status, BoundaryStatus::Breached);
        assert_eq!(
            simulation.snapshot().terminal_reason,
            TerminalReason::RawLeakLimit
        );
    }

    #[test]
    fn daily_seed_is_stable_and_date_specific() {
        assert_eq!(daily_seed(20260611), daily_seed(20260611));
        assert_ne!(daily_seed(20260611), daily_seed(20260612));
    }

    fn test_entity(kind: EntityKind, lane: u8, position: u16) -> Entity {
        Entity {
            active: true,
            id: 1,
            kind,
            lane,
            position,
            speed: 1,
            flags: 1,
            spawn_tick: 0,
        }
    }

    #[test]
    fn raw_frame_cannot_be_converted_to_typed_intent() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            3,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.entities[0] = test_entity(EntityKind::RawFrame, 0, 100);
        simulation.select_lane(0);
        simulation.apply_action(BoundaryAction::Convert);

        assert_eq!(simulation.entities[0].kind, EntityKind::RawFrame);
        assert_eq!(simulation.feedback, FeedbackCode::IncorrectAction);
    }

    #[test]
    fn conversion_requires_active_consent_and_evidence() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            4,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.entities[0] = test_entity(EntityKind::ValidatedIntent, 1, 100);
        simulation.select_lane(1);
        simulation.apply_action(BoundaryAction::Convert);
        assert_eq!(simulation.entities[0].kind, EntityKind::ValidatedIntent);
        assert_eq!(simulation.feedback, FeedbackCode::ConversionBlocked);

        simulation.consent = ConsentState {
            active: true,
            scope: CONSENT_SCOPE_REQUIRED,
            activation_tick: 0,
            expiry_tick: 1_000,
        };
        simulation.evidence = EvidenceLevel::L1;
        simulation.apply_action(BoundaryAction::Convert);
        assert_eq!(simulation.entities[0].kind, EntityKind::TypedIntent);
        assert_eq!(simulation.feedback, FeedbackCode::IntentConverted);
    }

    #[test]
    fn revocation_blocks_subsequent_conversion() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            5,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.consent = ConsentState {
            active: true,
            scope: CONSENT_SCOPE_REQUIRED,
            activation_tick: 0,
            expiry_tick: 1_000,
        };
        simulation.evidence = EvidenceLevel::L1;
        simulation.entities[0] = test_entity(EntityKind::RevokedConsent, 2, 200);
        simulation.select_lane(2);
        simulation.apply_action(BoundaryAction::Consent);
        assert!(!simulation.consent.active);

        simulation.entities[0] = test_entity(EntityKind::ValidatedIntent, 2, 200);
        simulation.apply_action(BoundaryAction::Convert);
        assert_eq!(simulation.entities[0].kind, EntityKind::ValidatedIntent);
        assert_eq!(simulation.feedback, FeedbackCode::ConversionBlocked);
    }

    #[test]
    fn unsupported_claim_cannot_raise_evidence() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            6,
            RunMode::Audit,
            Difficulty::Standard,
        ));
        simulation.entities[0] = test_entity(EntityKind::UnsupportedClaim, 3, 100);
        simulation.select_lane(3);
        simulation.apply_action(BoundaryAction::Evidence);

        assert_eq!(simulation.evidence, EvidenceLevel::L0);
        assert!(simulation.entities[0].active);
        assert_eq!(simulation.feedback, FeedbackCode::IncorrectAction);
    }

    #[test]
    fn raw_leak_is_an_absolute_release_blocker() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            7,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.tick = minimum_release_tick(RunMode::Standard);
        simulation.trust = 100;
        simulation.risk = 0;
        simulation.integrity = 100;
        simulation.evidence = EvidenceLevel::L3;
        simulation.review_gates = ALL_REVIEW_GATES;
        simulation.consent = ConsentState {
            active: true,
            scope: CONSENT_SCOPE_REQUIRED,
            activation_tick: 0,
            expiry_tick: simulation.tick + 100,
        };
        simulation.raw_leaks = 1;

        assert_eq!(simulation.release_blockers(), BLOCK_RAW_LEAK);
        simulation.apply_action(BoundaryAction::Release);
        assert_eq!(simulation.status, BoundaryStatus::Open);
        assert_eq!(simulation.feedback, FeedbackCode::ReleaseBlocked);
    }

    #[test]
    fn exact_release_invariants_seal_the_run() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            8,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.tick = minimum_release_tick(RunMode::Standard);
        simulation.trust = 90;
        simulation.risk = 20;
        simulation.integrity = 80;
        simulation.evidence = EvidenceLevel::L3;
        simulation.review_gates = ALL_REVIEW_GATES;
        simulation.privacy_containments = 1;
        simulation.consent = ConsentState {
            active: true,
            scope: CONSENT_SCOPE_REQUIRED,
            activation_tick: 0,
            expiry_tick: simulation.tick + 100,
        };

        simulation.apply_action(BoundaryAction::Release);
        assert_eq!(simulation.status, BoundaryStatus::Sealed);
        assert_eq!(simulation.terminal_reason, TerminalReason::Released);
        assert_eq!(simulation.feedback, FeedbackCode::ReleaseSealed);
    }

    #[test]
    fn conversion_requires_the_conversion_scope_bit() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            9,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.entities[0] = test_entity(EntityKind::ValidatedIntent, 1, 100);
        simulation.selected_lane = 1;
        simulation.evidence = EvidenceLevel::L1;
        simulation.consent = ConsentState {
            active: true,
            scope: CONSENT_SCOPE_RELEASE,
            activation_tick: 0,
            expiry_tick: 1_000,
        };

        simulation.apply_action(BoundaryAction::Convert);
        assert_eq!(simulation.entities[0].kind, EntityKind::ValidatedIntent);
        assert_eq!(simulation.feedback, FeedbackCode::ConversionBlocked);
    }

    #[test]
    fn release_requires_the_release_scope_bit() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            9,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.tick = minimum_release_tick(RunMode::Standard);
        simulation.trust = 100;
        simulation.risk = 0;
        simulation.integrity = 100;
        simulation.evidence = EvidenceLevel::L3;
        simulation.review_gates = ALL_REVIEW_GATES;
        simulation.privacy_containments = 1;
        simulation.consent = ConsentState {
            active: true,
            scope: CONSENT_SCOPE_CONVERT,
            activation_tick: 0,
            expiry_tick: simulation.tick + 100,
        };

        assert_eq!(simulation.release_blockers(), BLOCK_CONSENT);
        simulation.apply_action(BoundaryAction::Release);
        assert_eq!(simulation.status, BoundaryStatus::Open);
        assert_eq!(simulation.feedback, FeedbackCode::ReleaseBlocked);
    }

    #[test]
    fn consent_expiry_clears_scope_and_gate() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            9,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.consent = ConsentState {
            active: true,
            scope: CONSENT_SCOPE_REQUIRED,
            activation_tick: 0,
            expiry_tick: 1,
        };
        simulation.review_gates |= GATE_CONSENT;

        simulation.tick();
        assert!(!simulation.consent.active);
        assert_eq!(simulation.consent.scope, 0);
        assert_eq!(simulation.review_gates & GATE_CONSENT, 0);
        assert_eq!(simulation.feedback, FeedbackCode::ConsentExpired);
    }

    #[test]
    fn unresolved_entities_block_release() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            9,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.tick = minimum_release_tick(RunMode::Standard);
        simulation.trust = 100;
        simulation.risk = 0;
        simulation.integrity = 100;
        simulation.evidence = EvidenceLevel::L3;
        simulation.review_gates = ALL_REVIEW_GATES;
        simulation.privacy_containments = 1;
        simulation.consent = ConsentState {
            active: true,
            scope: CONSENT_SCOPE_REQUIRED,
            activation_tick: 0,
            expiry_tick: simulation.tick + 100,
        };
        simulation.entities[0] = test_entity(EntityKind::TypedIntent, 1, 100);

        assert_eq!(simulation.release_blockers(), BLOCK_ACTIVE_ENTITIES);
        simulation.apply_action(BoundaryAction::Release);
        assert_eq!(simulation.status, BoundaryStatus::Open);
        assert_eq!(simulation.feedback, FeedbackCode::ReleaseBlocked);
    }

    #[test]
    fn terminal_state_is_immutable() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            10,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.status = BoundaryStatus::Unsafe;
        simulation.terminal_reason = TerminalReason::InvariantViolation;
        let before = simulation.state_hash();

        simulation.tick();
        simulation.apply_action(BoundaryAction::Quarantine);
        assert_eq!(simulation.state_hash(), before);
    }

    #[test]
    fn fixed_entity_pool_never_exceeds_capacity() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            11,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        for (index, slot) in simulation.entities.iter_mut().enumerate() {
            *slot = test_entity(EntityKind::Artifact, (index % LANE_COUNT) as u8, 10);
            slot.id = index as u32 + 1;
        }
        let risk_before = simulation.risk;
        let integrity_before = simulation.integrity;
        simulation.spawn_next();

        assert_eq!(simulation.active_entity_count(), ENTITY_CAPACITY);
        assert_eq!(simulation.risk, risk_before + 5);
        assert_eq!(simulation.integrity, integrity_before - 2);
    }

    #[test]
    fn different_seeds_produce_different_canonical_states() {
        let left = Simulation::new(SimulationConfig::canonical(
            12,
            RunMode::Daily,
            Difficulty::Standard,
        ));
        let right = Simulation::new(SimulationConfig::canonical(
            13,
            RunMode::Daily,
            Difficulty::Standard,
        ));
        assert_ne!(left.state_hash(), right.state_hash());
    }

    #[test]
    fn invalid_lane_is_clamped() {
        let mut simulation = Simulation::new(SimulationConfig::canonical(
            1,
            RunMode::Standard,
            Difficulty::Standard,
        ));
        simulation.select_lane(99);
        assert_eq!(simulation.snapshot().selected_lane, 4);
    }
}
