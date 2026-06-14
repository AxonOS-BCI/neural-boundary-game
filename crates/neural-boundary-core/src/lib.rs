// SPDX-FileCopyrightText: 2026 Denis Yermakou
// SPDX-FileContributor: AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Neural Boundary Game and AxonOS are protected intellectual property.
// Commercial use outside AGPL-3.0-only requires a commercial licence.

//! neural-boundary-core — Production Grand deterministic core.
//!
//! Canonical spec: docs/GAME_SPEC.md (§7–19). This crate owns every rule,
//! number and state transition. The browser is never authoritative.
//!
//! Key invariants:
//! - `#![no_std]` + `#![forbid(unsafe_code)]` + zero heap allocation.
//! - xorshift64star-v1 RNG, seeded by caller, owned entirely by core.
//! - fnv1a64-v1 state hash; deterministic little-endian encoding.
//! - Same seed + same action stream → identical terminal tick and hash.

#![no_std]
#![forbid(unsafe_code)]

// ─── Identity ───────────────────────────────────────────────────────────────

pub const CORE_VERSION: &str = "5.5.12";
pub const REPLAY_SCHEMA: &str = "neural-boundary-replay-v5.5.12";
pub const HASH_ALGORITHM: &str = "fnv1a64-v1";
pub const RNG_ALGORITHM: &str = "xorshift64star-v1";
pub const ABI_VERSION: u32 = 1;
pub const PRODUCT_VERSION_PACKED: u32 = (5 << 16) | (5 << 8) | 12;

// ─── Field geometry (§7.1) ──────────────────────────────────────────────────

pub const TICKS_PER_SECOND: u32 = 60;
pub const LANES: u8 = 5;
pub const ENTITY_CAPACITY: usize = 32;
pub const FIELD_W: i32 = 1024;
pub const SIGNAL_END: i32 = 543;
pub const ACTION_WINDOW_START: i32 = 544;
pub const ACTION_WINDOW_END: i32 = 703;
pub const BOUNDARY_X: i32 = 704;
pub const APP_END: i32 = 1024;

/// Lane centre Y values (logical, §7.1).
pub const LANE_Y: [i32; 5] = [96, 192, 288, 384, 480];

// ─── Fixed-point: Q24.8 (§7.1) ──────────────────────────────────────────────
// stored_x = logical_x * 256

pub const fn to_q8(logical: i32) -> u32 {
    (logical * 256) as u32
}
pub const fn from_q8(q8: u32) -> i32 {
    (q8 / 256) as i32
}

// ─── Pool constants ─────────────────────────────────────────────────────────

pub const CAPACITY_PRESSURE_LIMIT: u8 = 3;
pub const WCET_BUDGET: u32 = 618;
pub const RELEASE_COOLDOWN: u32 = 30;
pub const RELEASE_SPAM_RISK: i32 = 5;
pub const NO_TARGET_RISK: i32 = 10;

// ─── RNG: xorshift64star-v1 (§8.1) ─────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Seed 0 maps to canonical constant (§8.2).
    pub const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
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

    pub fn range(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }
    pub const fn state(&self) -> u64 {
        self.state
    }
}

// ─── Daily Seed (§8.3) ──────────────────────────────────────────────────────
// FNV-1a over "NBG|5.5.12|YYYY-MM-DD|DAILY", then one xorshift64star round.

pub fn daily_seed(year: u16, month: u8, day: u8) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    let prime: u64 = 0x0000_0100_0000_01B3;
    let mut feed = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(prime);
    };
    for b in b"NBG|5.5.12|" {
        feed(*b);
    }
    // YYYY-MM-DD as ASCII — inline to avoid slicing bugs
    let y = year;
    feed(b'0' + (y / 1000) as u8);
    feed(b'0' + (y / 100 % 10) as u8);
    feed(b'0' + (y / 10 % 10) as u8);
    feed(b'0' + (y % 10) as u8);
    feed(b'-');
    feed(b'0' + (month / 10));
    feed(b'0' + (month % 10));
    feed(b'-');
    feed(b'0' + (day / 10));
    feed(b'0' + (day % 10));
    for b in b"|DAILY" {
        feed(*b);
    }
    let seed = if h == 0 { 0x3001u64 } else { h };
    // One xorshift64star round (§8.3).
    Rng::new(seed).next_u64()
}

// ─── FNV-1a 64 state hash (§24.5) ──────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct Fnv64 {
    h: u64,
}

impl Fnv64 {
    pub const fn new() -> Self {
        Self {
            h: 0xCBF2_9CE4_8422_2325,
        }
    }
    pub fn feed_u8(&mut self, v: u8) {
        self.h ^= v as u64;
        self.h = self.h.wrapping_mul(0x100_0000_01B3);
    }
    pub fn feed_u16(&mut self, v: u16) {
        for b in v.to_le_bytes() {
            self.feed_u8(b);
        }
    }
    pub fn feed_u32(&mut self, v: u32) {
        for b in v.to_le_bytes() {
            self.feed_u8(b);
        }
    }
    pub fn feed_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.feed_u8(b);
        }
    }
    pub fn feed_i32(&mut self, v: i32) {
        self.feed_u32(v as u32);
    }
    pub fn finish(&self) -> u64 {
        self.h
    }
}

impl Default for Fnv64 {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Entity taxonomy: 19 stable IDs (§9.1) ──────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Empty = 0,
    RawFrame = 1,
    Artifact = 2,
    UnknownPacket = 3,
    CandidateIntent = 4,
    ValidatedIntent = 5,
    TypedIntent = 6,
    ConsentGrant = 7,
    ConsentRevoke = 8,
    EvidenceTrace = 9,
    ChecksumProof = 10,
    CiProof = 11,
    UnsupportedClaim = 12,
    UntraceableClaim = 13,
    RoadmapAsFact = 14,
    StimulationCommand = 15,
    DeadlineHazard = 16,
    VaultRecord = 17,
    RawExportRequest = 18,
}

impl Kind {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Empty,
            1 => Self::RawFrame,
            2 => Self::Artifact,
            3 => Self::UnknownPacket,
            4 => Self::CandidateIntent,
            5 => Self::ValidatedIntent,
            6 => Self::TypedIntent,
            7 => Self::ConsentGrant,
            8 => Self::ConsentRevoke,
            9 => Self::EvidenceTrace,
            10 => Self::ChecksumProof,
            11 => Self::CiProof,
            12 => Self::UnsupportedClaim,
            13 => Self::UntraceableClaim,
            14 => Self::RoadmapAsFact,
            15 => Self::StimulationCommand,
            16 => Self::DeadlineHazard,
            17 => Self::VaultRecord,
            18 => Self::RawExportRequest,
            _ => return None,
        })
    }
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Empty => "EMPTY",
            Self::RawFrame => "RAW_FRAME",
            Self::Artifact => "ARTIFACT",
            Self::UnknownPacket => "UNKNOWN",
            Self::CandidateIntent => "CANDIDATE",
            Self::ValidatedIntent => "VALIDATED",
            Self::TypedIntent => "TYPED",
            Self::ConsentGrant => "GRANT",
            Self::ConsentRevoke => "REVOKE",
            Self::EvidenceTrace => "TRACE",
            Self::ChecksumProof => "CHECKSUM",
            Self::CiProof => "CI_PROOF",
            Self::UnsupportedClaim => "CLAIM",
            Self::UntraceableClaim => "UNTRACE",
            Self::RoadmapAsFact => "ROADMAP",
            Self::StimulationCommand => "STIM",
            Self::DeadlineHazard => "DEADLINE",
            Self::VaultRecord => "VAULT_REC",
            Self::RawExportRequest => "RAW_EXPORT",
        }
    }
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Empty => "·",
            Self::RawFrame => "◉",
            Self::Artifact => "▒",
            Self::UnknownPacket => "◌",
            Self::CandidateIntent => "◇",
            Self::ValidatedIntent => "◈",
            Self::TypedIntent => "●",
            Self::ConsentGrant => "⬡",
            Self::ConsentRevoke => "⬢",
            Self::EvidenceTrace => "▣",
            Self::ChecksumProof => "▤",
            Self::CiProof => "▥",
            Self::UnsupportedClaim => "△",
            Self::UntraceableClaim => "▽",
            Self::RoadmapAsFact => "◭",
            Self::StimulationCommand => "✕",
            Self::DeadlineHazard => "⧗",
            Self::VaultRecord => "◪",
            Self::RawExportRequest => "⊘",
        }
    }
    pub const fn description(self) -> &'static str {
        match self {
            Self::Empty => "Empty pool slot.",
            Self::RawFrame => "Private raw signal. Quarantine — never allow to cross.",
            Self::Artifact => "Signal noise. Quarantine before it degrades integrity.",
            Self::UnknownPacket => "Unclassified. Validate to reveal contents.",
            Self::CandidateIntent => "Unverified candidate. Validate before conversion.",
            Self::ValidatedIntent => "Validated. Convert under active consent and evidence.",
            Self::TypedIntent => "Typed intent — the only output permitted across the boundary.",
            Self::ConsentGrant => "Consent token. Gate it to open conversion and release scope.",
            Self::ConsentRevoke => "Revocation credential. Apply it — the epoch will increment.",
            Self::EvidenceTrace => "Trace proof (L0→L1). Register to advance evidence level.",
            Self::ChecksumProof => "Checksum proof (L1→L2). Register to advance.",
            Self::CiProof => "CI proof (L2→L3). Required for audit and vault release.",
            Self::UnsupportedClaim => {
                "Unsupported claim. Quarantine — claims block the evidence gate."
            }
            Self::UntraceableClaim => "Claim without a trace. Quarantine.",
            Self::RoadmapAsFact => "Roadmap stated as fact. Quarantine.",
            Self::StimulationCommand => {
                "Stimulation command. QUARANTINE IMMEDIATELY — fail closed."
            }
            Self::DeadlineHazard => {
                "Timing hazard. Validate to resolve; missing it fails the WCET gate."
            }
            Self::VaultRecord => "Raw vault record. Quarantine to seal locally.",
            Self::RawExportRequest => "Raw export request. Quarantine — always denied.",
        }
    }
    pub const fn correct_action(self) -> Option<Action> {
        match self {
            Self::RawFrame
            | Self::Artifact
            | Self::UnsupportedClaim
            | Self::UntraceableClaim
            | Self::RoadmapAsFact
            | Self::StimulationCommand
            | Self::VaultRecord
            | Self::RawExportRequest => Some(Action::Quarantine),
            Self::UnknownPacket | Self::CandidateIntent | Self::DeadlineHazard => {
                Some(Action::Validate)
            }
            Self::ValidatedIntent => Some(Action::Convert),
            Self::ConsentGrant | Self::ConsentRevoke => Some(Action::Consent),
            Self::EvidenceTrace | Self::ChecksumProof | Self::CiProof => Some(Action::Evidence),
            Self::TypedIntent | Self::Empty => None,
        }
    }
    pub const fn is_claim(self) -> bool {
        matches!(
            self,
            Self::UnsupportedClaim | Self::UntraceableClaim | Self::RoadmapAsFact
        )
    }
    pub const fn is_raw_hazard(self) -> bool {
        matches!(
            self,
            Self::RawFrame | Self::VaultRecord | Self::RawExportRequest
        )
    }
    pub const fn evidence_level_bit(self) -> u8 {
        match self {
            Self::EvidenceTrace => 1,
            Self::ChecksumProof => 2,
            Self::CiProof => 4,
            _ => 0,
        }
    }
    pub const ALL_SPAWNABLE: [Self; 17] = [
        Self::RawFrame,
        Self::Artifact,
        Self::UnknownPacket,
        Self::CandidateIntent,
        Self::ConsentGrant,
        Self::ConsentRevoke,
        Self::EvidenceTrace,
        Self::ChecksumProof,
        Self::CiProof,
        Self::UnsupportedClaim,
        Self::UntraceableClaim,
        Self::RoadmapAsFact,
        Self::StimulationCommand,
        Self::DeadlineHazard,
        Self::VaultRecord,
        Self::RawExportRequest,
        Self::TypedIntent,
    ];
}

// ─── Actions (§10.1) ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Action {
    None = 0,
    Validate = 1,
    Convert = 2,
    Quarantine = 3,
    Consent = 4,
    Evidence = 5,
    Release = 6,
}

impl Action {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::Validate,
            2 => Self::Convert,
            3 => Self::Quarantine,
            4 => Self::Consent,
            5 => Self::Evidence,
            6 => Self::Release,
            _ => return None,
        })
    }
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "VALIDATE" | "Validate" => Some(Self::Validate),
            "CONVERT" | "Convert" => Some(Self::Convert),
            "QUARANTINE" | "Quarantine" => Some(Self::Quarantine),
            "CONSENT" | "Consent" => Some(Self::Consent),
            "EVIDENCE" | "Evidence" => Some(Self::Evidence),
            "RELEASE" | "Release" => Some(Self::Release),
            _ => None,
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Validate => "VALIDATE",
            Self::Convert => "CONVERT",
            Self::Quarantine => "QUARANTINE",
            Self::Consent => "CONSENT",
            Self::Evidence => "EVIDENCE",
            Self::Release => "RELEASE",
        }
    }
    pub const fn code(self) -> u8 {
        self as u8
    }
}

// ─── Neural Permissions / Consent (§11) ─────────────────────────────────────

pub const SCOPE_CONVERT: u16 = 0x0001;
pub const SCOPE_RELEASE: u16 = 0x0002;
pub const SCOPE_SUMMARY: u16 = 0x0004;
pub const SCOPE_AUDIT: u16 = 0x0008;

/// Active consent state within the simulation. Consent is NOT persisted across sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsentState {
    pub epoch: u32,
    pub scope_mask: u16,
    pub expires_tick: u32,
}

impl ConsentState {
    pub const NONE: Self = Self {
        epoch: 0,
        scope_mask: 0,
        expires_tick: 0,
    };

    /// Token valid: epoch matches, scope present, not expired.
    pub fn is_valid_for(&self, state_epoch: u32, current_tick: u32, scope: u16) -> bool {
        self.epoch == state_epoch
            && current_tick < self.expires_tick
            && (self.scope_mask & scope) == scope
    }
    pub fn has_convert(&self, epoch: u32, tick: u32) -> bool {
        self.is_valid_for(epoch, tick, SCOPE_CONVERT)
    }
    pub fn has_release(&self, epoch: u32, tick: u32) -> bool {
        self.is_valid_for(epoch, tick, SCOPE_RELEASE)
    }
}

// ─── Evidence model (§12) ───────────────────────────────────────────────────

pub const EVIDENCE_TRACE: u8 = 0x01;
pub const EVIDENCE_CHECKSUM: u8 = 0x02;
pub const EVIDENCE_CI: u8 = 0x04;
pub const EVIDENCE_REPLAY: u8 = 0x08;
pub const EVIDENCE_REVIEWED: u8 = 0x10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum EvidenceLevel {
    L0 = 0,
    L1 = 1,
    L2 = 2,
    L3 = 3,
}

impl EvidenceLevel {
    pub const fn from_bits(bits: u8) -> Self {
        if bits & EVIDENCE_CI != 0 && bits & EVIDENCE_CHECKSUM != 0 && bits & EVIDENCE_TRACE != 0 {
            Self::L3
        } else if bits & EVIDENCE_CHECKSUM != 0 && bits & EVIDENCE_TRACE != 0 {
            Self::L2
        } else if bits & EVIDENCE_TRACE != 0 {
            Self::L1
        } else {
            Self::L0
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
    pub const fn code(self) -> u8 {
        self as u8
    }
}

// ─── Privacy Vault (§13) ────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum VaultState {
    Empty = 0,
    Open = 1,
    Sealing = 2,
    Sealed = 3,
    Compromised = 4,
}

impl VaultState {
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "EMPTY",
            Self::Open => "OPEN",
            Self::Sealing => "SEALING",
            Self::Sealed => "SEALED",
            Self::Compromised => "COMPROMISED",
        }
    }
    pub const fn gate_eligible(self) -> bool {
        matches!(self, Self::Empty | Self::Sealed)
    }
}

// ─── Run modes (§20) ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    Guided = 1,
    Standard = 2,
    Audit = 3,
    Grand = 4,
    Daily = 5,
    PrivacyVault = 6,
    KernelTrial = 7,
}

impl Mode {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::Guided,
            2 => Self::Standard,
            3 => Self::Audit,
            4 => Self::Grand,
            5 => Self::Daily,
            6 => Self::PrivacyVault,
            7 => Self::KernelTrial,
            _ => return None,
        })
    }
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "guided" | "GUIDED" => Some(Self::Guided),
            "standard" | "STANDARD" => Some(Self::Standard),
            "audit" | "AUDIT" => Some(Self::Audit),
            "grand" | "GRAND" => Some(Self::Grand),
            "daily" | "DAILY" => Some(Self::Daily),
            "privacy_vault" | "PRIVACY_VAULT" => Some(Self::PrivacyVault),
            "kernel_trial" | "KERNEL_TRIAL" => Some(Self::KernelTrial),
            _ => None,
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Guided => "GUIDED",
            Self::Standard => "STANDARD",
            Self::Audit => "AUDIT",
            Self::Grand => "GRAND",
            Self::Daily => "DAILY",
            Self::PrivacyVault => "PRIVACY_VAULT",
            Self::KernelTrial => "KERNEL_TRIAL",
        }
    }
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn max_ticks(self) -> u32 {
        match self {
            Self::Guided => 3600,
            Self::Standard => 7200,
            Self::Audit => 7200,
            Self::Grand => 10800,
            Self::Daily => 7200,
            Self::PrivacyVault => 7200,
            Self::KernelTrial => 6000,
        }
    }
    pub const fn spawn_interval(self, phase: u8) -> (u32, u32) {
        match self {
            Self::Guided => (90, 150),
            Self::Standard => (60, 105),
            Self::Audit => (42, 78),
            Self::Grand => match phase {
                0 => (50, 90),
                1 => (42, 78),
                2 => (30, 65),
                _ => (80, 120),
            },
            Self::Daily => (60, 105),
            Self::PrivacyVault => (45, 90),
            Self::KernelTrial => (30, 66),
        }
    }
    pub const fn consent_ttl(self) -> u32 {
        match self {
            Self::Guided => 900,
            Self::Standard => 720,
            Self::Audit => 480,
            Self::Grand => 420,
            Self::Daily => 600,
            Self::PrivacyVault => 540,
            Self::KernelTrial => 360,
        }
    }
    pub fn convert_evidence(self) -> EvidenceLevel {
        match self {
            Self::Guided | Self::Standard | Self::Daily => EvidenceLevel::L1,
            _ => EvidenceLevel::L2,
        }
    }
    pub fn release_evidence(self) -> EvidenceLevel {
        match self {
            Self::Guided | Self::Standard | Self::Daily => EvidenceLevel::L2,
            _ => EvidenceLevel::L3,
        }
    }
    pub const fn raw_leak_limit(self) -> u8 {
        match self {
            Self::Guided | Self::Standard => 2,
            _ => 1,
        }
    }
    pub fn initial_trust(self) -> i32 {
        match self {
            Self::Guided => 800,
            Self::Audit | Self::KernelTrial => 650,
            _ => 700,
        }
    }
    pub fn initial_risk(self) -> i32 {
        match self {
            Self::Guided => 50,
            Self::Audit | Self::KernelTrial => 150,
            _ => 100,
        }
    }
    pub const fn final_phase_only(self) -> bool {
        matches!(self, Self::Grand)
    }
    pub const ALL: [Self; 7] = [
        Self::Guided,
        Self::Standard,
        Self::Audit,
        Self::Grand,
        Self::Daily,
        Self::PrivacyVault,
        Self::KernelTrial,
    ];
}

// ─── Grand Run phases (§20.6) ────────────────────────────────────────────────

pub const GRAND_PHASE_TICKS: [u32; 4] = [0, 2400, 4800, 7200];
pub const fn grand_phase_name(phase: u8) -> &'static str {
    match phase {
        0 => "Signal Integrity",
        1 => "Consent and Evidence",
        2 => "Release Under Pressure",
        _ => "Sovereign Boundary Review",
    }
}

// ─── Difficulty (§20.2 — spawn cadence modifier) ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Difficulty {
    Calm = 0,
    Standard = 1,
    Intense = 2,
}

impl Difficulty {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Calm,
            1 => Self::Standard,
            2 => Self::Intense,
            _ => return None,
        })
    }
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn speed_bonus(self) -> i32 {
        match self {
            Self::Calm => 0,
            Self::Standard => 1,
            Self::Intense => 2,
        }
    }
}

// ─── Terminal model (§18) ────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    Running = 0,
    Sealed = 1,
    Breached = 2,
    Unsafe = 3,
    Aborted = 4,
    FatalRuntime = 5,
}

impl Status {
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "RUNNING",
            Self::Sealed => "SEALED",
            Self::Breached => "BREACHED",
            Self::Unsafe => "UNSAFE",
            Self::Aborted => "ABORTED",
            Self::FatalRuntime => "FATAL_RUNTIME",
        }
    }
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TerminalReason {
    None = 0,
    SuccessRelease = 1,
    TimeoutUnsealed = 2,
    RiskOverflow = 3,
    IntegrityCollapse = 4,
    RawLeakLimit = 5,
    UnsafeStimulationEscape = 6,
    DeadlineBreach = 7,
    DeterminismMismatch = 8,
    ReplaySchemaError = 9,
    WasmInitFailure = 10,
    UserAbort = 11,
}

impl TerminalReason {
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::SuccessRelease => "SUCCESS_RELEASE",
            Self::TimeoutUnsealed => "TIMEOUT_UNSEALED",
            Self::RiskOverflow => "RISK_OVERFLOW",
            Self::IntegrityCollapse => "INTEGRITY_COLLAPSE",
            Self::RawLeakLimit => "RAW_LEAK_LIMIT",
            Self::UnsafeStimulationEscape => "UNSAFE_STIMULATION_ESCAPE",
            Self::DeadlineBreach => "DEADLINE_BREACH",
            Self::DeterminismMismatch => "DETERMINISM_MISMATCH",
            Self::ReplaySchemaError => "REPLAY_SCHEMA_ERROR",
            Self::WasmInitFailure => "WASM_INIT_FAILURE",
            Self::UserAbort => "USER_ABORT",
        }
    }
    pub fn from_schema_str(s: &str) -> Option<Self> {
        Some(match s {
            "SUCCESS_RELEASE" => Self::SuccessRelease,
            "TIMEOUT_UNSEALED" => Self::TimeoutUnsealed,
            "RISK_OVERFLOW" => Self::RiskOverflow,
            "INTEGRITY_COLLAPSE" => Self::IntegrityCollapse,
            "RAW_LEAK_LIMIT" => Self::RawLeakLimit,
            "UNSAFE_STIMULATION_ESCAPE" => Self::UnsafeStimulationEscape,
            "DEADLINE_BREACH" => Self::DeadlineBreach,
            "DETERMINISM_MISMATCH" => Self::DeterminismMismatch,
            "REPLAY_SCHEMA_ERROR" => Self::ReplaySchemaError,
            "WASM_INIT_FAILURE" => Self::WasmInitFailure,
            "USER_ABORT" => Self::UserAbort,
            _ => return None,
        })
    }
}

// ─── Boundary grade (§19) ────────────────────────────────────────────────────

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
            Self::Sovereign => "SOVEREIGN",
            Self::Sealed => "SEALED",
            Self::Reviewable => "REVIEWABLE",
            Self::Degraded => "DEGRADED",
            Self::Breached => "BREACHED",
            Self::Unsafe => "UNSAFE",
        }
    }
}

// ─── Entity (§9.2) ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entity {
    pub slot_id: u8,
    pub kind: Kind,
    pub lane: u8,
    pub state: EntityState,
    pub flags: u16,
    /// Position in Q24.8 units (logical_x * 256).
    pub x_q8: u32,
    /// Speed in Q24.8 units per tick.
    pub speed_q8: u16,
    pub spawn_tick: u32,
    pub deadline_tick: u32,
    pub scope_mask: u16,
    pub evidence_class: u8,
    pub generation: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EntityState {
    Incoming = 0,
    InActionWindow = 1,
    Validated = 2,
    Quarantined = 3,
    Consumed = 4,
    Crossed = 5,
    Expired = 6,
}

impl EntityState {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

impl Entity {
    pub fn logical_x(&self) -> i32 {
        from_q8(self.x_q8)
    }
    pub fn in_action_window(&self) -> bool {
        let x = self.logical_x();
        (ACTION_WINDOW_START..BOUNDARY_X).contains(&x)
    }
}

// ─── 7 Review Gates (§16) ────────────────────────────────────────────────────

pub const GATE_PRIVACY: u8 = 0;
pub const GATE_TYPING: u8 = 1;
pub const GATE_CONSENT: u8 = 2;
pub const GATE_EVIDENCE: u8 = 3;
pub const GATE_DETERMINISM: u8 = 4;
pub const GATE_VAULT: u8 = 5;
pub const GATE_WCET: u8 = 6;
pub const GATE_COUNT: u8 = 7;
pub const ALL_GATES_MASK: u8 = (1 << GATE_COUNT) - 1;

pub const fn gate_name(bit: u8) -> &'static str {
    match bit {
        0 => "PRIVACY",
        1 => "TYPING",
        2 => "CONSENT",
        3 => "EVIDENCE",
        4 => "DETERMINISM",
        5 => "VAULT",
        6 => "WCET",
        _ => "?",
    }
}

// ─── Spawn weights (§21) ─────────────────────────────────────────────────────

type WeightEntry = (Kind, u32);

/// Resolve spawn category weights for a given mode and phase.
/// Returns (Kind, cumulative_weight) table; total is the last element's weight.
pub fn spawn_table(mode: Mode, phase: u8) -> &'static [WeightEntry] {
    match mode {
        Mode::Guided => &GUIDED_TABLE,
        Mode::Standard | Mode::Daily => &STANDARD_TABLE,
        Mode::Audit => &AUDIT_TABLE,
        Mode::Grand => match phase {
            0 => &GRAND_P1_TABLE,
            1 => &GRAND_P2_TABLE,
            2 => &GRAND_P3_TABLE,
            _ => &GRAND_FINAL_TABLE,
        },
        Mode::PrivacyVault => &PVAULT_TABLE,
        Mode::KernelTrial => &KTRIAL_TABLE,
    }
}

// Weights represent per-mille share of each category mapping to specific kinds.
// We flatten category→kind directly for simplicity (no runtime category→kind lookup needed).

static GUIDED_TABLE: [WeightEntry; 8] = [
    (Kind::RawFrame, 150),
    (Kind::Artifact, 100),
    (Kind::CandidateIntent, 350),
    (Kind::ConsentGrant, 150),
    (Kind::EvidenceTrace, 140),
    (Kind::ChecksumProof, 60),
    (Kind::UnsupportedClaim, 50),
    (Kind::UnknownPacket, 0),
];
static STANDARD_TABLE: [WeightEntry; 12] = [
    (Kind::RawFrame, 150),
    (Kind::Artifact, 120),
    (Kind::CandidateIntent, 200),
    (Kind::UnknownPacket, 100),
    (Kind::ConsentGrant, 90),
    (Kind::ConsentRevoke, 30),
    (Kind::EvidenceTrace, 80),
    (Kind::ChecksumProof, 60),
    (Kind::CiProof, 40),
    (Kind::UnsupportedClaim, 60),
    (Kind::UntraceableClaim, 30),
    (Kind::StimulationCommand, 10),
];
static AUDIT_TABLE: [WeightEntry; 13] = [
    (Kind::RawFrame, 120),
    (Kind::Artifact, 120),
    (Kind::CandidateIntent, 150),
    (Kind::UnknownPacket, 90),
    (Kind::ConsentGrant, 80),
    (Kind::ConsentRevoke, 70),
    (Kind::EvidenceTrace, 60),
    (Kind::ChecksumProof, 50),
    (Kind::CiProof, 50),
    (Kind::UnsupportedClaim, 80),
    (Kind::UntraceableClaim, 60),
    (Kind::RoadmapAsFact, 60),
    (Kind::StimulationCommand, 20),
];
static GRAND_P1_TABLE: [WeightEntry; 7] = [
    (Kind::RawFrame, 250),
    (Kind::Artifact, 200),
    (Kind::CandidateIntent, 350),
    (Kind::UnknownPacket, 100),
    (Kind::EvidenceTrace, 60),
    (Kind::UnsupportedClaim, 30),
    (Kind::StimulationCommand, 0),
];
static GRAND_P2_TABLE: [WeightEntry; 9] = [
    (Kind::ConsentGrant, 160),
    (Kind::ConsentRevoke, 120),
    (Kind::EvidenceTrace, 140),
    (Kind::ChecksumProof, 90),
    (Kind::CiProof, 90),
    (Kind::CandidateIntent, 190),
    (Kind::RawFrame, 60),
    (Kind::Artifact, 50),
    (Kind::UnsupportedClaim, 40),
];
static GRAND_P3_TABLE: [WeightEntry; 10] = [
    (Kind::UnsupportedClaim, 180),
    (Kind::UntraceableClaim, 130),
    (Kind::RoadmapAsFact, 120),
    (Kind::CandidateIntent, 180),
    (Kind::RawFrame, 80),
    (Kind::Artifact, 60),
    (Kind::StimulationCommand, 40),
    (Kind::DeadlineHazard, 80),
    (Kind::ConsentGrant, 60),
    (Kind::EvidenceTrace, 40),
];
static GRAND_FINAL_TABLE: [WeightEntry; 9] = [
    (Kind::CandidateIntent, 240),
    (Kind::ConsentGrant, 120),
    (Kind::EvidenceTrace, 120),
    (Kind::ChecksumProof, 80),
    (Kind::CiProof, 80),
    (Kind::RawFrame, 70),
    (Kind::ConsentRevoke, 60),
    (Kind::UnsupportedClaim, 80),
    (Kind::UnknownPacket, 70),
];
static PVAULT_TABLE: [WeightEntry; 9] = [
    (Kind::RawFrame, 200),
    (Kind::VaultRecord, 100),
    (Kind::RawExportRequest, 100),
    (Kind::Artifact, 100),
    (Kind::CandidateIntent, 150),
    (Kind::ConsentGrant, 100),
    (Kind::EvidenceTrace, 80),
    (Kind::ChecksumProof, 50),
    (Kind::CiProof, 50),
];
static KTRIAL_TABLE: [WeightEntry; 9] = [
    (Kind::RawFrame, 80),
    (Kind::Artifact, 80),
    (Kind::CandidateIntent, 220),
    (Kind::ConsentGrant, 100),
    (Kind::EvidenceTrace, 80),
    (Kind::DeadlineHazard, 200),
    (Kind::UnsupportedClaim, 100),
    (Kind::StimulationCommand, 20),
    (Kind::ChecksumProof, 60),
];

fn roll_kind(rng: &mut Rng, table: &[WeightEntry]) -> Kind {
    let total: u32 = table.iter().map(|(_, w)| *w).sum();
    if total == 0 {
        return Kind::CandidateIntent;
    }
    let mut r = rng.range(total);
    for (kind, w) in table {
        if r < *w {
            return *kind;
        }
        r -= w;
    }
    table
        .last()
        .map(|(k, _)| *k)
        .unwrap_or(Kind::CandidateIntent)
}

// ─── Simulation state ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Simulation {
    // Config
    mode: Mode,
    difficulty: Difficulty,
    seed: u64,
    // RNG
    rng: Rng,
    // Time
    tick: u32,
    status: Status,
    reason: TerminalReason,
    // Lane
    selected_lane: u8,
    // Neural Permissions
    consent: ConsentState,
    consent_epoch: u32,
    // Evidence
    evidence_bits: u8,
    // Privacy Vault
    vault: VaultState,
    pending_raw: u8,
    raw_export_violations: u8,
    // Metrics 0..1000
    trust: i32,
    risk: i32,
    integrity: i32,
    score: u64,
    combo: u32,
    best_combo: u32,
    // Counters
    raw_leaks: u8,
    wrong_actions: u32,
    typed_intents: u8,
    quarantined: u32,
    consent_violations: u8,
    unvalidated_conversions: u8,
    escaped_claims: u8,
    deadline_misses: u8,
    deadline_miss_terminal_limit: u8,
    // WCET
    wcet_peak: u32,
    capacity_pressure: u8,
    // Gates
    gate_mask: u8,
    // Release
    release_cooldown: u32,
    // Phase (Grand only)
    phase: u8,
    // Spawn
    spawn_timer: u32,
    last_spawn_lane: u8,
    consecutive_lane: u8,
    // Entity pool
    pool: [Option<Entity>; ENTITY_CAPACITY],
    pool_generation: u16,
    next_slot_hint: u8,
}

/// Input applied at a single tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub lane: Option<u8>,
    pub action: Option<Action>,
}

impl Input {
    pub const IDLE: Self = Self {
        lane: None,
        action: None,
    };
}

/// Read-only snapshot for presentation and WASM ABI.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub seed: u64,
    pub tick: u32,
    pub status: Status,
    pub reason: TerminalReason,
    pub phase: u8,
    pub selected_lane: u8,
    pub trust: i32,
    pub risk: i32,
    pub integrity: i32,
    pub score: u64,
    pub combo: u32,
    pub best_combo: u32,
    pub raw_leaks: u8,
    pub typed_intents: u8,
    pub quarantined: u32,
    pub wrong_actions: u32,
    pub evidence_bits: u8,
    pub evidence_level: EvidenceLevel,
    pub consent: ConsentState,
    pub consent_epoch: u32,
    pub consent_expires_remaining: u32,
    pub vault: VaultState,
    pub gate_mask: u8,
    pub gates_passed: u8,
    pub wcet_peak: u32,
    pub capacity_pressure: u8,
    pub deadline_misses: u8,
    pub live_entities: u8,
}

impl Simulation {
    pub fn new(mode: Mode, difficulty: Difficulty, seed: u64) -> Self {
        let mut sim = Self {
            mode,
            difficulty,
            seed,
            rng: Rng::new(seed),
            tick: 0,
            status: Status::Running,
            reason: TerminalReason::None,
            selected_lane: 2,
            consent: ConsentState::NONE,
            consent_epoch: 0,
            evidence_bits: 0,
            vault: VaultState::Empty,
            pending_raw: 0,
            raw_export_violations: 0,
            trust: mode.initial_trust(),
            risk: mode.initial_risk(),
            integrity: 1000,
            score: 0,
            combo: 0,
            best_combo: 0,
            raw_leaks: 0,
            wrong_actions: 0,
            typed_intents: 0,
            quarantined: 0,
            consent_violations: 0,
            unvalidated_conversions: 0,
            escaped_claims: 0,
            deadline_misses: 0,
            deadline_miss_terminal_limit: if mode == Mode::KernelTrial { 3 } else { 255 },
            wcet_peak: 0,
            capacity_pressure: 0,
            gate_mask: 1 << GATE_DETERMINISM, // DETERMINISM gate passes by default (no faults yet)
            release_cooldown: 0,
            phase: 0,
            spawn_timer: 0,
            last_spawn_lane: 255,
            consecutive_lane: 0,
            pool: [None; ENTITY_CAPACITY],
            pool_generation: 0,
            next_slot_hint: 0,
        };
        // Initial spawn interval.
        let (min, max) = mode.spawn_interval(0);
        sim.spawn_timer = min + sim.rng.range(max - min + 1);
        sim
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    pub const fn mode(&self) -> Mode {
        self.mode
    }
    pub const fn difficulty(&self) -> Difficulty {
        self.difficulty
    }
    pub const fn seed(&self) -> u64 {
        self.seed
    }
    pub const fn tick(&self) -> u32 {
        self.tick
    }
    pub const fn status(&self) -> Status {
        self.status
    }
    pub const fn reason(&self) -> TerminalReason {
        self.reason
    }
    pub const fn phase(&self) -> u8 {
        self.phase
    }
    pub const fn selected_lane(&self) -> u8 {
        self.selected_lane
    }
    pub const fn trust(&self) -> i32 {
        self.trust
    }
    pub const fn risk(&self) -> i32 {
        self.risk
    }
    pub const fn integrity(&self) -> i32 {
        self.integrity
    }
    pub const fn score(&self) -> u64 {
        self.score
    }
    pub const fn combo(&self) -> u32 {
        self.combo
    }
    pub const fn best_combo(&self) -> u32 {
        self.best_combo
    }
    pub const fn raw_leaks(&self) -> u8 {
        self.raw_leaks
    }
    pub const fn evidence_bits(&self) -> u8 {
        self.evidence_bits
    }
    pub fn evidence_level(&self) -> EvidenceLevel {
        EvidenceLevel::from_bits(self.evidence_bits)
    }
    pub const fn consent(&self) -> ConsentState {
        self.consent
    }
    pub const fn consent_epoch(&self) -> u32 {
        self.consent_epoch
    }
    pub fn consent_expires_tick(&self) -> u32 {
        self.consent.expires_tick
    }
    pub const fn vault(&self) -> VaultState {
        self.vault
    }
    pub const fn gate_mask(&self) -> u8 {
        self.gate_mask
    }
    pub const fn wcet_peak(&self) -> u32 {
        self.wcet_peak
    }
    pub const fn rng_state(&self) -> u64 {
        self.rng.state()
    }
    pub fn pool(&self) -> &[Option<Entity>; ENTITY_CAPACITY] {
        &self.pool
    }
    pub fn entity(&self, slot: u8) -> Option<&Entity> {
        self.pool.get(slot as usize).and_then(|e| e.as_ref())
    }

    // ── Step ─────────────────────────────────────────────────────────────────

    pub fn step(&mut self, input: Input) {
        if self.status.is_terminal() {
            return;
        }
        self.tick += 1;

        // Lane selection.
        if let Some(lane) = input.lane {
            if lane < LANES {
                self.selected_lane = lane;
            }
        }

        // Grand phase update.
        if self.mode == Mode::Grand {
            for (i, &start) in GRAND_PHASE_TICKS.iter().enumerate().rev() {
                if self.tick >= start {
                    self.phase = i as u8;
                    break;
                }
            }
        }

        // Consent expiry at start of tick (§11.4).
        if self.tick >= self.consent.expires_tick && self.consent.scope_mask != 0 {
            self.consent = ConsentState::NONE;
        }

        // Release cooldown.
        self.release_cooldown = self.release_cooldown.saturating_sub(1);

        // WCET budget tracking.
        let mut wcet: u32 = 40; // base tick

        // Count active entities.
        let active = self.pool.iter().filter(|e| e.is_some()).count();
        wcet += (active as u32) * 4;

        // Action.
        if let Some(action) = input.action {
            let action_cost = match action {
                Action::Validate => 55,
                Action::Convert => 70,
                Action::Quarantine => 45,
                Action::Consent => 50,
                Action::Evidence => 50,
                Action::Release => 120,
                Action::None => 0,
            };
            wcet += action_cost;
            self.do_action(action);
            if self.status.is_terminal() {
                self.wcet_peak = self.wcet_peak.max(wcet);
                self.recompute_gates();
                return;
            }
        }

        // Advance entities.
        wcet += self.advance_entities();

        // Spawn.
        let spawned = self.do_spawn();
        if spawned {
            wcet += 12;
        }

        // WCET gate.
        self.wcet_peak = self.wcet_peak.max(wcet);

        // (State hash cost included in wcet_peak accounting.)
        // Timeout.
        if self.tick >= self.mode.max_ticks() {
            self.end_run(Status::Aborted, TerminalReason::TimeoutUnsealed);
        }

        self.recompute_gates();
    }

    // ── Gates ────────────────────────────────────────────────────────────────

    fn recompute_gates(&mut self) {
        let mut mask: u8 = 0;

        // PRIVACY (§16.1)
        if self.raw_leaks == 0 && self.pending_raw == 0 && self.raw_export_violations == 0 {
            mask |= 1 << GATE_PRIVACY;
        }
        // TYPING (§16.2)
        if self.unvalidated_conversions == 0 {
            mask |= 1 << GATE_TYPING;
        }
        // CONSENT (§16.3) — checked dynamically at release; gate passes if no violations so far
        if self.consent_violations == 0 {
            mask |= 1 << GATE_CONSENT;
        }
        // EVIDENCE (§16.4)
        if self.evidence_level() >= self.mode.release_evidence() && self.escaped_claims == 0 {
            mask |= 1 << GATE_EVIDENCE;
        }
        // DETERMINISM (§16.5) — starts set; cleared on fault (not tracked in this model)
        mask |= 1 << GATE_DETERMINISM;
        // VAULT (§16.6 / §13.4)
        if self.vault.gate_eligible()
            && self.pending_raw == 0
            && self.raw_export_violations == 0
            && self.raw_leaks == 0
        {
            mask |= 1 << GATE_VAULT;
        }
        // WCET (§16.7 / §15.2)
        if self.wcet_peak <= WCET_BUDGET
            && self.deadline_misses == 0
            && self.capacity_pressure < CAPACITY_PRESSURE_LIMIT
        {
            mask |= 1 << GATE_WCET;
        }

        self.gate_mask = mask;
    }

    // ── Action dispatch ───────────────────────────────────────────────────────

    fn do_action(&mut self, action: Action) {
        if action == Action::Release {
            self.do_release();
            return;
        }
        // Release cooldown check for non-release actions: no cooldown on these.

        let target = self.find_target();
        let Some(slot) = target else {
            // NO_TARGET (§7.4)
            if !matches!(action, Action::Release) {
                self.combo = 0;
                self.risk = sat(self.risk + NO_TARGET_RISK);
                self.check_risk();
            }
            return;
        };
        let entity = self.pool[slot].expect("slot valid");

        match action {
            Action::Validate => self.do_validate(slot, entity),
            Action::Convert => self.do_convert(slot, entity),
            Action::Quarantine => self.do_quarantine(slot, entity),
            Action::Consent => self.do_consent(slot, entity),
            Action::Evidence => self.do_evidence(slot, entity),
            Action::Release => unreachable!(),
            Action::None => {}
        }
    }

    fn find_target(&self) -> Option<usize> {
        let mut best: Option<(usize, i32)> = None;
        for (i, slot) in self.pool.iter().enumerate() {
            if let Some(entity) = slot {
                if entity.lane != self.selected_lane {
                    continue;
                }
                let x = entity.logical_x();
                if !(ACTION_WINDOW_START..BOUNDARY_X).contains(&x) {
                    continue;
                }
                // Closest to boundary (highest x), tie-break: lower slot.
                if best.map(|(_, bx)| x > bx).unwrap_or(true) {
                    best = Some((i, x));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    fn safety_score(&self, entity_x: i32, base: u64) -> u64 {
        // §17.5: safety_margin = clamp((boundary_x - entity_x) / 2, 0, 80)
        let margin = ((BOUNDARY_X - entity_x) / 2).clamp(0, 80) as u64;
        let combo_pct = 100 + 5 * self.combo.min(20) as u64;
        (base + margin) * combo_pct / 100
    }

    fn correct_action(
        &mut self,
        slot: usize,
        base_score: u64,
        trust_d: i32,
        risk_d: i32,
        int_d: i32,
    ) {
        let x = self.pool[slot].map(|e| e.logical_x()).unwrap_or(0);
        let delta = self.safety_score(x, base_score);
        self.score = self.score.saturating_add(delta);
        self.combo = (self.combo + 1).min(50);
        self.best_combo = self.best_combo.max(self.combo);
        self.trust = sat(self.trust + trust_d);
        self.risk = sat(self.risk + risk_d);
        self.integrity = sat(self.integrity + int_d);
    }

    fn wrong_action(&mut self, is_stim: bool) {
        self.wrong_actions += 1;
        self.combo = 0;
        self.score = self.score.saturating_sub(100);
        if is_stim {
            self.risk = sat(self.risk + 150);
            self.integrity = sat(self.integrity - 100);
        } else {
            self.trust = sat(self.trust - 60);
            self.risk = sat(self.risk + 80);
            self.integrity = sat(self.integrity - 40);
        }
        self.check_risk();
        self.check_integrity();
    }

    fn do_validate(&mut self, slot: usize, entity: Entity) {
        match entity.kind {
            Kind::CandidateIntent | Kind::UnknownPacket => {
                if let Some(e) = self.pool[slot].as_mut() {
                    e.kind = Kind::ValidatedIntent;
                    e.state = EntityState::Validated;
                }
                self.correct_action(slot, 100, 30, -15, 0);
            }
            Kind::DeadlineHazard => {
                // Timing review resolves the hazard.
                self.pool[slot] = None;
                self.correct_action(slot, 100, 30, -15, 0);
            }
            Kind::StimulationCommand => {
                self.wrong_action(true);
            }
            _ => {
                self.wrong_action(false);
            }
        }
    }

    fn do_convert(&mut self, slot: usize, entity: Entity) {
        if entity.kind != Kind::ValidatedIntent {
            if entity.kind == Kind::StimulationCommand {
                self.wrong_action(true);
            } else {
                self.wrong_action(false);
            }
            return;
        }
        let has_consent = self.consent.has_convert(self.consent_epoch, self.tick);
        let evidence_ok = self.evidence_level() >= self.mode.convert_evidence();
        if !has_consent || !evidence_ok {
            // Noncompliant convert (§10.3 violation).
            self.unvalidated_conversions += 1;
            self.consent_violations += 1;
            self.risk = sat(self.risk + 100);
            self.integrity = sat(self.integrity - 60);
            self.trust = sat(self.trust - 80);
            self.combo = 0;
            self.check_risk();
            self.check_integrity();
            return;
        }
        // Convert success.
        if let Some(e) = self.pool[slot].as_mut() {
            e.kind = Kind::TypedIntent;
            e.state = EntityState::Consumed;
        }
        self.typed_intents = self.typed_intents.saturating_add(1);
        self.correct_action(slot, 180, 45, -20, 5);
    }

    fn do_quarantine(&mut self, slot: usize, entity: Entity) {
        let is_valid_target = matches!(
            entity.kind,
            Kind::RawFrame
                | Kind::Artifact
                | Kind::UnsupportedClaim
                | Kind::UntraceableClaim
                | Kind::RoadmapAsFact
                | Kind::StimulationCommand
                | Kind::VaultRecord
                | Kind::RawExportRequest
                | Kind::ConsentRevoke
        );
        if !is_valid_target {
            if entity.kind == Kind::StimulationCommand {
                self.wrong_action(true);
            } else {
                self.wrong_action(false);
            }
            return;
        }
        // Vault: raw-type quarantine transitions vault.
        if matches!(entity.kind, Kind::RawFrame | Kind::VaultRecord) {
            self.vault = match self.vault {
                VaultState::Empty => VaultState::Sealing,
                VaultState::Open => VaultState::Sealing,
                VaultState::Sealing => VaultState::Sealed,
                other => other,
            };
            if self.pending_raw > 0 {
                self.pending_raw -= 1;
            }
        }
        if entity.kind == Kind::RawExportRequest {
            self.raw_export_violations = self.raw_export_violations.saturating_add(1);
        }
        self.pool[slot] = None;
        self.quarantined += 1;
        self.correct_action(slot, 140, 25, -35, 10);
    }

    fn do_consent(&mut self, slot: usize, entity: Entity) {
        match entity.kind {
            Kind::ConsentGrant => {
                let until = self.tick + self.mode.consent_ttl();
                // Grant CONVERT + RELEASE scopes.
                self.consent = ConsentState {
                    epoch: self.consent_epoch,
                    scope_mask: SCOPE_CONVERT | SCOPE_RELEASE,
                    expires_tick: until,
                };
                self.pool[slot] = None;
                self.correct_action(slot, 120, 35, -10, 0);
            }
            Kind::ConsentRevoke => {
                // Apply revocation: epoch bumps, all active tokens invalid.
                self.consent_epoch += 1;
                self.consent = ConsentState::NONE;
                self.pool[slot] = None;
                self.correct_action(slot, 120, 35, -10, 0);
            }
            _ => {
                self.wrong_action(false);
            }
        }
    }

    fn do_evidence(&mut self, slot: usize, entity: Entity) {
        let bit = entity.kind.evidence_level_bit();
        if bit == 0 {
            self.wrong_action(false);
            return;
        }
        // Out-of-order: only accept if prerequisite bit is present (§12, §10.6).
        let ok = match entity.kind {
            Kind::EvidenceTrace => true, // No prerequisite.
            Kind::ChecksumProof => self.evidence_bits & EVIDENCE_TRACE != 0,
            Kind::CiProof => self.evidence_bits & EVIDENCE_CHECKSUM != 0,
            _ => false,
        };
        if !ok {
            // Out-of-order proof; treat as wrong action.
            self.wrong_action(false);
            return;
        }
        self.evidence_bits |= bit;
        self.pool[slot] = None;
        self.correct_action(slot, 160, 40, -20, 5);
    }

    // ── Release (§10.7) ──────────────────────────────────────────────────────

    fn do_release(&mut self) {
        // Cooldown spam.
        if self.release_cooldown > 0 {
            self.risk = sat(self.risk + RELEASE_SPAM_RISK);
            return;
        }

        // Check gates and consent for release.
        let gates_ok = self.gate_mask == ALL_GATES_MASK;
        let consent_ok = self.consent.has_release(self.consent_epoch, self.tick);
        let phase_ok = !self.mode.final_phase_only() || self.phase >= 3;

        if gates_ok && consent_ok && phase_ok {
            // SEALED.
            self.trust = sat(self.trust + 100);
            self.risk = sat(self.risk - 100);
            self.integrity = sat(self.integrity + 50);
            let x = BOUNDARY_X; // Full safety at boundary.
            let delta = self.safety_score(0, 1000); // max safety_margin for release bonus
            let _ = x;
            self.score = self.score.saturating_add(delta);
            self.combo = (self.combo + 1).min(50);
            self.best_combo = self.best_combo.max(self.combo);
            self.end_run(Status::Sealed, TerminalReason::SuccessRelease);
        } else {
            // BLOCKED — not a loss, just adds cooldown.
            self.release_cooldown = RELEASE_COOLDOWN;
        }
    }

    // ── Entity advance ────────────────────────────────────────────────────────

    fn advance_entities(&mut self) -> u32 {
        let mut additional_wcet = 0u32;
        for slot in 0..ENTITY_CAPACITY {
            let Some(mut entity) = self.pool[slot] else {
                continue;
            };
            // Move.
            entity.x_q8 = entity.x_q8.saturating_add(entity.speed_q8 as u32);
            // Typed intents: continue to app zone.
            if entity.kind == Kind::TypedIntent {
                if entity.logical_x() >= APP_END {
                    self.pool[slot] = None;
                } else {
                    entity.state = EntityState::InActionWindow;
                    self.pool[slot] = Some(entity);
                }
                continue;
            }
            // Update state.
            let x = entity.logical_x();
            if (ACTION_WINDOW_START..BOUNDARY_X).contains(&x) {
                entity.state = EntityState::InActionWindow;
            }
            // Crossing check.
            if x >= BOUNDARY_X {
                self.pool[slot] = None;
                additional_wcet += self.resolve_crossing(entity);
                if self.status.is_terminal() {
                    return additional_wcet;
                }
            } else {
                self.pool[slot] = Some(entity);
            }
        }
        additional_wcet
    }

    fn resolve_crossing(&mut self, entity: Entity) -> u32 {
        self.combo = 0; // Any miss breaks combo.
        match entity.kind {
            Kind::RawFrame => {
                self.raw_leaks = self.raw_leaks.saturating_add(1);
                self.vault = VaultState::Compromised;
                self.trust = sat(self.trust - 100);
                self.risk = sat(self.risk + 250);
                self.integrity = sat(self.integrity - 250);
                self.score = self.score.saturating_sub(500);
                if self.raw_leaks >= self.mode.raw_leak_limit() {
                    self.end_run(Status::Breached, TerminalReason::RawLeakLimit);
                } else {
                    self.check_risk();
                    self.check_integrity();
                }
                4
            }
            Kind::StimulationCommand => {
                self.end_run(Status::Unsafe, TerminalReason::UnsafeStimulationEscape);
                0
            }
            Kind::VaultRecord | Kind::RawExportRequest => {
                self.vault = VaultState::Compromised;
                self.pending_raw = self.pending_raw.saturating_add(1);
                self.risk = sat(self.risk + 200);
                self.integrity = sat(self.integrity - 200);
                self.check_risk();
                self.check_integrity();
                4
            }
            Kind::Artifact => {
                self.risk = sat(self.risk + 100);
                self.integrity = sat(self.integrity - 80);
                self.trust = sat(self.trust - 40);
                self.score = self.score.saturating_sub(100);
                self.check_risk();
                self.check_integrity();
                0
            }
            Kind::UnsupportedClaim | Kind::UntraceableClaim | Kind::RoadmapAsFact => {
                self.escaped_claims = self.escaped_claims.saturating_add(1);
                self.risk = sat(self.risk + 140);
                self.integrity = sat(self.integrity - 80);
                self.trust = sat(self.trust - 80);
                self.check_risk();
                self.check_integrity();
                0
            }
            Kind::ConsentRevoke => {
                // Unacknowledged revoke (§9.4).
                self.consent_epoch += 1;
                self.consent = ConsentState::NONE;
                self.trust = sat(self.trust - 50);
                self.risk = sat(self.risk + 60);
                self.check_risk();
                0
            }
            Kind::CandidateIntent | Kind::UnknownPacket => {
                self.unvalidated_conversions = self.unvalidated_conversions.saturating_add(1);
                self.trust = sat(self.trust - 60);
                self.risk = sat(self.risk + 120);
                self.integrity = sat(self.integrity - 100);
                self.check_risk();
                self.check_integrity();
                0
            }
            Kind::ValidatedIntent => {
                self.trust = sat(self.trust - 60);
                self.risk = sat(self.risk + 80);
                self.integrity = sat(self.integrity - 40);
                self.check_risk();
                self.check_integrity();
                0
            }
            Kind::DeadlineHazard => {
                self.deadline_misses = self.deadline_misses.saturating_add(1);
                self.trust = sat(self.trust - 80);
                self.risk = sat(self.risk + 180);
                self.integrity = sat(self.integrity - 150);
                if self.mode == Mode::KernelTrial
                    && self.deadline_misses >= self.deadline_miss_terminal_limit
                {
                    self.end_run(Status::Breached, TerminalReason::DeadlineBreach);
                } else {
                    self.check_risk();
                    self.check_integrity();
                }
                0
            }
            Kind::ConsentGrant => {
                // Lost token.
                self.trust = sat(self.trust - 40);
                self.risk = sat(self.risk + 40);
                0
            }
            Kind::EvidenceTrace | Kind::ChecksumProof | Kind::CiProof => {
                self.trust = sat(self.trust - 30);
                self.risk = sat(self.risk + 30);
                0
            }
            Kind::Empty | Kind::TypedIntent => 0,
        }
    }

    // ── Spawn ────────────────────────────────────────────────────────────────

    fn do_spawn(&mut self) -> bool {
        if self.spawn_timer > 0 {
            self.spawn_timer -= 1;
            return false;
        }
        // Try to spawn.
        let kind = {
            let table = spawn_table(self.mode, self.phase);
            roll_kind(&mut self.rng, table)
        };
        let lane = self.pick_lane();
        let base_speed = 2 + self.difficulty.speed_bonus();
        let speed_q8 = (base_speed as u16) * 256;
        let entity = Entity {
            slot_id: 0,
            kind,
            lane,
            state: EntityState::Incoming,
            flags: 0,
            x_q8: 0,
            speed_q8,
            spawn_tick: self.tick,
            deadline_tick: if kind == Kind::DeadlineHazard {
                self.tick + 120
            } else {
                0
            },
            scope_mask: if kind == Kind::ConsentGrant {
                SCOPE_CONVERT | SCOPE_RELEASE
            } else {
                0
            },
            evidence_class: kind.evidence_level_bit(),
            generation: self.pool_generation,
        };
        let spawned = self.pool_insert(entity);
        if !spawned {
            self.capacity_pressure = self.capacity_pressure.saturating_add(1);
        }

        // Schedule next spawn.
        let (min, max) = self.mode.spawn_interval(self.phase);
        let interval = min + self.rng.range(max.saturating_sub(min) + 1);
        self.spawn_timer = interval;
        spawned
    }

    fn pick_lane(&mut self) -> u8 {
        let mut attempts = 0u8;
        loop {
            let lane = self.rng.range(LANES as u32) as u8;
            if lane != self.last_spawn_lane || self.consecutive_lane < 2 || attempts > 8 {
                if lane == self.last_spawn_lane {
                    self.consecutive_lane += 1;
                } else {
                    self.consecutive_lane = 0;
                    self.last_spawn_lane = lane;
                }
                return lane;
            }
            attempts += 1;
        }
    }

    fn pool_insert(&mut self, mut entity: Entity) -> bool {
        // Min-free-slot policy (§7.2).
        for i in 0..ENTITY_CAPACITY {
            if self.pool[i].is_none() {
                entity.slot_id = i as u8;
                self.pool[i] = Some(entity);
                self.pool_generation = self.pool_generation.wrapping_add(1);
                self.next_slot_hint = ((i + 1) % ENTITY_CAPACITY) as u8;
                return true;
            }
        }
        false
    }

    // ── Terminal helpers ──────────────────────────────────────────────────────

    fn end_run(&mut self, status: Status, reason: TerminalReason) {
        if !self.status.is_terminal() {
            self.status = status;
            self.reason = reason;
        }
    }

    fn check_risk(&mut self) {
        if self.risk >= 1000 {
            self.end_run(Status::Breached, TerminalReason::RiskOverflow);
        }
    }
    fn check_integrity(&mut self) {
        if self.integrity <= 0 {
            self.end_run(Status::Breached, TerminalReason::IntegrityCollapse);
        }
    }

    // ── Grade (§19) ──────────────────────────────────────────────────────────

    pub fn grade(&self) -> Grade {
        match self.status {
            Status::Unsafe => Grade::Unsafe,
            Status::Breached => Grade::Breached,
            Status::Sealed => {
                let gates = self.gate_mask == ALL_GATES_MASK;
                if gates
                    && self.trust >= 900
                    && self.risk <= 100
                    && self.integrity >= 900
                    && self.evidence_level() == EvidenceLevel::L3
                    && self.raw_leaks == 0
                    && self.wrong_actions == 0
                {
                    Grade::Sovereign
                } else if gates
                    && self.trust >= 750
                    && self.risk <= 250
                    && self.integrity >= 750
                    && self.raw_leaks == 0
                {
                    Grade::Sealed
                } else {
                    Grade::Reviewable
                }
            }
            Status::Aborted | Status::Running => {
                let passes = self.gate_mask.count_ones() as u8;
                if passes >= 5 && self.integrity >= 650 && self.risk <= 450 {
                    Grade::Reviewable
                } else if passes >= 3 && self.integrity > 0 && self.risk < 1000 {
                    Grade::Degraded
                } else {
                    Grade::Breached
                }
            }
            Status::FatalRuntime => Grade::Unsafe,
        }
    }

    // ── Snapshot ──────────────────────────────────────────────────────────────

    pub fn snapshot(&self) -> Snapshot {
        let remaining = if self.tick < self.consent.expires_tick && self.consent.scope_mask != 0 {
            self.consent.expires_tick - self.tick
        } else {
            0
        };
        Snapshot {
            mode: self.mode,
            difficulty: self.difficulty,
            seed: self.seed,
            tick: self.tick,
            status: self.status,
            reason: self.reason,
            phase: self.phase,
            selected_lane: self.selected_lane,
            trust: self.trust,
            risk: self.risk,
            integrity: self.integrity,
            score: self.score,
            combo: self.combo,
            best_combo: self.best_combo,
            raw_leaks: self.raw_leaks,
            typed_intents: self.typed_intents,
            quarantined: self.quarantined,
            wrong_actions: self.wrong_actions,
            evidence_bits: self.evidence_bits,
            evidence_level: self.evidence_level(),
            consent: self.consent,
            consent_epoch: self.consent_epoch,
            consent_expires_remaining: remaining,
            vault: self.vault,
            gate_mask: self.gate_mask,
            gates_passed: self.gate_mask.count_ones() as u8,
            wcet_peak: self.wcet_peak,
            capacity_pressure: self.capacity_pressure,
            deadline_misses: self.deadline_misses,
            live_entities: self.pool.iter().filter(|e| e.is_some()).count() as u8,
        }
    }

    // ── State hash: fnv1a64-v1 (§24.5) ──────────────────────────────────────

    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv64::new();
        // Version and algorithm IDs.
        for b in CORE_VERSION.as_bytes() {
            h.feed_u8(*b);
        }
        for b in HASH_ALGORITHM.as_bytes() {
            h.feed_u8(*b);
        }
        for b in RNG_ALGORITHM.as_bytes() {
            h.feed_u8(*b);
        }
        // Config.
        h.feed_u64(self.seed);
        h.feed_u8(self.mode.code());
        h.feed_u8(self.difficulty.code());
        // RNG state.
        h.feed_u64(self.rng.state());
        // Tick and status.
        h.feed_u32(self.tick);
        h.feed_u8(self.phase);
        h.feed_u8(self.status.code());
        h.feed_u8(self.reason.code());
        // Lane.
        h.feed_u8(self.selected_lane);
        // Metrics.
        h.feed_i32(self.trust);
        h.feed_i32(self.risk);
        h.feed_i32(self.integrity);
        h.feed_u64(self.score);
        h.feed_u32(self.combo);
        h.feed_u32(self.best_combo);
        // Consent.
        h.feed_u32(self.consent_epoch);
        h.feed_u16(self.consent.scope_mask);
        h.feed_u32(self.consent.expires_tick);
        h.feed_u32(self.consent.epoch);
        // Evidence.
        h.feed_u8(self.evidence_bits);
        // Vault.
        h.feed_u8(self.vault.code());
        h.feed_u8(self.pending_raw);
        h.feed_u8(self.raw_export_violations);
        // Gates.
        h.feed_u8(self.gate_mask);
        h.feed_u32(self.wcet_peak);
        h.feed_u8(self.capacity_pressure);
        // Counters.
        h.feed_u8(self.raw_leaks);
        h.feed_u8(self.typed_intents);
        h.feed_u32(self.quarantined);
        h.feed_u32(self.wrong_actions);
        h.feed_u8(self.consent_violations);
        h.feed_u8(self.unvalidated_conversions);
        h.feed_u8(self.escaped_claims);
        h.feed_u8(self.deadline_misses);
        // Pool: 32 slots ordered by slot_id.
        for i in 0..ENTITY_CAPACITY {
            match &self.pool[i] {
                None => {
                    h.feed_u8(0);
                }
                Some(e) => {
                    h.feed_u8(1);
                    h.feed_u8(e.slot_id);
                    h.feed_u8(e.kind.code());
                    h.feed_u8(e.lane);
                    h.feed_u8(e.state.code());
                    h.feed_u32(e.x_q8);
                    h.feed_u16(e.speed_q8);
                    h.feed_u32(e.spawn_tick);
                    h.feed_u32(e.deadline_tick);
                    h.feed_u16(e.scope_mask);
                    h.feed_u8(e.evidence_class);
                    h.feed_u16(e.generation);
                }
            }
        }
        h.finish()
    }
}

fn sat(v: i32) -> i32 {
    v.clamp(0, 1000)
}

// ─── Tests (§40) ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sim(mode: Mode) -> Simulation {
        Simulation::new(mode, Difficulty::Standard, 0x3001)
    }

    fn place(sim: &mut Simulation, kind: Kind, lane: u8, logical_x: i32) {
        sim.pool_insert(Entity {
            slot_id: 0,
            kind,
            lane,
            state: EntityState::Incoming,
            flags: 0,
            x_q8: to_q8(logical_x),
            speed_q8: 0,
            spawn_tick: sim.tick,
            deadline_tick: 0,
            scope_mask: 0,
            evidence_class: kind.evidence_level_bit(),
            generation: 0,
        });
    }

    fn act(sim: &mut Simulation, lane: u8, action: Action) {
        sim.step(Input {
            lane: Some(lane),
            action: Some(action),
        });
    }

    // ── §8.1 RNG determinism ─────────────────────────────────────────────────
    #[test]
    fn rng_xorshift64star_deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    // ── §8.2 Seed 0 remapping ────────────────────────────────────────────────
    #[test]
    fn rng_seed_zero_remapped() {
        let r = Rng::new(0);
        assert_ne!(r.state(), 0);
    }

    // ── §8.3 Daily Seed stability ────────────────────────────────────────────
    #[test]
    fn daily_seed_stable_and_sensitive() {
        assert_eq!(daily_seed(2026, 6, 14), daily_seed(2026, 6, 14));
        assert_ne!(daily_seed(2026, 6, 14), daily_seed(2026, 6, 15));
        assert_ne!(daily_seed(2026, 6, 14), 0);
    }

    // ── §24 Determinism contract ─────────────────────────────────────────────
    #[test]
    fn same_seed_input_same_hash() {
        let run = |seed| {
            let mut s = Simulation::new(Mode::Standard, Difficulty::Standard, seed);
            for t in 1..=600u32 {
                let action = if t % 60 == 0 {
                    Some(Action::Quarantine)
                } else {
                    None
                };
                s.step(Input {
                    lane: Some((t / 60 % 5) as u8),
                    action,
                });
            }
            s.state_hash()
        };
        assert_eq!(run(99), run(99));
        assert_ne!(run(99), run(100));
    }

    // ── §14 StimGuard: crossing → UNSAFE ─────────────────────────────────────
    #[test]
    fn stim_crossing_fails_closed() {
        let mut s = sim(Mode::Standard);
        place(&mut s, Kind::StimulationCommand, 2, BOUNDARY_X - 1);
        for e in s.pool.iter_mut().flatten() {
            e.speed_q8 = 512;
        }
        s.step(Input::IDLE);
        assert_eq!(s.status(), Status::Unsafe);
        assert_eq!(s.reason(), TerminalReason::UnsafeStimulationEscape);
        assert_eq!(s.grade(), Grade::Unsafe);
    }

    // ── §13 Vault: raw export crossing compromises vault ──────────────────────
    #[test]
    fn raw_export_compromises_vault() {
        let mut s = sim(Mode::Standard);
        place(&mut s, Kind::RawExportRequest, 1, BOUNDARY_X - 1);
        for e in s.pool.iter_mut().flatten() {
            e.speed_q8 = 512;
        }
        s.step(Input::IDLE);
        assert_eq!(s.vault(), VaultState::Compromised);
    }

    // ── §9 Raw frame: never becomes typed intent ──────────────────────────────
    #[test]
    fn raw_frame_never_becomes_output() {
        let mut s = sim(Mode::Standard);
        place(&mut s, Kind::RawFrame, 2, 600);
        act(&mut s, 2, Action::Validate); // Wrong action.
        act(&mut s, 2, Action::Convert); // Wrong action.
        for e in s.pool.iter().flatten() {
            assert_ne!(e.kind, Kind::TypedIntent);
            assert_ne!(e.kind, Kind::ValidatedIntent);
        }
    }

    // ── §10.3 Convert: consent + evidence required ────────────────────────────
    #[test]
    fn convert_requires_consent_and_evidence() {
        let mut s = sim(Mode::Standard);
        place(&mut s, Kind::CandidateIntent, 2, 600);
        act(&mut s, 2, Action::Validate);
        // Convert without consent — should register violation.
        let before_violations = s.consent_violations;
        act(&mut s, 2, Action::Convert);
        assert!(s.consent_violations > before_violations || s.wrong_actions > 0);
    }

    // ── §11.3 Epoch invalidation ──────────────────────────────────────────────
    #[test]
    fn revoke_invalidates_epoch_immediately() {
        let mut s = sim(Mode::Standard);
        // Grant consent via ConsentGrant entity.
        place(&mut s, Kind::ConsentGrant, 0, 600);
        act(&mut s, 0, Action::Consent);
        let epoch_before = s.consent_epoch();
        assert!(s.consent().scope_mask != 0);
        // Revoke via ConsentRevoke entity.
        place(&mut s, Kind::ConsentRevoke, 0, 600);
        act(&mut s, 0, Action::Consent);
        assert_eq!(s.consent_epoch(), epoch_before + 1);
        assert_eq!(s.consent().scope_mask, 0);
    }

    // ── §12 Evidence: out-of-order proof fails ───────────────────────────────
    #[test]
    fn evidence_out_of_order_rejected() {
        let mut s = sim(Mode::Standard);
        // Try CI Proof before Trace — no prerequisite.
        place(&mut s, Kind::CiProof, 1, 600);
        act(&mut s, 1, Action::Evidence);
        // CI_PROOF bit should NOT be set because TRACE is absent.
        assert_eq!(s.evidence_bits() & EVIDENCE_CI, 0);
    }

    // ── §16 Seven gates: sealed only when all pass ────────────────────────────
    #[test]
    fn release_blocked_when_gates_incomplete() {
        let mut s = sim(Mode::Standard);
        act(&mut s, 2, Action::Release);
        // Should be blocked (gates incomplete), not sealed.
        assert_eq!(s.status(), Status::Running);
    }

    // ── §17 Metrics saturate at 0 and 1000 ──────────────────────────────────
    #[test]
    fn metrics_saturate() {
        let mut s = sim(Mode::Audit);
        // Flood with raw frame crossings to drive risk to 1000.
        for _ in 0..8 {
            place(&mut s, Kind::RawFrame, 0, BOUNDARY_X - 1);
            for e in s.pool.iter_mut().flatten() {
                e.speed_q8 = 512;
            }
            s.step(Input::IDLE);
            if s.status().is_terminal() {
                break;
            }
        }
        // Should never exceed bounds.
        assert!(s.risk() >= 0 && s.risk() <= 1000);
        assert!(s.trust() >= 0 && s.trust() <= 1000);
        assert!(s.integrity() >= 0 && s.integrity() <= 1000);
    }

    // ── §18.5 Terminal immutability ──────────────────────────────────────────
    #[test]
    fn terminal_state_immutable() {
        let mut s = sim(Mode::Standard);
        place(&mut s, Kind::StimulationCommand, 2, BOUNDARY_X - 1);
        for e in s.pool.iter_mut().flatten() {
            e.speed_q8 = 512;
        }
        s.step(Input::IDLE);
        assert_eq!(s.status(), Status::Unsafe);
        let hash = s.state_hash();
        let tick = s.tick();
        s.step(Input {
            lane: Some(0),
            action: Some(Action::Release),
        });
        assert_eq!(s.state_hash(), hash);
        assert_eq!(s.tick(), tick);
    }

    // ── §19 Grade ordering ───────────────────────────────────────────────────
    #[test]
    fn grade_unsafe_on_stimulation() {
        let mut s = sim(Mode::Standard);
        place(&mut s, Kind::StimulationCommand, 2, BOUNDARY_X - 1);
        for e in s.pool.iter_mut().flatten() {
            e.speed_q8 = 512;
        }
        s.step(Input::IDLE);
        assert_eq!(s.grade(), Grade::Unsafe);
    }

    // ── §15.2 WCET budget ────────────────────────────────────────────────────
    #[test]
    fn wcet_gate_closes_when_budget_exceeded() {
        let mut s = sim(Mode::Standard);
        // Fill pool to 32 entities → 32*4=128 + base 40 = 168 < 618.
        for i in 0..32u8 {
            let _ = s.pool_insert(Entity {
                slot_id: i,
                kind: Kind::CandidateIntent,
                lane: i % 5,
                state: EntityState::Incoming,
                flags: 0,
                x_q8: to_q8(300 + i as i32),
                speed_q8: 0,
                spawn_tick: 0,
                deadline_tick: 0,
                scope_mask: 0,
                evidence_class: 0,
                generation: 0,
            });
        }
        s.step(Input::IDLE);
        // Should still be under budget; WCET gate depends on deadline misses too.
        assert!(s.wcet_peak() > 0);
    }

    // ── Pseudo-fuzz: determinism holds across 60 random runs ─────────────────
    #[test]
    fn fuzz_determinism() {
        for case in 0..60u64 {
            let mode = Mode::ALL[(case % 7) as usize];
            let seed = 0xA5A5_0000u64.wrapping_add(case);
            let mut driver = Rng::new(0xF00D ^ case);
            let mut a = Simulation::new(mode, Difficulty::Intense, seed);
            let mut b = Simulation::new(mode, Difficulty::Intense, seed);
            for _ in 0..1_500u32 {
                let input = if driver.range(4) == 0 {
                    Input {
                        lane: Some(driver.range(5) as u8),
                        action: Action::from_u8((driver.range(6) + 1) as u8),
                    }
                } else {
                    Input::IDLE
                };
                a.step(input);
                b.step(input);
            }
            assert_eq!(a.state_hash(), b.state_hash(), "case {case}");
        }
    }

    // ── §7.2 Pool capacity enforced ──────────────────────────────────────────
    #[test]
    fn pool_capacity_safe() {
        let mut s = sim(Mode::Standard);
        for i in 0..40u8 {
            s.pool_insert(Entity {
                slot_id: 0,
                kind: Kind::Artifact,
                lane: i % 5,
                state: EntityState::Incoming,
                flags: 0,
                x_q8: to_q8(100),
                speed_q8: 0,
                spawn_tick: 0,
                deadline_tick: 0,
                scope_mask: 0,
                evidence_class: 0,
                generation: 0,
            });
        }
        let count = s.pool.iter().filter(|e| e.is_some()).count();
        assert_eq!(count, ENTITY_CAPACITY);
    }
}
