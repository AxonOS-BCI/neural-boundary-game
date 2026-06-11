//! neural-boundary-core — deterministic `no_std` simulation core for the
//! Neural Boundary Game v3.0.1 (Sovereign Boundary Edition).
//!
//! The browser is never authoritative: every rule, number and transition in
//! the game lives here, behind `#![forbid(unsafe_code)]`, with zero heap
//! allocation in the simulation loop. One [`Simulation::step`] per tick at a
//! fixed 60 Hz; identical seed + config + ticked inputs always reproduce the
//! identical terminal tick, terminal reason and 64-bit state hash.
//!
//! Normative documentation: `docs/GAME_SPEC.md` and `docs/REPLAY_SPEC.md`.

#![no_std]
#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Canonical identity
// ---------------------------------------------------------------------------

/// Core crate version, mirrored by `release.toml` and the workspace manifest.
pub const CORE_VERSION: &str = "3.0.1";
/// Canonical replay schema accepted by the verifier.
pub const REPLAY_SCHEMA: &str = "neural-boundary-replay-v3.0.1";
/// State-hash algorithm identifier carried inside replay files.
pub const HASH_ALGORITHM: &str = "fnv1a64-v1";

// ---------------------------------------------------------------------------
// Field geometry and timing
// ---------------------------------------------------------------------------

/// Fixed simulation rate. One [`Simulation::step`] equals one tick.
pub const TICKS_PER_SECOND: u32 = 60;
/// Number of parallel signal lanes.
pub const LANES: u8 = 5;
/// Entities enter the field at this logical x position.
pub const SPAWN_X: i32 = 0;
/// The sovereign boundary (membrane) between signal zone and app zone.
pub const BOUNDARY_X: i32 = 680;
/// The application threshold; typed intent delivered here counts.
pub const FIELD_END_X: i32 = 1000;
/// Actions reach the frontmost entity inside `[BOUNDARY_X - GATE_WINDOW, BOUNDARY_X)`.
pub const GATE_WINDOW: i32 = 320;
/// Fixed entity pool size. No allocation, ever.
pub const MAX_ENTITIES: usize = 24;
/// Fixed per-tick event buffer size.
pub const MAX_EVENTS: usize = 8;
/// Cooldown after a successful action.
pub const ACTION_COOLDOWN: u32 = 18;
/// Cooldown after an action that found no valid target.
pub const WHIFF_COOLDOWN: u32 = 6;
/// Ticks a consent grant stays valid after gating a token.
pub const CONSENT_DURATION: u32 = 1500;
/// Unsupported claims travel faster than evidence.
pub const CLAIM_SPEED_BONUS: i32 = 2;
/// Raw-frame leaks at which the boundary is considered breached.
pub const RAW_LEAK_LIMIT: u8 = 3;
/// Evidence points saturate here.
pub const EVIDENCE_POINTS_MAX: u8 = 12;
/// Streak value at which the score bonus saturates.
pub const STREAK_BONUS_CAP: u32 = 10;

// ---------------------------------------------------------------------------
// Deterministic primitives
// ---------------------------------------------------------------------------

/// xorshift64* — the only randomness source. Owned by the core; never the
/// browser. Seed 0 is remapped to a fixed non-zero constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng64 {
    state: u64,
}

impl Rng64 {
    pub const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform value in `0..bound` (bound > 0).
    pub fn range(&mut self, bound: u32) -> u32 {
        (self.next_u64() % bound as u64) as u32
    }

    pub const fn state(&self) -> u64 {
        self.state
    }
}

/// FNV-1a 64-bit accumulator used for the deterministic state hash.
/// This is an integrity fingerprint for replay verification, not a
/// cryptographic primitive.
#[derive(Clone, Copy, Debug)]
pub struct Fnv64 {
    hash: u64,
}

impl Fnv64 {
    pub const OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    pub const PRIME: u64 = 0x0000_0100_0000_01B3;

    pub const fn new() -> Self {
        Self { hash: Self::OFFSET }
    }

    pub fn write_u8(&mut self, value: u8) {
        self.hash ^= value as u64;
        self.hash = self.hash.wrapping_mul(Self::PRIME);
    }

    pub fn write_u16(&mut self, value: u16) {
        for byte in value.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    pub fn write_u32(&mut self, value: u32) {
        for byte in value.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    pub fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    pub fn write_i32(&mut self, value: i32) {
        self.write_u32(value as u32);
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_u8(*byte);
        }
    }

    pub const fn finish(&self) -> u64 {
        self.hash
    }
}

impl Default for Fnv64 {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic Daily Seed derivation, documented in `docs/GAME_SPEC.md`:
/// FNV-1a over the replay schema string and the little-endian UTC date,
/// finalized through a splitmix64 avalanche. Same date + same schema =
/// same seed, with no backend and no clock access inside the core.
pub fn daily_seed(year: u16, month: u8, day: u8) -> u64 {
    let mut fnv = Fnv64::new();
    fnv.write_bytes(REPLAY_SCHEMA.as_bytes());
    fnv.write_u16(year);
    fnv.write_u8(month);
    fnv.write_u8(day);
    let mut x = fnv.finish();
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    if x == 0 {
        0x3001
    } else {
        x
    }
}

// ---------------------------------------------------------------------------
// Modes and difficulty
// ---------------------------------------------------------------------------

/// Run mode. Modes change pacing, spawn composition, gate targets and
/// release thresholds; they never change the deterministic contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunMode {
    Guided,
    Standard,
    Audit,
    Grand,
    Daily,
}

impl RunMode {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Guided => "guided",
            Self::Standard => "standard",
            Self::Audit => "audit",
            Self::Grand => "grand",
            Self::Daily => "daily",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "guided" => Some(Self::Guided),
            "standard" => Some(Self::Standard),
            "audit" => Some(Self::Audit),
            "grand" => Some(Self::Grand),
            "daily" => Some(Self::Daily),
            _ => None,
        }
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Guided => 0,
            Self::Standard => 1,
            Self::Audit => 2,
            Self::Grand => 3,
            Self::Daily => 4,
        }
    }

    pub const ALL: [Self; 5] = [
        Self::Guided,
        Self::Standard,
        Self::Audit,
        Self::Grand,
        Self::Daily,
    ];
}

/// Difficulty controls spawn cadence and entity speed, orthogonal to mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Calm,
    Standard,
    Intense,
}

impl Difficulty {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Standard => "standard",
            Self::Intense => "intense",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "calm" => Some(Self::Calm),
            "standard" => Some(Self::Standard),
            "intense" => Some(Self::Intense),
            _ => None,
        }
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Calm => 0,
            Self::Standard => 1,
            Self::Intense => 2,
        }
    }

    const fn cadence(self) -> Cadence {
        match self {
            Self::Calm => Cadence {
                base_interval: 68,
                interval_jitter: 20,
                min_interval: 44,
                interval_shrink_every: 700,
                base_speed: 2,
                speed_jitter: 1,
            },
            Self::Standard => Cadence {
                base_interval: 52,
                interval_jitter: 18,
                min_interval: 30,
                interval_shrink_every: 600,
                base_speed: 2,
                speed_jitter: 2,
            },
            Self::Intense => Cadence {
                base_interval: 40,
                interval_jitter: 14,
                min_interval: 22,
                interval_shrink_every: 480,
                base_speed: 2,
                speed_jitter: 3,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Cadence {
    base_interval: u32,
    interval_jitter: u32,
    min_interval: u32,
    interval_shrink_every: u32,
    base_speed: i32,
    speed_jitter: u32,
}

/// Per-mode release thresholds and review-gate targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeRules {
    /// Hard run horizon in ticks; reaching it ends the run as `HorizonReached`.
    pub horizon: u32,
    pub trust_min: i32,
    pub risk_max: i32,
    pub integrity_min: i32,
    pub evidence_points_min: u8,
    pub gate_validations: u8,
    pub gate_quarantines: u8,
    pub gate_deliveries: u8,
    /// Grand Run only: release is accepted only in the final phase.
    pub release_final_phase_only: bool,
}

impl RunMode {
    pub const fn rules(self) -> ModeRules {
        match self {
            Self::Guided => ModeRules {
                horizon: 3_600,
                trust_min: 58,
                risk_max: 40,
                integrity_min: 60,
                evidence_points_min: 2,
                gate_validations: 2,
                gate_quarantines: 2,
                gate_deliveries: 2,
                release_final_phase_only: false,
            },
            Self::Standard | Self::Daily => ModeRules {
                horizon: 9_000,
                trust_min: 90,
                risk_max: 20,
                integrity_min: 80,
                evidence_points_min: 4,
                gate_validations: 3,
                gate_quarantines: 3,
                gate_deliveries: 5,
                release_final_phase_only: false,
            },
            Self::Audit => ModeRules {
                horizon: 9_000,
                trust_min: 90,
                risk_max: 15,
                integrity_min: 85,
                evidence_points_min: 7,
                gate_validations: 4,
                gate_quarantines: 4,
                gate_deliveries: 6,
                release_final_phase_only: false,
            },
            Self::Grand => ModeRules {
                horizon: 6_600,
                trust_min: 90,
                risk_max: 20,
                integrity_min: 80,
                evidence_points_min: 4,
                gate_validations: 3,
                gate_quarantines: 4,
                gate_deliveries: 6,
                release_final_phase_only: true,
            },
        }
    }
}

/// Grand Run phase boundaries in ticks.
pub const GRAND_PHASE_STARTS: [u32; 4] = [0, 1_500, 3_000, 4_500];

/// Grand Run phase names, indexed by phase number.
pub const fn grand_phase_name(phase: u8) -> &'static str {
    match phase {
        0 => "Signal Integrity",
        1 => "Consent and Evidence",
        2 => "Release Under Pressure",
        _ => "Sovereign Boundary Review",
    }
}

// ---------------------------------------------------------------------------
// Entity taxonomy
// ---------------------------------------------------------------------------

/// Risk category used for presentation and accessible descriptions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiskCategory {
    /// Crossing is an immediate or counted boundary breach.
    Critical,
    /// Crossing erodes integrity.
    Hazard,
    /// Unsupported statement; crossing erodes integrity, moves fast.
    Claim,
    /// Useful payload that must be processed before the membrane.
    Payload,
    /// Capability or proof artifact; crossing wastes it.
    Resource,
    /// Must be classified before any other decision.
    Unknown,
    /// The only kind that may legally cross.
    Sealed,
}

impl RiskCategory {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Hazard => "hazard",
            Self::Claim => "claim",
            Self::Payload => "payload",
            Self::Resource => "resource",
            Self::Unknown => "unknown",
            Self::Sealed => "sealed",
        }
    }
}

/// Entity taxonomy with stable numeric IDs (`code`). IDs are part of the
/// replay/state-hash contract and must never be reused or renumbered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    RawFrame,
    Artifact,
    IntentCandidate,
    ValidatedIntent,
    TypedIntent,
    UnknownPacket,
    ConsentToken,
    RevokedConsent,
    Evidence,
    Checksum,
    CiTest,
    UnsupportedClaim,
    UntraceableClaim,
    RoadmapAsFactClaim,
    StimulationCommand,
}

impl EntityKind {
    /// Stable numeric ID.
    pub const fn code(self) -> u8 {
        match self {
            Self::RawFrame => 0,
            Self::Artifact => 1,
            Self::IntentCandidate => 2,
            Self::ValidatedIntent => 3,
            Self::TypedIntent => 4,
            Self::UnknownPacket => 5,
            Self::ConsentToken => 6,
            Self::RevokedConsent => 7,
            Self::Evidence => 8,
            Self::Checksum => 9,
            Self::CiTest => 10,
            Self::UnsupportedClaim => 11,
            Self::UntraceableClaim => 12,
            Self::RoadmapAsFactClaim => 13,
            Self::StimulationCommand => 14,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::RawFrame,
            1 => Self::Artifact,
            2 => Self::IntentCandidate,
            3 => Self::ValidatedIntent,
            4 => Self::TypedIntent,
            5 => Self::UnknownPacket,
            6 => Self::ConsentToken,
            7 => Self::RevokedConsent,
            8 => Self::Evidence,
            9 => Self::Checksum,
            10 => Self::CiTest,
            11 => Self::UnsupportedClaim,
            12 => Self::UntraceableClaim,
            13 => Self::RoadmapAsFactClaim,
            14 => Self::StimulationCommand,
            _ => return None,
        })
    }

    /// Stable schema name used inside replay files and documentation.
    pub const fn schema_name(self) -> &'static str {
        match self {
            Self::RawFrame => "raw_frame",
            Self::Artifact => "artifact",
            Self::IntentCandidate => "intent_candidate",
            Self::ValidatedIntent => "validated_intent",
            Self::TypedIntent => "typed_intent",
            Self::UnknownPacket => "unknown_packet",
            Self::ConsentToken => "consent_token",
            Self::RevokedConsent => "revoked_consent",
            Self::Evidence => "evidence",
            Self::Checksum => "checksum",
            Self::CiTest => "ci_test",
            Self::UnsupportedClaim => "unsupported_claim",
            Self::UntraceableClaim => "untraceable_claim",
            Self::RoadmapAsFactClaim => "roadmap_as_fact_claim",
            Self::StimulationCommand => "stimulation_command",
        }
    }

    /// Short UI label, eight characters or fewer.
    pub const fn label(self) -> &'static str {
        match self {
            Self::RawFrame => "RAW",
            Self::Artifact => "ARTIFACT",
            Self::IntentCandidate => "INTENT",
            Self::ValidatedIntent => "VALID",
            Self::TypedIntent => "TYPED",
            Self::UnknownPacket => "?PKT",
            Self::ConsentToken => "CONSENT",
            Self::RevokedConsent => "REVOKED",
            Self::Evidence => "EVIDENCE",
            Self::Checksum => "CHECKSUM",
            Self::CiTest => "CI TEST",
            Self::UnsupportedClaim => "CLAIM",
            Self::UntraceableClaim => "NO TRACE",
            Self::RoadmapAsFactClaim => "ROADMAP",
            Self::StimulationCommand => "STIM",
        }
    }

    /// Non-color visual symbol; state is never communicated by color alone.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::RawFrame => "◉",
            Self::Artifact => "▒",
            Self::IntentCandidate => "◇",
            Self::ValidatedIntent => "◈",
            Self::TypedIntent => "●",
            Self::UnknownPacket => "◌",
            Self::ConsentToken => "⬡",
            Self::RevokedConsent => "⬢",
            Self::Evidence => "▣",
            Self::Checksum => "▤",
            Self::CiTest => "▥",
            Self::UnsupportedClaim => "△",
            Self::UntraceableClaim => "▽",
            Self::RoadmapAsFactClaim => "◭",
            Self::StimulationCommand => "✕",
        }
    }

    /// Accessible description used for screen-reader labels and help.
    pub const fn description(self) -> &'static str {
        match self {
            Self::RawFrame => "Private raw signal frame. Quarantine it; it must never cross.",
            Self::Artifact => "Noise artifact that resembles intent. Quarantine it.",
            Self::IntentCandidate => "Unverified intent candidate. Validate it first.",
            Self::ValidatedIntent => "Validated intent. Convert it under consent and evidence.",
            Self::TypedIntent => "Typed intent. The only payload allowed across the boundary.",
            Self::UnknownPacket => "Unclassified packet. Validate to reveal what it conceals.",
            Self::ConsentToken => "Scoped consent token. Gate it to enable conversion.",
            Self::RevokedConsent => {
                "Revoked consent credential. Quarantine; it is no longer valid."
            }
            Self::Evidence => "Review evidence worth two points. Register it.",
            Self::Checksum => "Checksum proof worth one point. Register it.",
            Self::CiTest => "CI validation proof worth one point. Register it.",
            Self::UnsupportedClaim => "Unsupported claim. Quarantine; claims never raise evidence.",
            Self::UntraceableClaim => "Claim without a trace. Quarantine it.",
            Self::RoadmapAsFactClaim => "Roadmap stated as fact. Quarantine it.",
            Self::StimulationCommand => "Stimulation command. Fail closed: quarantine immediately.",
        }
    }

    pub const fn risk_category(self) -> RiskCategory {
        match self {
            Self::RawFrame | Self::RevokedConsent | Self::StimulationCommand => {
                RiskCategory::Critical
            }
            Self::Artifact => RiskCategory::Hazard,
            Self::UnsupportedClaim | Self::UntraceableClaim | Self::RoadmapAsFactClaim => {
                RiskCategory::Claim
            }
            Self::IntentCandidate | Self::ValidatedIntent => RiskCategory::Payload,
            Self::ConsentToken | Self::Evidence | Self::Checksum | Self::CiTest => {
                RiskCategory::Resource
            }
            Self::UnknownPacket => RiskCategory::Unknown,
            Self::TypedIntent => RiskCategory::Sealed,
        }
    }

    /// The correct boundary action for this kind, when one exists.
    pub const fn required_action(self) -> Option<Action> {
        match self {
            Self::RawFrame
            | Self::Artifact
            | Self::RevokedConsent
            | Self::UnsupportedClaim
            | Self::UntraceableClaim
            | Self::RoadmapAsFactClaim
            | Self::StimulationCommand => Some(Action::Quarantine),
            Self::IntentCandidate | Self::UnknownPacket => Some(Action::Validate),
            Self::ValidatedIntent => Some(Action::Convert),
            Self::ConsentToken => Some(Action::ConsentGate),
            Self::Evidence | Self::Checksum | Self::CiTest => Some(Action::EvidenceGate),
            Self::TypedIntent => None,
        }
    }

    pub const fn evidence_points(self) -> u8 {
        match self {
            Self::Evidence => 2,
            Self::Checksum | Self::CiTest => 1,
            _ => 0,
        }
    }

    /// Kinds the player must remove with Quarantine.
    pub const fn is_quarantine_target(self) -> bool {
        matches!(
            self,
            Self::RawFrame
                | Self::Artifact
                | Self::RevokedConsent
                | Self::UnsupportedClaim
                | Self::UntraceableClaim
                | Self::RoadmapAsFactClaim
                | Self::StimulationCommand
        )
    }

    const fn is_claim(self) -> bool {
        matches!(
            self,
            Self::UnsupportedClaim | Self::UntraceableClaim | Self::RoadmapAsFactClaim
        )
    }

    pub const ALL: [Self; 15] = [
        Self::RawFrame,
        Self::Artifact,
        Self::IntentCandidate,
        Self::ValidatedIntent,
        Self::TypedIntent,
        Self::UnknownPacket,
        Self::ConsentToken,
        Self::RevokedConsent,
        Self::Evidence,
        Self::Checksum,
        Self::CiTest,
        Self::UnsupportedClaim,
        Self::UntraceableClaim,
        Self::RoadmapAsFactClaim,
        Self::StimulationCommand,
    ];
}

// ---------------------------------------------------------------------------
// Actions, consent, evidence, gates
// ---------------------------------------------------------------------------

/// Player boundary actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Validate,
    Convert,
    Quarantine,
    ConsentGate,
    EvidenceGate,
    Release,
}

impl Action {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Validate => "Validate",
            Self::Convert => "Convert",
            Self::Quarantine => "Quarantine",
            Self::ConsentGate => "ConsentGate",
            Self::EvidenceGate => "EvidenceGate",
            Self::Release => "Release",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Validate" => Some(Self::Validate),
            "Convert" => Some(Self::Convert),
            "Quarantine" => Some(Self::Quarantine),
            "ConsentGate" => Some(Self::ConsentGate),
            "EvidenceGate" => Some(Self::EvidenceGate),
            "Release" => Some(Self::Release),
            _ => None,
        }
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Validate => 0,
            Self::Convert => 1,
            Self::Quarantine => 2,
            Self::ConsentGate => 3,
            Self::EvidenceGate => 4,
            Self::Release => 5,
        }
    }
}

/// Consent lifecycle. A revoked credential is never visually or logically
/// equivalent to a valid one; conversion fails closed in both inactive
/// states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentState {
    /// No consent has been granted, or the previous grant expired.
    Inactive,
    /// Consent is active for the conversion scope until `until` (exclusive).
    Active { since: u32, until: u32 },
    /// Consent was revoked at `at`; revocation is immediate and explicit.
    Revoked { at: u32 },
}

impl ConsentState {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Inactive => 0,
            Self::Active { .. } => 1,
            Self::Revoked { .. } => 2,
        }
    }
}

/// Evidence levels derived from accumulated points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceLevel {
    L0,
    L1,
    L2,
    L3,
}

impl EvidenceLevel {
    pub const fn from_points(points: u8) -> Self {
        match points {
            0..=1 => Self::L0,
            2..=3 => Self::L1,
            4..=6 => Self::L2,
            _ => Self::L3,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::L0 => "L0",
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::L3 => "L3",
        }
    }
}

/// Review-gate index names. The gate mask is bit `1 << index`.
pub const fn gate_name(index: u8) -> &'static str {
    match index {
        0 => "SCHEMA",
        1 => "CONSENT",
        2 => "EVIDENCE",
        3 => "CONTAIN",
        _ => "DELIVERY",
    }
}

/// Number of review gates.
pub const REVIEW_GATES: u8 = 5;

// ---------------------------------------------------------------------------
// Terminal model and grading
// ---------------------------------------------------------------------------

/// Why a run ended. Stable schema strings are part of the replay contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalReason {
    Sealed,
    HorizonReached,
    IntegrityCollapse,
    RiskOverflow,
    RawLeakThreshold,
    StimulationCrossed,
}

impl TerminalReason {
    pub const fn schema_str(self) -> &'static str {
        match self {
            Self::Sealed => "sealed",
            Self::HorizonReached => "horizon_reached",
            Self::IntegrityCollapse => "integrity_collapse",
            Self::RiskOverflow => "risk_overflow",
            Self::RawLeakThreshold => "raw_leak_threshold",
            Self::StimulationCrossed => "stimulation_crossed",
        }
    }

    pub fn from_schema_str(value: &str) -> Option<Self> {
        Some(match value {
            "sealed" => Self::Sealed,
            "horizon_reached" => Self::HorizonReached,
            "integrity_collapse" => Self::IntegrityCollapse,
            "risk_overflow" => Self::RiskOverflow,
            "raw_leak_threshold" => Self::RawLeakThreshold,
            "stimulation_crossed" => Self::StimulationCrossed,
            _ => return None,
        })
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Sealed => 1,
            Self::HorizonReached => 2,
            Self::IntegrityCollapse => 3,
            Self::RiskOverflow => 4,
            Self::RawLeakThreshold => 5,
            Self::StimulationCrossed => 6,
        }
    }

    pub const fn is_breach(self) -> bool {
        matches!(
            self,
            Self::IntegrityCollapse
                | Self::RiskOverflow
                | Self::RawLeakThreshold
                | Self::StimulationCrossed
        )
    }
}

/// Run status. A terminal state never transitions back to running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Running,
    Terminal(TerminalReason),
}

impl Status {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Terminal(TerminalReason::Sealed) => "sealed",
            Self::Terminal(TerminalReason::HorizonReached) => "expired",
            Self::Terminal(_) => "breached",
        }
    }

    /// Boundary banner string for HUD and verifier output.
    pub const fn boundary(self) -> &'static str {
        match self {
            Self::Running => "HOLDING",
            Self::Terminal(TerminalReason::Sealed) => "SEALED",
            Self::Terminal(TerminalReason::HorizonReached) => "HOLDING",
            Self::Terminal(_) => "BREACHED",
        }
    }
}

/// Boundary grade awarded on the result screen. The formula is normative in
/// `docs/GAME_SPEC.md` §grades and covered by tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Sovereign,
    Sealed,
    Reviewable,
    Degraded,
    Breached,
    Unsafe,
}

impl Grade {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sovereign => "Sovereign",
            Self::Sealed => "Sealed",
            Self::Reviewable => "Reviewable",
            Self::Degraded => "Degraded",
            Self::Breached => "Breached",
            Self::Unsafe => "Unsafe",
        }
    }
}

/// Control buckets used for the "weakest control" result statistic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlBucket {
    Validate,
    Convert,
    Quarantine,
    Consent,
    Evidence,
    Release,
}

impl ControlBucket {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Validate => "VALIDATE",
            Self::Convert => "CONVERT",
            Self::Quarantine => "QUARANTINE",
            Self::Consent => "CONSENT",
            Self::Evidence => "EVIDENCE",
            Self::Release => "RELEASE",
        }
    }

    pub const ALL: [Self; 6] = [
        Self::Validate,
        Self::Convert,
        Self::Quarantine,
        Self::Consent,
        Self::Evidence,
        Self::Release,
    ];
}

// ---------------------------------------------------------------------------
// Entities, input, events
// ---------------------------------------------------------------------------

/// A live entity on the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entity {
    pub id: u16,
    pub kind: EntityKind,
    pub lane: u8,
    pub x: i32,
    pub speed: i32,
    /// For `UnknownPacket`: the concealed kind revealed by Validate.
    pub concealed: Option<EntityKind>,
}

/// Player input applied at one tick. `IDLE` applies nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub select_lane: Option<u8>,
    pub action: Option<Action>,
}

impl Input {
    pub const IDLE: Self = Self {
        select_lane: None,
        action: None,
    };
}

/// Reasons a release attempt is rejected. A blocked release is not a loss;
/// it reports the first failing invariant and play continues.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseBlocker {
    TrustBelowMinimum,
    RiskAboveMaximum,
    IntegrityBelowMinimum,
    EvidenceBelowMinimum,
    GatesIncomplete,
    RawLeaksPresent,
    ConsentInvalid,
    PhaseNotFinal,
}

impl ReleaseBlocker {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TrustBelowMinimum => "TRUST BELOW MINIMUM",
            Self::RiskAboveMaximum => "RISK ABOVE MAXIMUM",
            Self::IntegrityBelowMinimum => "INTEGRITY BELOW MINIMUM",
            Self::EvidenceBelowMinimum => "EVIDENCE BELOW MINIMUM",
            Self::GatesIncomplete => "REVIEW GATES INCOMPLETE",
            Self::RawLeaksPresent => "RAW LEAKS PRESENT",
            Self::ConsentInvalid => "CONSENT INVALID",
            Self::PhaseNotFinal => "FINAL REVIEW PHASE NOT REACHED",
        }
    }
}

/// Per-tick events. At most [`MAX_EVENTS`] are retained per tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Validated(u8),
    Revealed(EntityKind),
    Converted,
    ConvertBlockedConsent(ConsentState),
    ConvertBlockedEvidence,
    Quarantined(EntityKind),
    QuarantineBlockedUnknown,
    FalsePositive(EntityKind),
    ConsentOn { until: u32 },
    ConsentExpired,
    ConsentRevoked { at: u32 },
    ConsentRevokedToken,
    EvidenceUp(EvidenceLevel, u8),
    GatePassed(u8),
    Delivered,
    MissedIntent(EntityKind),
    LostArtifact(EntityKind),
    MinorBreach(EntityKind),
    RawLeakBreach,
    StimBreach,
    WrongTarget(Action, EntityKind),
    NoTarget(Action),
    ReleaseRejected(ReleaseBlocker),
    ReleaseSealed,
    PhaseChanged(u8),
    GuidedStep(u8),
    Terminal(TerminalReason),
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Immutable run configuration. For `RunMode::Daily` the seed must equal
/// [`daily_seed`] of the run date; the verifier enforces this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulationConfig {
    pub seed: u64,
    pub mode: RunMode,
    pub difficulty: Difficulty,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            seed: 0x3001,
            mode: RunMode::Standard,
            difficulty: Difficulty::Standard,
        }
    }
}

// ---------------------------------------------------------------------------
// Scripted spawns (warm-up and Guided Run)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct ScriptEntry {
    tick: u32,
    kind: EntityKind,
    lane: u8,
    /// Guided hint emitted just before the spawn, when present.
    hint: Option<u8>,
}

const fn entry(tick: u32, kind: EntityKind, lane: u8) -> ScriptEntry {
    ScriptEntry {
        tick,
        kind,
        lane,
        hint: None,
    }
}

const fn hinted(tick: u32, kind: EntityKind, lane: u8, hint: u8) -> ScriptEntry {
    ScriptEntry {
        tick,
        kind,
        lane,
        hint: Some(hint),
    }
}

/// Standard/Audit/Grand/Daily warm-up wave: consent before intent, evidence
/// before conversion, one concealed raw frame to teach classification.
const WARMUP: [ScriptEntry; 6] = [
    entry(40, EntityKind::ConsentToken, 2),
    entry(86, EntityKind::IntentCandidate, 2),
    entry(132, EntityKind::Evidence, 1),
    entry(178, EntityKind::IntentCandidate, 3),
    entry(224, EntityKind::Checksum, 1),
    entry(270, EntityKind::UnknownPacket, 0),
];

/// Guided Run script: a 45–60 second teaching sequence. Hint IDs map to
/// coach copy in the web layer; the core stays presentation-free.
const GUIDED_SCRIPT: [ScriptEntry; 13] = [
    hinted(60, EntityKind::RawFrame, 2, 0),
    hinted(360, EntityKind::IntentCandidate, 1, 1),
    hinted(660, EntityKind::Evidence, 3, 2),
    hinted(940, EntityKind::ConsentToken, 2, 3),
    hinted(1_220, EntityKind::IntentCandidate, 3, 4),
    entry(1_500, EntityKind::Checksum, 1),
    entry(1_760, EntityKind::CiTest, 4),
    hinted(2_020, EntityKind::UnsupportedClaim, 0, 5),
    entry(2_280, EntityKind::RawFrame, 4),
    entry(2_520, EntityKind::IntentCandidate, 2),
    entry(2_780, EntityKind::Evidence, 0),
    entry(3_020, EntityKind::ConsentToken, 3),
    hinted(3_200, EntityKind::Checksum, 1, 6),
];

// ---------------------------------------------------------------------------
// Spawn tables (per-mille weights)
// ---------------------------------------------------------------------------

type WeightRow = (EntityKind, u32);

const STANDARD_TABLE: [WeightRow; 13] = [
    (EntityKind::IntentCandidate, 230),
    (EntityKind::ConsentToken, 90),
    (EntityKind::RevokedConsent, 25),
    (EntityKind::Evidence, 80),
    (EntityKind::Checksum, 60),
    (EntityKind::CiTest, 60),
    (EntityKind::RawFrame, 100),
    (EntityKind::Artifact, 80),
    (EntityKind::StimulationCommand, 35),
    (EntityKind::UnsupportedClaim, 80),
    (EntityKind::UntraceableClaim, 50),
    (EntityKind::RoadmapAsFactClaim, 50),
    (EntityKind::UnknownPacket, 60),
];

const AUDIT_TABLE: [WeightRow; 13] = [
    (EntityKind::IntentCandidate, 170),
    (EntityKind::ConsentToken, 70),
    (EntityKind::RevokedConsent, 70),
    (EntityKind::Evidence, 60),
    (EntityKind::Checksum, 50),
    (EntityKind::CiTest, 50),
    (EntityKind::RawFrame, 100),
    (EntityKind::Artifact, 80),
    (EntityKind::StimulationCommand, 45),
    (EntityKind::UnsupportedClaim, 95),
    (EntityKind::UntraceableClaim, 60),
    (EntityKind::RoadmapAsFactClaim, 60),
    (EntityKind::UnknownPacket, 90),
];

const GRAND_P1_TABLE: [WeightRow; 10] = [
    (EntityKind::RawFrame, 220),
    (EntityKind::Artifact, 170),
    (EntityKind::StimulationCommand, 60),
    (EntityKind::IntentCandidate, 200),
    (EntityKind::UnknownPacket, 110),
    (EntityKind::Evidence, 60),
    (EntityKind::Checksum, 50),
    (EntityKind::CiTest, 40),
    (EntityKind::ConsentToken, 50),
    (EntityKind::UnsupportedClaim, 40),
];

const GRAND_P2_TABLE: [WeightRow; 10] = [
    (EntityKind::ConsentToken, 160),
    (EntityKind::RevokedConsent, 120),
    (EntityKind::Evidence, 140),
    (EntityKind::Checksum, 90),
    (EntityKind::CiTest, 90),
    (EntityKind::IntentCandidate, 190),
    (EntityKind::RawFrame, 60),
    (EntityKind::Artifact, 50),
    (EntityKind::UnknownPacket, 60),
    (EntityKind::UnsupportedClaim, 40),
];

const GRAND_P3_TABLE: [WeightRow; 11] = [
    (EntityKind::UnsupportedClaim, 180),
    (EntityKind::UntraceableClaim, 130),
    (EntityKind::RoadmapAsFactClaim, 120),
    (EntityKind::IntentCandidate, 180),
    (EntityKind::RawFrame, 80),
    (EntityKind::Artifact, 60),
    (EntityKind::StimulationCommand, 40),
    (EntityKind::UnknownPacket, 80),
    (EntityKind::ConsentToken, 60),
    (EntityKind::Evidence, 40),
    (EntityKind::Checksum, 30),
];

const GRAND_FINAL_TABLE: [WeightRow; 11] = [
    (EntityKind::IntentCandidate, 240),
    (EntityKind::ConsentToken, 120),
    (EntityKind::Evidence, 120),
    (EntityKind::Checksum, 80),
    (EntityKind::CiTest, 80),
    (EntityKind::RawFrame, 70),
    (EntityKind::RevokedConsent, 60),
    (EntityKind::UnsupportedClaim, 80),
    (EntityKind::UnknownPacket, 70),
    (EntityKind::StimulationCommand, 30),
    (EntityKind::Artifact, 50),
];

/// Kinds an `UnknownPacket` may conceal, with weights.
const CONCEALED_TABLE: [WeightRow; 6] = [
    (EntityKind::RawFrame, 300),
    (EntityKind::IntentCandidate, 250),
    (EntityKind::UnsupportedClaim, 180),
    (EntityKind::Artifact, 120),
    (EntityKind::Evidence, 100),
    (EntityKind::StimulationCommand, 50),
];

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

/// Read-only view of authoritative state for rendering and reporting.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub tick: u32,
    pub mode: RunMode,
    pub difficulty: Difficulty,
    pub status: Status,
    pub selected_lane: u8,
    pub trust: i32,
    pub risk: i32,
    pub integrity: i32,
    pub evidence_points: u8,
    pub evidence_level: EvidenceLevel,
    pub consent: ConsentState,
    pub consent_remaining: u32,
    pub gates_mask: u8,
    pub gates_passed: u8,
    pub raw_leaks: u8,
    pub delivered: u8,
    pub score: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub phase: u8,
    pub cooldown: u32,
    pub horizon: u32,
    pub false_positives: u8,
    pub release_rejections: u8,
    /// Missed-handling counters per control bucket, in `ControlBucket::ALL`
    /// order, used for the weakest-control statistic.
    pub control_misses: [u8; 6],
    /// Live entities in the fixed pool (pool pressure indicator).
    pub live_entities: u8,
}

// ---------------------------------------------------------------------------
// Simulation
// ---------------------------------------------------------------------------

/// The authoritative deterministic simulation.
#[derive(Clone, Debug)]
pub struct Simulation {
    config: SimulationConfig,
    rules: ModeRules,
    cadence: Cadence,
    rng: Rng64,
    tick: u32,
    status: Status,
    selected_lane: u8,
    trust: i32,
    risk: i32,
    integrity: i32,
    evidence_points: u8,
    consent: ConsentState,
    gates: u8,
    raw_leaks: u8,
    score: u32,
    streak: u32,
    best_streak: u32,
    phase: u8,
    validations: u8,
    quarantines: u8,
    delivered: u8,
    reveals: u8,
    false_positives: u8,
    minor_breaches: u8,
    release_rejections: u8,
    control_misses: [u8; 6],
    cooldown: u32,
    script_index: u8,
    spawn_timer: u32,
    spawned: u16,
    next_id: u16,
    entities: [Option<Entity>; MAX_ENTITIES],
    events: [Option<Event>; MAX_EVENTS],
    event_count: u8,
}

impl Simulation {
    pub fn new(config: SimulationConfig) -> Self {
        let cadence = config.difficulty.cadence();
        let mut simulation = Self {
            config,
            rules: config.mode.rules(),
            cadence,
            rng: Rng64::new(config.seed),
            tick: 0,
            status: Status::Running,
            selected_lane: 2,
            trust: 50,
            risk: 0,
            integrity: 100,
            evidence_points: 0,
            consent: ConsentState::Inactive,
            gates: 0,
            raw_leaks: 0,
            score: 0,
            streak: 0,
            best_streak: 0,
            phase: 0,
            validations: 0,
            quarantines: 0,
            delivered: 0,
            reveals: 0,
            false_positives: 0,
            minor_breaches: 0,
            release_rejections: 0,
            control_misses: [0; 6],
            cooldown: 0,
            script_index: 0,
            spawn_timer: 0,
            spawned: 0,
            next_id: 1,
            entities: [None; MAX_ENTITIES],
            events: [None; MAX_EVENTS],
            event_count: 0,
        };
        simulation.spawn_timer = simulation.first_random_delay();
        simulation
    }

    fn first_random_delay(&self) -> u32 {
        match self.config.mode {
            RunMode::Guided => GUIDED_SCRIPT[GUIDED_SCRIPT.len() - 1].tick + 200,
            _ => WARMUP[WARMUP.len() - 1].tick + self.cadence.base_interval,
        }
    }

    // -- accessors ----------------------------------------------------------

    pub const fn config(&self) -> SimulationConfig {
        self.config
    }

    pub const fn rules(&self) -> ModeRules {
        self.rules
    }

    pub const fn status(&self) -> Status {
        self.status
    }

    pub const fn tick(&self) -> u32 {
        self.tick
    }

    pub fn entities(&self) -> &[Option<Entity>; MAX_ENTITIES] {
        &self.entities
    }

    /// Events emitted during the most recent `step`.
    pub fn events(&self) -> impl Iterator<Item = Event> + '_ {
        self.events
            .iter()
            .take(self.event_count as usize)
            .flatten()
            .copied()
    }

    // -- stepping -----------------------------------------------------------

    /// Advance one tick. After a terminal state this is a no-op; terminal
    /// states never transition back to running.
    pub fn step(&mut self, input: Input) {
        self.events = [None; MAX_EVENTS];
        self.event_count = 0;

        if self.status != Status::Running {
            return;
        }

        self.tick += 1;

        if let Some(lane) = input.select_lane {
            if lane < LANES {
                self.selected_lane = lane;
            }
        }

        self.update_phase();
        self.update_consent();

        if self.cooldown > 0 {
            self.cooldown -= 1;
        }

        if let Some(action) = input.action {
            if self.cooldown == 0 {
                self.apply_action(action);
            }
        }

        if self.status != Status::Running {
            self.push(Event::Terminal(self.terminal_reason_unchecked()));
            return;
        }

        self.advance_entities();

        if self.status == Status::Running {
            self.spawn_phase();
        }

        if self.status == Status::Running && self.tick >= self.rules.horizon {
            self.status = Status::Terminal(TerminalReason::HorizonReached);
        }

        if self.status != Status::Running {
            self.push(Event::Terminal(self.terminal_reason_unchecked()));
        }
    }

    fn terminal_reason_unchecked(&self) -> TerminalReason {
        match self.status {
            Status::Terminal(reason) => reason,
            Status::Running => TerminalReason::HorizonReached,
        }
    }

    fn update_phase(&mut self) {
        if self.config.mode != RunMode::Grand {
            return;
        }
        let mut phase = 0u8;
        for (index, start) in GRAND_PHASE_STARTS.iter().enumerate() {
            if self.tick >= *start {
                phase = index as u8;
            }
        }
        if phase != self.phase {
            self.phase = phase;
            self.push(Event::PhaseChanged(phase));
        }
    }

    fn update_consent(&mut self) {
        if let ConsentState::Active { until, .. } = self.consent {
            if self.tick >= until {
                self.consent = ConsentState::Inactive;
                self.push(Event::ConsentExpired);
            }
        }
    }

    // -- player actions -----------------------------------------------------

    /// Frontmost entity in the gate window of the selected lane.
    fn target_index(&self) -> Option<usize> {
        let mut best: Option<(usize, i32)> = None;
        for (index, slot) in self.entities.iter().enumerate() {
            if let Some(entity) = slot {
                if entity.lane != self.selected_lane {
                    continue;
                }
                if entity.x < BOUNDARY_X - GATE_WINDOW || entity.x >= BOUNDARY_X {
                    continue;
                }
                if best.map(|(_, x)| entity.x > x).unwrap_or(true) {
                    best = Some((index, entity.x));
                }
            }
        }
        best.map(|(index, _)| index)
    }

    fn apply_action(&mut self, action: Action) {
        if action == Action::Release {
            self.attempt_release();
            return;
        }

        let Some(index) = self.target_index() else {
            self.push(Event::NoTarget(action));
            self.cooldown = WHIFF_COOLDOWN;
            return;
        };
        let entity = self.entities[index].expect("target exists");

        match (action, entity.kind) {
            (Action::Validate, EntityKind::IntentCandidate) => {
                if let Some(slot) = self.entities[index].as_mut() {
                    slot.kind = EntityKind::ValidatedIntent;
                }
                self.validations = self.validations.saturating_add(1);
                self.reward(30);
                self.push(Event::Validated(self.validations));
                if self.validations >= self.rules.gate_validations {
                    self.pass_gate(0);
                }
                self.cooldown = ACTION_COOLDOWN;
            }
            (Action::Validate, EntityKind::UnknownPacket) => {
                let revealed = entity.concealed.unwrap_or(EntityKind::Artifact);
                if let Some(slot) = self.entities[index].as_mut() {
                    slot.kind = revealed;
                    slot.concealed = None;
                }
                self.reveals = self.reveals.saturating_add(1);
                self.risk = saturate(self.risk + 4);
                self.reward(25);
                self.push(Event::Revealed(revealed));
                self.check_risk();
                self.cooldown = ACTION_COOLDOWN;
            }
            (Action::Convert, EntityKind::ValidatedIntent) => {
                if !self.consent.is_active() {
                    self.push(Event::ConvertBlockedConsent(self.consent));
                    self.cooldown = WHIFF_COOLDOWN;
                    return;
                }
                if EvidenceLevel::from_points(self.evidence_points) < EvidenceLevel::L1 {
                    self.push(Event::ConvertBlockedEvidence);
                    self.cooldown = WHIFF_COOLDOWN;
                    return;
                }
                if let Some(slot) = self.entities[index].as_mut() {
                    slot.kind = EntityKind::TypedIntent;
                    slot.speed += 1;
                }
                self.reward(45);
                self.push(Event::Converted);
                self.cooldown = ACTION_COOLDOWN;
            }
            (Action::Quarantine, EntityKind::UnknownPacket) => {
                self.push(Event::QuarantineBlockedUnknown);
                self.cooldown = WHIFF_COOLDOWN;
            }
            (Action::Quarantine, kind) if kind.is_quarantine_target() => {
                self.entities[index] = None;
                self.quarantines = self.quarantines.saturating_add(1);
                self.reward(if kind.is_claim() { 45 } else { 60 });
                self.push(Event::Quarantined(kind));
                if self.quarantines >= self.rules.gate_quarantines {
                    self.pass_gate(3);
                }
                self.cooldown = ACTION_COOLDOWN;
            }
            (Action::Quarantine, kind) => {
                self.entities[index] = None;
                self.false_positives = self.false_positives.saturating_add(1);
                self.trust = saturate(self.trust - 3);
                self.fail_streak();
                self.penalize(40);
                self.push(Event::FalsePositive(kind));
                self.cooldown = ACTION_COOLDOWN;
            }
            (Action::ConsentGate, EntityKind::ConsentToken) => {
                self.entities[index] = None;
                let until = self.tick + CONSENT_DURATION;
                self.consent = ConsentState::Active {
                    since: self.tick,
                    until,
                };
                self.reward(40);
                self.push(Event::ConsentOn { until });
                self.pass_gate(1);
                self.cooldown = ACTION_COOLDOWN;
            }
            (Action::ConsentGate, EntityKind::RevokedConsent) => {
                self.push(Event::ConsentRevokedToken);
                self.cooldown = WHIFF_COOLDOWN;
            }
            (Action::EvidenceGate, kind) if kind.evidence_points() > 0 => {
                self.entities[index] = None;
                let before = EvidenceLevel::from_points(self.evidence_points);
                self.evidence_points = self
                    .evidence_points
                    .saturating_add(kind.evidence_points())
                    .min(EVIDENCE_POINTS_MAX);
                let after = EvidenceLevel::from_points(self.evidence_points);
                self.reward(35);
                self.push(Event::EvidenceUp(after, self.evidence_points));
                if after > before || after >= EvidenceLevel::L1 {
                    // level event already carries the detail
                }
                if self.evidence_points >= self.rules.evidence_points_min {
                    self.pass_gate(2);
                }
                self.cooldown = ACTION_COOLDOWN;
            }
            (action, kind) => {
                self.push(Event::WrongTarget(action, kind));
                self.cooldown = WHIFF_COOLDOWN;
            }
        }
    }

    /// First failing release invariant, if any. Fail-closed: any invariant
    /// violation blocks sealing.
    pub fn release_blocker(&self) -> Option<ReleaseBlocker> {
        if self.rules.release_final_phase_only && self.phase < 3 {
            return Some(ReleaseBlocker::PhaseNotFinal);
        }
        if self.raw_leaks > 0 {
            return Some(ReleaseBlocker::RawLeaksPresent);
        }
        if matches!(self.consent, ConsentState::Revoked { .. }) {
            return Some(ReleaseBlocker::ConsentInvalid);
        }
        if self.gates != (1 << REVIEW_GATES) - 1 {
            return Some(ReleaseBlocker::GatesIncomplete);
        }
        if self.evidence_points < self.rules.evidence_points_min {
            return Some(ReleaseBlocker::EvidenceBelowMinimum);
        }
        if self.trust < self.rules.trust_min {
            return Some(ReleaseBlocker::TrustBelowMinimum);
        }
        if self.risk > self.rules.risk_max {
            return Some(ReleaseBlocker::RiskAboveMaximum);
        }
        if self.integrity < self.rules.integrity_min {
            return Some(ReleaseBlocker::IntegrityBelowMinimum);
        }
        None
    }

    /// All currently failing release invariants, for blocked-release UI.
    pub fn release_blockers(&self) -> ([Option<ReleaseBlocker>; 8], u8) {
        let mut out = [None; 8];
        let mut count = 0u8;
        let mut add = |blocker: ReleaseBlocker| {
            if (count as usize) < out.len() {
                out[count as usize] = Some(blocker);
                count += 1;
            }
        };
        if self.rules.release_final_phase_only && self.phase < 3 {
            add(ReleaseBlocker::PhaseNotFinal);
        }
        if self.raw_leaks > 0 {
            add(ReleaseBlocker::RawLeaksPresent);
        }
        if matches!(self.consent, ConsentState::Revoked { .. }) {
            add(ReleaseBlocker::ConsentInvalid);
        }
        if self.gates != (1 << REVIEW_GATES) - 1 {
            add(ReleaseBlocker::GatesIncomplete);
        }
        if self.evidence_points < self.rules.evidence_points_min {
            add(ReleaseBlocker::EvidenceBelowMinimum);
        }
        if self.trust < self.rules.trust_min {
            add(ReleaseBlocker::TrustBelowMinimum);
        }
        if self.risk > self.rules.risk_max {
            add(ReleaseBlocker::RiskAboveMaximum);
        }
        if self.integrity < self.rules.integrity_min {
            add(ReleaseBlocker::IntegrityBelowMinimum);
        }
        (out, count)
    }

    fn attempt_release(&mut self) {
        match self.release_blocker() {
            None => {
                self.reward(200);
                self.push(Event::ReleaseSealed);
                self.status = Status::Terminal(TerminalReason::Sealed);
            }
            Some(blocker) => {
                self.risk = saturate(self.risk + 6);
                self.release_rejections = self.release_rejections.saturating_add(1);
                self.bump_miss(ControlBucket::Release);
                self.fail_streak();
                self.push(Event::ReleaseRejected(blocker));
                self.check_risk();
                self.cooldown = ACTION_COOLDOWN;
            }
        }
    }

    // -- scoring ------------------------------------------------------------

    fn reward(&mut self, base: u32) {
        self.streak = self.streak.saturating_add(1);
        if self.streak > self.best_streak {
            self.best_streak = self.streak;
        }
        let bonus = base * self.streak.min(STREAK_BONUS_CAP) / STREAK_BONUS_CAP;
        self.score = self.score.saturating_add(base + bonus);
    }

    fn penalize(&mut self, amount: u32) {
        self.score = self.score.saturating_sub(amount);
    }

    fn fail_streak(&mut self) {
        self.streak = 0;
    }

    fn bump_miss(&mut self, bucket: ControlBucket) {
        let index = bucket as usize;
        self.control_misses[index] = self.control_misses[index].saturating_add(1);
    }

    fn pass_gate(&mut self, index: u8) {
        let bit = 1u8 << index;
        if self.gates & bit == 0 {
            self.gates |= bit;
            self.push(Event::GatePassed(index));
        }
    }

    fn check_risk(&mut self) {
        if self.risk >= 100 {
            self.status = Status::Terminal(TerminalReason::RiskOverflow);
        }
    }

    // -- world advance ------------------------------------------------------

    fn advance_entities(&mut self) {
        for index in 0..MAX_ENTITIES {
            let Some(mut entity) = self.entities[index] else {
                continue;
            };
            entity.x += entity.speed;

            if entity.kind == EntityKind::TypedIntent {
                if entity.x >= FIELD_END_X {
                    self.entities[index] = None;
                    self.delivered = self.delivered.saturating_add(1);
                    self.trust = saturate(self.trust + 6);
                    self.reward(120);
                    self.push(Event::Delivered);
                    if self.delivered >= self.rules.gate_deliveries {
                        self.pass_gate(4);
                    }
                } else {
                    self.entities[index] = Some(entity);
                }
                continue;
            }

            if entity.x >= BOUNDARY_X {
                self.entities[index] = None;
                self.resolve_crossing(entity);
                if self.status != Status::Running {
                    return;
                }
            } else {
                self.entities[index] = Some(entity);
            }
        }
    }

    fn resolve_crossing(&mut self, entity: Entity) {
        let kind = if entity.kind == EntityKind::UnknownPacket {
            entity.concealed.unwrap_or(EntityKind::Artifact)
        } else {
            entity.kind
        };

        match kind {
            EntityKind::RawFrame => {
                self.raw_leaks = self.raw_leaks.saturating_add(1);
                self.integrity = saturate(self.integrity - 10);
                self.bump_miss(ControlBucket::Quarantine);
                self.fail_streak();
                self.penalize(90);
                self.push(Event::RawLeakBreach);
                if self.raw_leaks >= RAW_LEAK_LIMIT {
                    self.status = Status::Terminal(TerminalReason::RawLeakThreshold);
                    return;
                }
                self.check_integrity();
            }
            EntityKind::StimulationCommand => {
                self.fail_streak();
                self.push(Event::StimBreach);
                self.status = Status::Terminal(TerminalReason::StimulationCrossed);
            }
            EntityKind::Artifact => {
                self.minor_breach(EntityKind::Artifact, ControlBucket::Quarantine);
            }
            EntityKind::UnsupportedClaim
            | EntityKind::UntraceableClaim
            | EntityKind::RoadmapAsFactClaim => {
                self.minor_breach(kind, ControlBucket::Quarantine);
            }
            EntityKind::RevokedConsent => {
                self.consent = ConsentState::Revoked { at: self.tick };
                self.risk = saturate(self.risk + 6);
                self.bump_miss(ControlBucket::Consent);
                self.fail_streak();
                self.penalize(60);
                self.push(Event::ConsentRevoked { at: self.tick });
                self.check_risk();
            }
            EntityKind::IntentCandidate => {
                self.trust = saturate(self.trust - 1);
                self.bump_miss(ControlBucket::Validate);
                self.fail_streak();
                self.push(Event::MissedIntent(EntityKind::IntentCandidate));
            }
            EntityKind::ValidatedIntent => {
                self.trust = saturate(self.trust - 1);
                self.bump_miss(ControlBucket::Convert);
                self.fail_streak();
                self.push(Event::MissedIntent(EntityKind::ValidatedIntent));
            }
            EntityKind::ConsentToken => {
                self.bump_miss(ControlBucket::Consent);
                self.push(Event::LostArtifact(kind));
            }
            EntityKind::Evidence | EntityKind::Checksum | EntityKind::CiTest => {
                self.bump_miss(ControlBucket::Evidence);
                self.push(Event::LostArtifact(kind));
            }
            EntityKind::TypedIntent | EntityKind::UnknownPacket => {}
        }
    }

    fn minor_breach(&mut self, kind: EntityKind, bucket: ControlBucket) {
        self.integrity = saturate(self.integrity - 6);
        self.minor_breaches = self.minor_breaches.saturating_add(1);
        self.bump_miss(bucket);
        self.fail_streak();
        self.penalize(60);
        self.push(Event::MinorBreach(kind));
        self.check_integrity();
    }

    fn check_integrity(&mut self) {
        if self.integrity <= 0 {
            self.status = Status::Terminal(TerminalReason::IntegrityCollapse);
        }
    }

    // -- spawning -----------------------------------------------------------

    fn spawn_phase(&mut self) {
        let script: &[ScriptEntry] = match self.config.mode {
            RunMode::Guided => &GUIDED_SCRIPT,
            _ => &WARMUP,
        };
        while (self.script_index as usize) < script.len()
            && script[self.script_index as usize].tick <= self.tick
        {
            let entry = script[self.script_index as usize];
            if let Some(hint) = entry.hint {
                self.push(Event::GuidedStep(hint));
            }
            self.insert_entity(entry.kind, entry.lane, self.script_speed(entry.kind));
            self.script_index += 1;
        }
        if (self.script_index as usize) < script.len() {
            return;
        }

        if self.spawn_timer > 0 {
            self.spawn_timer -= 1;
            return;
        }

        let kind = self.roll_kind();
        let lane = self.rng.range(LANES as u32) as u8;
        let mut speed =
            self.cadence.base_speed + self.rng.range(self.cadence.speed_jitter + 1) as i32;
        if kind.is_claim() {
            speed += CLAIM_SPEED_BONUS;
        }
        if self.config.mode == RunMode::Guided {
            speed = speed.min(2);
        }
        self.insert_entity(kind, lane, speed);
        self.spawn_timer = self.next_interval();
    }

    fn script_speed(&self, kind: EntityKind) -> i32 {
        let base = if self.config.mode == RunMode::Guided {
            1
        } else {
            self.cadence.base_speed
        };
        if kind.is_claim() {
            base + 1
        } else {
            base
        }
    }

    fn next_interval(&mut self) -> u32 {
        let shrink = (self.tick / self.cadence.interval_shrink_every).min(20);
        let base = self
            .cadence
            .base_interval
            .saturating_sub(shrink)
            .max(self.cadence.min_interval);
        let jitter = self.rng.range(self.cadence.interval_jitter * 2 + 1);
        let interval = base + jitter - self.cadence.interval_jitter.min(base + jitter);
        let guided_floor = if self.config.mode == RunMode::Guided {
            120
        } else {
            0
        };
        interval.max(self.cadence.min_interval).max(guided_floor)
    }

    fn active_table(&self) -> &'static [WeightRow] {
        match self.config.mode {
            RunMode::Audit => &AUDIT_TABLE,
            RunMode::Grand => match self.phase {
                0 => &GRAND_P1_TABLE,
                1 => &GRAND_P2_TABLE,
                2 => &GRAND_P3_TABLE,
                _ => &GRAND_FINAL_TABLE,
            },
            _ => &STANDARD_TABLE,
        }
    }

    fn roll_from(&mut self, table: &[WeightRow]) -> EntityKind {
        let total: u32 = table.iter().map(|(_, weight)| *weight).sum();
        let mut roll = self.rng.range(total);
        for (kind, weight) in table {
            if roll < *weight {
                return *kind;
            }
            roll -= *weight;
        }
        EntityKind::IntentCandidate
    }

    fn roll_kind(&mut self) -> EntityKind {
        let table = self.active_table();
        self.roll_from(table)
    }

    fn insert_entity(&mut self, kind: EntityKind, lane: u8, speed: i32) {
        let concealed = if kind == EntityKind::UnknownPacket {
            Some(self.roll_from(&CONCEALED_TABLE))
        } else {
            None
        };
        let entity = Entity {
            id: self.next_id,
            kind,
            lane: lane.min(LANES - 1),
            x: SPAWN_X,
            speed: speed.max(1),
            concealed,
        };
        for slot in self.entities.iter_mut() {
            if slot.is_none() {
                *slot = Some(entity);
                self.next_id = self.next_id.wrapping_add(1).max(1);
                self.spawned = self.spawned.saturating_add(1);
                return;
            }
        }
        // Pool saturated: the spawn is suppressed. This is the documented
        // graceful-degradation policy (docs/GAME_SPEC.md §pool): existing
        // boundary decisions are never evicted, authoritative invariants do
        // not depend on pool pressure, and the run never terminates because
        // of engine resource limits.
    }

    // -- events -------------------------------------------------------------

    fn push(&mut self, event: Event) {
        if (self.event_count as usize) < MAX_EVENTS {
            self.events[self.event_count as usize] = Some(event);
            self.event_count += 1;
        }
    }

    // -- reporting ----------------------------------------------------------

    pub fn snapshot(&self) -> Snapshot {
        let consent_remaining = match self.consent {
            ConsentState::Active { until, .. } => until.saturating_sub(self.tick),
            _ => 0,
        };
        Snapshot {
            tick: self.tick,
            mode: self.config.mode,
            difficulty: self.config.difficulty,
            status: self.status,
            selected_lane: self.selected_lane,
            trust: self.trust,
            risk: self.risk,
            integrity: self.integrity,
            evidence_points: self.evidence_points,
            evidence_level: EvidenceLevel::from_points(self.evidence_points),
            consent: self.consent,
            consent_remaining,
            gates_mask: self.gates,
            gates_passed: self.gates.count_ones() as u8,
            raw_leaks: self.raw_leaks,
            delivered: self.delivered,
            score: self.score,
            streak: self.streak,
            best_streak: self.best_streak,
            phase: self.phase,
            cooldown: self.cooldown,
            horizon: self.rules.horizon,
            false_positives: self.false_positives,
            release_rejections: self.release_rejections,
            control_misses: self.control_misses,
            live_entities: self.entities.iter().flatten().count() as u8,
        }
    }

    /// Boundary grade. Normative formula, tested in this crate:
    /// Sovereign = sealed with integrity ≥ 95, risk ≤ 10 and no false
    /// positives; Sealed = any other seal; Reviewable = horizon end with no
    /// leaks and integrity ≥ 60; Degraded = other horizon ends and metric
    /// collapses; Breached = raw-leak threshold; Unsafe = stimulation
    /// crossing.
    pub fn grade(&self) -> Grade {
        match self.status {
            Status::Running => Grade::Reviewable,
            Status::Terminal(TerminalReason::Sealed) => {
                if self.integrity >= 95 && self.risk <= 10 && self.false_positives == 0 {
                    Grade::Sovereign
                } else {
                    Grade::Sealed
                }
            }
            Status::Terminal(TerminalReason::HorizonReached) => {
                if self.raw_leaks == 0 && self.integrity >= 60 {
                    Grade::Reviewable
                } else {
                    Grade::Degraded
                }
            }
            Status::Terminal(TerminalReason::IntegrityCollapse)
            | Status::Terminal(TerminalReason::RiskOverflow) => Grade::Degraded,
            Status::Terminal(TerminalReason::RawLeakThreshold) => Grade::Breached,
            Status::Terminal(TerminalReason::StimulationCrossed) => Grade::Unsafe,
        }
    }

    /// The control bucket with the most missed-handling incidents, if any.
    pub fn weakest_control(&self) -> Option<ControlBucket> {
        let mut best: Option<(ControlBucket, u8)> = None;
        for (index, bucket) in ControlBucket::ALL.iter().enumerate() {
            let misses = self.control_misses[index];
            if misses == 0 {
                continue;
            }
            if best.map(|(_, current)| misses > current).unwrap_or(true) {
                best = Some((*bucket, misses));
            }
        }
        best.map(|(bucket, _)| bucket)
    }

    /// Deterministic 64-bit state hash. Field order and encoding are
    /// normative (`docs/REPLAY_SPEC.md`, algorithm `fnv1a64-v1`).
    pub fn state_hash(&self) -> u64 {
        let mut hash = Fnv64::new();
        hash.write_u64(self.config.seed);
        hash.write_u8(self.config.mode.code());
        hash.write_u8(self.config.difficulty.code());
        hash.write_u64(self.rng.state());
        hash.write_u32(self.tick);
        hash.write_u8(match self.status {
            Status::Running => 0,
            Status::Terminal(reason) => 0x10 | reason.code(),
        });
        hash.write_u8(self.selected_lane);
        hash.write_i32(self.trust);
        hash.write_i32(self.risk);
        hash.write_i32(self.integrity);
        hash.write_u8(self.evidence_points);
        hash.write_u8(self.consent.code());
        match self.consent {
            ConsentState::Inactive => hash.write_u32(0),
            ConsentState::Active { until, .. } => hash.write_u32(until),
            ConsentState::Revoked { at } => hash.write_u32(at),
        }
        hash.write_u8(self.gates);
        hash.write_u8(self.raw_leaks);
        hash.write_u32(self.score);
        hash.write_u32(self.streak);
        hash.write_u32(self.best_streak);
        hash.write_u8(self.phase);
        hash.write_u8(self.validations);
        hash.write_u8(self.quarantines);
        hash.write_u8(self.delivered);
        hash.write_u8(self.reveals);
        hash.write_u8(self.false_positives);
        hash.write_u8(self.minor_breaches);
        hash.write_u8(self.release_rejections);
        for misses in self.control_misses {
            hash.write_u8(misses);
        }
        hash.write_u32(self.cooldown);
        hash.write_u32(self.spawn_timer);
        hash.write_u16(self.spawned);
        hash.write_u16(self.next_id);
        for slot in &self.entities {
            match slot {
                None => hash.write_u8(0),
                Some(entity) => {
                    hash.write_u8(1);
                    hash.write_u16(entity.id);
                    hash.write_u8(entity.kind.code());
                    hash.write_u8(entity.lane);
                    hash.write_i32(entity.x);
                    hash.write_i32(entity.speed);
                    match entity.concealed {
                        None => hash.write_u8(0xFF),
                        Some(kind) => hash.write_u8(kind.code()),
                    }
                }
            }
        }
        hash.finish()
    }
}

const fn saturate(value: i32) -> i32 {
    if value < 0 {
        0
    } else if value > 100 {
        100
    } else {
        value
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn config(mode: RunMode) -> SimulationConfig {
        SimulationConfig {
            seed: 0x3001,
            mode,
            difficulty: Difficulty::Standard,
        }
    }

    /// A simulation with random spawning pushed beyond reach and no script.
    fn isolate(mode: RunMode) -> Simulation {
        let mut simulation = Simulation::new(config(mode));
        simulation.script_index = match mode {
            RunMode::Guided => GUIDED_SCRIPT.len() as u8,
            _ => WARMUP.len() as u8,
        };
        simulation.spawn_timer = u32::MAX;
        simulation
    }

    fn place(simulation: &mut Simulation, kind: EntityKind, lane: u8, x: i32) {
        simulation.insert_entity(kind, lane, 0);
        for slot in simulation.entities.iter_mut().flatten() {
            if slot.x == SPAWN_X && slot.kind == kind && slot.lane == lane {
                slot.x = x;
                break;
            }
        }
    }

    fn act(simulation: &mut Simulation, lane: u8, action: Action) {
        simulation.cooldown = 0;
        simulation.step(Input {
            select_lane: Some(lane),
            action: Some(action),
        });
    }

    fn grant_consent(simulation: &mut Simulation) {
        place(simulation, EntityKind::ConsentToken, 0, 500);
        act(simulation, 0, Action::ConsentGate);
        assert!(simulation.consent.is_active());
    }

    fn grant_evidence(simulation: &mut Simulation) {
        place(simulation, EntityKind::Evidence, 1, 500);
        act(simulation, 1, Action::EvidenceGate);
        assert!(simulation.evidence_points >= 2);
    }

    #[test]
    fn rng_is_deterministic() {
        let mut a = Rng64::new(7);
        let mut b = Rng64::new(7);
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn daily_seed_is_stable_and_date_sensitive() {
        assert_eq!(daily_seed(2026, 6, 11), daily_seed(2026, 6, 11));
        assert_ne!(daily_seed(2026, 6, 11), daily_seed(2026, 6, 12));
        assert_ne!(daily_seed(2026, 6, 11), 0);
    }

    #[test]
    fn same_seed_same_inputs_same_hash() {
        let run = |seed| {
            let mut simulation = Simulation::new(SimulationConfig {
                seed,
                mode: RunMode::Standard,
                difficulty: Difficulty::Standard,
            });
            for tick in 1..=600u32 {
                let input = if tick % 90 == 0 {
                    Input {
                        select_lane: Some((tick / 90 % 5) as u8),
                        action: Some(Action::Validate),
                    }
                } else {
                    Input::IDLE
                };
                simulation.step(input);
            }
            simulation.state_hash()
        };
        assert_eq!(run(99), run(99));
        assert_ne!(run(99), run(100));
    }

    #[test]
    fn same_seed_different_inputs_diverge() {
        let mut idle = Simulation::new(config(RunMode::Standard));
        let mut active = Simulation::new(config(RunMode::Standard));
        for _ in 1..=400u32 {
            idle.step(Input::IDLE);
            active.step(Input {
                select_lane: Some(2),
                action: Some(Action::Validate),
            });
        }
        assert_ne!(idle.state_hash(), active.state_hash());
    }

    #[test]
    fn raw_frame_cannot_become_output_and_counts_as_leak() {
        let mut simulation = isolate(RunMode::Standard);
        place(&mut simulation, EntityKind::RawFrame, 2, BOUNDARY_X - 1);
        for slot in simulation.entities.iter_mut().flatten() {
            slot.speed = 2;
        }
        simulation.step(Input::IDLE);
        assert_eq!(simulation.raw_leaks, 1);
        assert_eq!(simulation.integrity, 90);
        assert_eq!(simulation.delivered, 0);
    }

    #[test]
    fn raw_leak_threshold_breaches() {
        let mut simulation = isolate(RunMode::Standard);
        for _ in 0..RAW_LEAK_LIMIT {
            place(&mut simulation, EntityKind::RawFrame, 1, BOUNDARY_X - 1);
            for slot in simulation.entities.iter_mut().flatten() {
                slot.speed = 2;
            }
            simulation.step(Input::IDLE);
        }
        assert_eq!(
            simulation.status(),
            Status::Terminal(TerminalReason::RawLeakThreshold)
        );
        assert_eq!(simulation.grade(), Grade::Breached);
    }

    #[test]
    fn stimulation_command_fails_closed() {
        let mut simulation = isolate(RunMode::Standard);
        place(
            &mut simulation,
            EntityKind::StimulationCommand,
            3,
            BOUNDARY_X - 1,
        );
        for slot in simulation.entities.iter_mut().flatten() {
            slot.speed = 2;
        }
        simulation.step(Input::IDLE);
        assert_eq!(
            simulation.status(),
            Status::Terminal(TerminalReason::StimulationCrossed)
        );
        assert_eq!(simulation.grade(), Grade::Unsafe);
    }

    #[test]
    fn typed_intent_requires_validation_consent_and_evidence() {
        let mut simulation = isolate(RunMode::Standard);
        place(&mut simulation, EntityKind::IntentCandidate, 2, 500);
        // Convert before validation: wrong target.
        act(&mut simulation, 2, Action::Convert);
        assert!(simulation
            .events()
            .any(|event| matches!(event, Event::WrongTarget(Action::Convert, _))));
        // Validate.
        act(&mut simulation, 2, Action::Validate);
        // Convert without consent: blocked.
        act(&mut simulation, 2, Action::Convert);
        assert!(simulation
            .events()
            .any(|event| matches!(event, Event::ConvertBlockedConsent(_))));
        grant_consent(&mut simulation);
        // Convert without evidence: blocked.
        act(&mut simulation, 2, Action::Convert);
        assert!(simulation
            .events()
            .any(|event| matches!(event, Event::ConvertBlockedEvidence)));
        grant_evidence(&mut simulation);
        act(&mut simulation, 2, Action::Convert);
        assert!(simulation.events().any(|event| event == Event::Converted));
        assert!(simulation
            .entities()
            .iter()
            .flatten()
            .any(|entity| entity.kind == EntityKind::TypedIntent));
    }

    #[test]
    fn revoked_consent_blocks_conversion_immediately() {
        let mut simulation = isolate(RunMode::Standard);
        grant_consent(&mut simulation);
        grant_evidence(&mut simulation);
        place(&mut simulation, EntityKind::IntentCandidate, 2, 460);
        act(&mut simulation, 2, Action::Validate);
        // Revoked credential crosses the boundary.
        place(
            &mut simulation,
            EntityKind::RevokedConsent,
            4,
            BOUNDARY_X - 1,
        );
        for slot in simulation.entities.iter_mut().flatten() {
            if slot.kind == EntityKind::RevokedConsent {
                slot.speed = 2;
            }
        }
        simulation.step(Input::IDLE);
        assert!(matches!(simulation.consent, ConsentState::Revoked { .. }));
        act(&mut simulation, 2, Action::Convert);
        assert!(simulation.events().any(|event| matches!(
            event,
            Event::ConvertBlockedConsent(ConsentState::Revoked { .. })
        )));
    }

    #[test]
    fn consent_expires_deterministically() {
        let mut simulation = isolate(RunMode::Standard);
        grant_consent(&mut simulation);
        let ConsentState::Active { until, .. } = simulation.consent else {
            panic!("consent active");
        };
        while simulation.tick < until {
            simulation.step(Input::IDLE);
        }
        assert_eq!(simulation.consent, ConsentState::Inactive);
    }

    #[test]
    fn artifact_is_quarantined_and_crossing_is_a_minor_breach() {
        let mut simulation = isolate(RunMode::Standard);
        place(&mut simulation, EntityKind::Artifact, 1, 500);
        act(&mut simulation, 1, Action::Quarantine);
        assert!(simulation
            .events()
            .any(|event| event == Event::Quarantined(EntityKind::Artifact)));
        place(&mut simulation, EntityKind::Artifact, 1, BOUNDARY_X - 1);
        for slot in simulation.entities.iter_mut().flatten() {
            slot.speed = 2;
        }
        simulation.step(Input::IDLE);
        assert_eq!(simulation.integrity, 94);
    }

    #[test]
    fn unsupported_claim_cannot_increase_evidence() {
        let mut simulation = isolate(RunMode::Standard);
        place(&mut simulation, EntityKind::UnsupportedClaim, 2, 500);
        act(&mut simulation, 2, Action::EvidenceGate);
        assert_eq!(simulation.evidence_points, 0);
        assert!(simulation
            .events()
            .any(|event| matches!(event, Event::WrongTarget(Action::EvidenceGate, _))));
    }

    #[test]
    fn quarantining_good_payload_is_a_false_positive() {
        let mut simulation = isolate(RunMode::Standard);
        place(&mut simulation, EntityKind::Evidence, 0, 500);
        act(&mut simulation, 0, Action::Quarantine);
        assert_eq!(simulation.false_positives, 1);
        assert_eq!(simulation.trust, 47);
    }

    #[test]
    fn unknown_packet_must_be_classified_before_quarantine() {
        let mut simulation = isolate(RunMode::Standard);
        place(&mut simulation, EntityKind::UnknownPacket, 2, 500);
        act(&mut simulation, 2, Action::Quarantine);
        assert!(simulation
            .events()
            .any(|event| event == Event::QuarantineBlockedUnknown));
        act(&mut simulation, 2, Action::Validate);
        assert!(simulation
            .events()
            .any(|event| matches!(event, Event::Revealed(_))));
        assert_eq!(simulation.risk, 4);
    }

    #[test]
    fn release_reports_blockers_and_costs_risk() {
        let mut simulation = isolate(RunMode::Standard);
        act(&mut simulation, 2, Action::Release);
        assert!(simulation
            .events()
            .any(|event| matches!(event, Event::ReleaseRejected(_))));
        assert_eq!(simulation.risk, 6);
        assert_eq!(simulation.status(), Status::Running);
        let (_, count) = simulation.release_blockers();
        assert!(count >= 2);
    }

    #[test]
    fn release_impossible_with_raw_leaks() {
        let mut simulation = isolate(RunMode::Standard);
        simulation.raw_leaks = 1;
        assert_eq!(
            simulation.release_blocker(),
            Some(ReleaseBlocker::RawLeaksPresent)
        );
    }

    #[test]
    fn full_clean_path_seals_standard_mode() {
        let mut simulation = isolate(RunMode::Standard);
        grant_consent(&mut simulation);
        // Evidence to L2 (4 points).
        for _ in 0..2 {
            place(&mut simulation, EntityKind::Evidence, 1, 500);
            act(&mut simulation, 1, Action::EvidenceGate);
        }
        // Validations for SCHEMA; convert and deliver seven for trust >= 90.
        for round in 0..7u8 {
            place(&mut simulation, EntityKind::IntentCandidate, 2, 560);
            act(&mut simulation, 2, Action::Validate);
            act(&mut simulation, 2, Action::Convert);
            // Drive the typed intent to delivery (speed 1 from x=560).
            for _ in 0..700 {
                if simulation.delivered > round {
                    break;
                }
                simulation.step(Input::IDLE);
            }
            assert_eq!(simulation.delivered, round + 1);
            // Keep consent fresh across the long drive.
            if simulation.snapshot().consent_remaining < 800 {
                grant_consent(&mut simulation);
            }
        }
        // Three quarantines for CONTAIN.
        for _ in 0..3 {
            place(&mut simulation, EntityKind::RawFrame, 3, 500);
            act(&mut simulation, 3, Action::Quarantine);
        }
        assert_eq!(simulation.snapshot().gates_passed, 5);
        assert!(simulation.trust >= 90, "trust {}", simulation.trust);
        act(&mut simulation, 2, Action::Release);
        assert_eq!(
            simulation.status(),
            Status::Terminal(TerminalReason::Sealed)
        );
        assert!(matches!(
            simulation.grade(),
            Grade::Sovereign | Grade::Sealed
        ));
    }

    #[test]
    fn grand_run_release_requires_final_phase() {
        let simulation = isolate(RunMode::Grand);
        assert_eq!(
            simulation.release_blocker(),
            Some(ReleaseBlocker::PhaseNotFinal)
        );
    }

    #[test]
    fn grand_run_phases_advance_in_order() {
        let mut simulation = isolate(RunMode::Grand);
        let mut seen = [false; 4];
        seen[0] = true;
        while simulation.status() == Status::Running {
            simulation.step(Input::IDLE);
            for event in simulation.events() {
                if let Event::PhaseChanged(phase) = event {
                    seen[phase as usize] = true;
                }
            }
        }
        assert_eq!(seen, [true; 4]);
        assert_eq!(
            simulation.status(),
            Status::Terminal(TerminalReason::HorizonReached)
        );
    }

    #[test]
    fn horizon_reached_grades_reviewable_when_clean() {
        let mut simulation = isolate(RunMode::Guided);
        while simulation.status() == Status::Running {
            simulation.step(Input::IDLE);
        }
        assert_eq!(
            simulation.status(),
            Status::Terminal(TerminalReason::HorizonReached)
        );
        assert_eq!(simulation.grade(), Grade::Reviewable);
    }

    #[test]
    fn terminal_state_never_resumes() {
        let mut simulation = isolate(RunMode::Standard);
        simulation.status = Status::Terminal(TerminalReason::Sealed);
        let hash = simulation.state_hash();
        for _ in 0..50 {
            simulation.step(Input {
                select_lane: Some(0),
                action: Some(Action::Release),
            });
        }
        assert_eq!(
            simulation.status(),
            Status::Terminal(TerminalReason::Sealed)
        );
        assert_eq!(simulation.state_hash(), hash);
        assert_eq!(simulation.tick(), 0);
    }

    #[test]
    fn metrics_saturate_safely() {
        let mut simulation = isolate(RunMode::Standard);
        simulation.trust = 0;
        simulation.trust = saturate(simulation.trust - 50);
        assert_eq!(simulation.trust, 0);
        simulation.risk = saturate(150);
        assert_eq!(simulation.risk, 100);
        simulation.evidence_points = EVIDENCE_POINTS_MAX;
        place(&mut simulation, EntityKind::Evidence, 1, 500);
        act(&mut simulation, 1, Action::EvidenceGate);
        assert_eq!(simulation.evidence_points, EVIDENCE_POINTS_MAX);
    }

    #[test]
    fn entity_pool_is_bounded() {
        let mut simulation = isolate(RunMode::Standard);
        for index in 0..(MAX_ENTITIES + 8) {
            simulation.insert_entity(EntityKind::Checksum, (index % 5) as u8, 0);
        }
        let live = simulation.entities().iter().flatten().count();
        assert_eq!(live, MAX_ENTITIES);
    }

    #[test]
    fn warmup_script_is_fixed() {
        let mut simulation = Simulation::new(config(RunMode::Standard));
        for _ in 0..WARMUP[0].tick + 1 {
            simulation.step(Input::IDLE);
        }
        let first = simulation
            .entities()
            .iter()
            .flatten()
            .next()
            .copied()
            .expect("warm-up spawned");
        assert_eq!(first.kind, EntityKind::ConsentToken);
        assert_eq!(first.lane, 2);
    }

    #[test]
    fn guided_script_emits_hints_in_order() {
        let mut simulation = Simulation::new(config(RunMode::Guided));
        let mut hints = [false; 7];
        for _ in 0..3_300u32 {
            simulation.step(Input::IDLE);
            for event in simulation.events() {
                if let Event::GuidedStep(hint) = event {
                    hints[hint as usize] = true;
                }
            }
        }
        assert_eq!(hints, [true; 7]);
    }

    #[test]
    fn weakest_control_tracks_worst_bucket() {
        let mut simulation = isolate(RunMode::Standard);
        assert_eq!(simulation.weakest_control(), None);
        for _ in 0..2 {
            place(&mut simulation, EntityKind::RawFrame, 0, BOUNDARY_X - 1);
            for slot in simulation.entities.iter_mut().flatten() {
                slot.speed = 2;
            }
            simulation.step(Input::IDLE);
        }
        assert_eq!(
            simulation.weakest_control(),
            Some(ControlBucket::Quarantine)
        );
    }

    #[test]
    fn streak_and_score_reward_correct_play() {
        let mut simulation = isolate(RunMode::Standard);
        place(&mut simulation, EntityKind::RawFrame, 2, 500);
        act(&mut simulation, 2, Action::Quarantine);
        let after_one = simulation.snapshot();
        assert_eq!(after_one.streak, 1);
        assert!(after_one.score >= 60);
        place(&mut simulation, EntityKind::Evidence, 2, 500);
        act(&mut simulation, 2, Action::Quarantine);
        assert_eq!(simulation.snapshot().streak, 0);
    }

    #[test]
    fn entity_metadata_is_complete_and_codes_round_trip() {
        for kind in EntityKind::ALL {
            assert!(!kind.label().is_empty());
            assert!(!kind.schema_name().is_empty());
            assert!(!kind.symbol().is_empty());
            assert!(!kind.description().is_empty());
            assert_eq!(EntityKind::from_code(kind.code()), Some(kind));
        }
        assert_eq!(EntityKind::from_code(200), None);
    }

    /// Deterministic pseudo-fuzz: 60 seeded random input streams across all
    /// modes. Property: replaying the identical stream reproduces the
    /// identical hash, and core invariants hold at every terminal state.
    #[test]
    fn fuzz_replay_reproducibility_and_invariants() {
        for case in 0..60u64 {
            let mode = RunMode::ALL[(case % 5) as usize];
            let config = SimulationConfig {
                seed: 0xA5A5_0000 + case,
                mode,
                difficulty: Difficulty::Intense,
            };
            let mut driver = Rng64::new(0xF00D ^ case);
            let mut first = Simulation::new(config);
            let mut second = Simulation::new(config);
            for _ in 0..2_500u32 {
                let input = if driver.range(4) == 0 {
                    Input {
                        select_lane: Some(driver.range(LANES as u32) as u8),
                        action: Some(match driver.range(6) {
                            0 => Action::Validate,
                            1 => Action::Convert,
                            2 => Action::Quarantine,
                            3 => Action::ConsentGate,
                            4 => Action::EvidenceGate,
                            _ => Action::Release,
                        }),
                    }
                } else {
                    Input::IDLE
                };
                first.step(input);
                second.step(input);
            }
            assert_eq!(first.state_hash(), second.state_hash(), "case {case}");
            let snapshot = first.snapshot();
            assert!((0..=100).contains(&snapshot.trust));
            assert!((0..=100).contains(&snapshot.risk));
            assert!((0..=100).contains(&snapshot.integrity));
            assert!(snapshot.live_entities as usize <= MAX_ENTITIES);
            if let Status::Terminal(reason) = snapshot.status {
                if reason == TerminalReason::Sealed {
                    assert_eq!(snapshot.raw_leaks, 0, "sealed with leaks, case {case}");
                }
            }
        }
    }

    /// Property: a sealed boundary is impossible while the raw-leak counter
    /// is non-zero, across random metric states.
    #[test]
    fn fuzz_release_fails_closed_with_leaks() {
        let mut driver = Rng64::new(0xBEEF);
        for _ in 0..200 {
            let mut simulation = isolate(RunMode::Standard);
            simulation.trust = 100;
            simulation.integrity = 100;
            simulation.evidence_points = EVIDENCE_POINTS_MAX;
            simulation.gates = (1 << REVIEW_GATES) - 1;
            simulation.risk = driver.range(21) as i32;
            simulation.raw_leaks = 1 + driver.range(2) as u8;
            assert_eq!(
                simulation.release_blocker(),
                Some(ReleaseBlocker::RawLeaksPresent)
            );
        }
    }

    #[test]
    fn audit_rules_are_stricter_than_standard() {
        let standard = RunMode::Standard.rules();
        let audit = RunMode::Audit.rules();
        assert!(audit.risk_max < standard.risk_max);
        assert!(audit.evidence_points_min > standard.evidence_points_min);
        assert!(audit.integrity_min > standard.integrity_min);
    }
}
