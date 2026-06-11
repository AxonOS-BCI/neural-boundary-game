//! neural-boundary-core — deterministic `no_std` simulation core for the
//! Neural Boundary Game (v2.1.2, Foundation Grande AxonOS Standard Edition).
//!
//! The core owns game state, a deterministic RNG, the player action model,
//! the entity model, review gates, scoring, win/failure conditions and a
//! replay-compatible state progression. It contains no browser APIs, no heap
//! allocation, no rendering logic, no wall-clock randomness and no JavaScript
//! bindings.
//!
//! One rule is being demonstrated: raw signal stays inside the device;
//! applications receive typed intent only.

#![no_std]
#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Field geometry and tuning constants (fixed-point, integer-only)
// ---------------------------------------------------------------------------

/// Number of horizontal lanes in the playfield.
pub const LANES: u8 = 5;
/// Maximum simultaneously active entities (fixed slot pool, zero alloc).
pub const MAX_ENTITIES: usize = 24;
/// Maximum events recorded per tick.
pub const MAX_EVENTS: usize = 8;
/// Entities spawn at this x position (milli-field units).
pub const SPAWN_X: i32 = 0;
/// The boundary membrane. Hazards that reach it breach; typed intent passes.
pub const BOUNDARY_X: i32 = 680;
/// The application threshold. Typed intent delivered here scores trust.
pub const FIELD_END_X: i32 = 1000;
/// Player actions reach entities inside `[BOUNDARY_X - GATE_WINDOW, BOUNDARY_X)`.
pub const GATE_WINDOW: i32 = 320;
/// Ticks of cooldown after a successful player action.
pub const ACTION_COOLDOWN: u16 = 18;
/// Ticks of cooldown after an action that found no valid target.
pub const WHIFF_COOLDOWN: u16 = 6;
/// Ticks a consent grant stays active after a consent token is gated.
pub const CONSENT_DURATION: u32 = 1500;
/// Unsupported claims travel faster than evidence: extra speed for
/// `Overclaim`, `NoTrace` and `RoadmapFact`.
pub const CLAIM_SPEED_BONUS: i32 = 2;
/// Nominal simulation rate used by front-ends (the core itself is tick-based).
pub const TICKS_PER_SECOND: u32 = 60;
/// Number of review gates that must pass before release.
pub const GATE_COUNT: u8 = 5;

const ALL_GATES_MASK: u8 = (1 << GATE_COUNT) - 1;

// Starting metrics.
const START_TRUST: i32 = 50;
const START_RISK: i32 = 0;
const START_INTEGRITY: i32 = 100;

// Scoring deltas.
const TRUST_PER_DELIVERY: i32 = 6;
const TRUST_LOST_PER_MISS: i32 = 1;
const TRUST_LOST_FALSE_POSITIVE: i32 = 3;
const RISK_PER_REVEAL: i32 = 4;
const RISK_PER_MAJOR_BREACH: i32 = 6;
const RISK_PER_REJECTED_RELEASE: i32 = 6;
const INTEGRITY_LOST_MINOR: i32 = 6;
const INTEGRITY_LOST_MAJOR: i32 = 8;
const INTEGRITY_LOST_RAW_LEAK: i32 = 10;

// Win thresholds (see docs/GAME_SPEC.md).
pub const WIN_TRUST: u8 = 90;
pub const WIN_MAX_RISK: u8 = 20;
pub const WIN_INTEGRITY: u8 = 80;
pub const WIN_EVIDENCE_POINTS: u8 = 4; // == EvidenceLevel::L2
pub const LOSE_RAW_LEAKS: u8 = 3;

// Gate milestones.
const GATE_SCHEMA_VALIDATIONS: u8 = 3;
const GATE_CONTAINMENT_QUARANTINES: u8 = 3;
const GATE_DELIVERY_INTENTS: u8 = 5;

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*)
// ---------------------------------------------------------------------------

/// Deterministic xorshift64* generator. Identical sequences on every target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rng64 {
    state: u64,
}

impl Rng64 {
    pub const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xA50F_2112_D00D_FEED
            } else {
                seed
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn range(&mut self, max_exclusive: u32) -> u32 {
        if max_exclusive == 0 {
            0
        } else {
            (self.next_u64() % max_exclusive as u64) as u32
        }
    }

    pub const fn state(&self) -> u64 {
        self.state
    }
}

// ---------------------------------------------------------------------------
// FNV-1a 64 state hashing (replay conformance)
// ---------------------------------------------------------------------------

/// Incremental FNV-1a 64-bit hasher used for replay state checksums.
#[derive(Clone, Copy, Debug)]
pub struct Fnv64(u64);

impl Fnv64 {
    pub const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub fn write_u8(&mut self, value: u8) {
        self.0 ^= value as u64;
        self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
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

    pub const fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv64 {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Difficulty
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Difficulty {
    Calm,
    Standard,
    Intense,
}

struct SpawnParams {
    base_interval: u32,
    interval_jitter: u32,
    min_interval: u32,
    interval_shrink_every: u32,
    base_speed: i32,
    speed_jitter: u32,
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

    const fn params(self) -> SpawnParams {
        match self {
            Self::Calm => SpawnParams {
                base_interval: 68,
                interval_jitter: 20,
                min_interval: 44,
                interval_shrink_every: 700,
                base_speed: 2,
                speed_jitter: 1,
            },
            Self::Standard => SpawnParams {
                base_interval: 52,
                interval_jitter: 18,
                min_interval: 30,
                interval_shrink_every: 600,
                base_speed: 2,
                speed_jitter: 2,
            },
            Self::Intense => SpawnParams {
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

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityKind {
    // Good entities.
    IntentCandidate,
    ConsentToken,
    Evidence,
    Checksum,
    CiTest,
    TypedIntent,
    // Hazards.
    RawLeak,
    DirectStim,
    UnsafeBlock,
    Unbounded,
    Overclaim,
    NoTrace,
    RoadmapFact,
    /// Concealed traffic. Must be validated (classified) before it can be
    /// quarantined or converted.
    UnknownPacket,
}

impl EntityKind {
    /// Short, readable on-card label (<= 8 characters).
    pub const fn label(self) -> &'static str {
        match self {
            Self::IntentCandidate => "INTENT",
            Self::ConsentToken => "CONSENT",
            Self::Evidence => "EVIDENCE",
            Self::Checksum => "CHECKSUM",
            Self::CiTest => "CI TEST",
            Self::TypedIntent => "TYPED",
            Self::RawLeak => "RAW",
            Self::DirectStim => "STIM",
            Self::UnsafeBlock => "UNSAFE",
            Self::Unbounded => "UNBOUND",
            Self::Overclaim => "CLAIM",
            Self::NoTrace => "NO TRACE",
            Self::RoadmapFact => "ROADMAP",
            Self::UnknownPacket => "?PKT",
        }
    }

    pub const fn is_hazard(self) -> bool {
        matches!(
            self,
            Self::RawLeak
                | Self::DirectStim
                | Self::UnsafeBlock
                | Self::Unbounded
                | Self::Overclaim
                | Self::NoTrace
                | Self::RoadmapFact
        )
    }

    /// Evidence artifacts consumable through the evidence gate.
    pub const fn evidence_points(self) -> u8 {
        match self {
            Self::Evidence => 2,
            Self::Checksum | Self::CiTest => 1,
            _ => 0,
        }
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::IntentCandidate => 1,
            Self::ConsentToken => 2,
            Self::Evidence => 3,
            Self::Checksum => 4,
            Self::CiTest => 5,
            Self::TypedIntent => 6,
            Self::RawLeak => 7,
            Self::DirectStim => 8,
            Self::UnsafeBlock => 9,
            Self::Unbounded => 10,
            Self::Overclaim => 11,
            Self::NoTrace => 12,
            Self::RoadmapFact => 13,
            Self::UnknownPacket => 14,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entity {
    pub id: u16,
    pub kind: EntityKind,
    pub lane: u8,
    /// Position in milli-field units (`SPAWN_X..=FIELD_END_X`).
    pub x: i32,
    /// Milli-field units advanced per tick.
    pub speed: i32,
    /// Set on intent candidates after a successful Validate.
    pub validated: bool,
    /// For `UnknownPacket`: the concealed kind revealed by Validate.
    pub concealed: Option<EntityKind>,
}

// ---------------------------------------------------------------------------
// Player input
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
}

/// One tick of player input. Lane selection and an action may be combined.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Input {
    pub select_lane: Option<u8>,
    pub action: Option<Action>,
}

impl Input {
    pub const IDLE: Input = Input {
        select_lane: None,
        action: None,
    };
}

// ---------------------------------------------------------------------------
// Evidence levels, gates, status
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
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

/// The five review gates. All must pass before release.
pub const fn gate_name(index: u8) -> &'static str {
    match index {
        0 => "SCHEMA",
        1 => "CONSENT",
        2 => "EVIDENCE",
        3 => "CONTAIN",
        _ => "DELIVERY",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefeatCause {
    IntegrityCollapse,
    RiskOverflow,
    RawLeaks,
    DirectStim,
}

impl DefeatCause {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IntegrityCollapse => "integrity_collapse",
            Self::RiskOverflow => "risk_overflow",
            Self::RawLeaks => "raw_leaks",
            Self::DirectStim => "direct_stim",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Running,
    Victory,
    Defeat(DefeatCause),
}

impl Status {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Victory => "victory",
            Self::Defeat(_) => "defeat",
        }
    }

    /// SEALED on victory, BREACHED on defeat, HOLDING while running.
    pub const fn boundary(self) -> &'static str {
        match self {
            Self::Running => "HOLDING",
            Self::Victory => "SEALED",
            Self::Defeat(_) => "BREACHED",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseBlocker {
    RawLeaks,
    Gates,
    Evidence,
    Integrity,
    Risk,
    Trust,
}

impl ReleaseBlocker {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RawLeaks => "RAW LEAKS ON RECORD",
            Self::Gates => "REVIEW GATES OPEN",
            Self::Evidence => "EVIDENCE BELOW L2",
            Self::Integrity => "INTEGRITY BELOW 80",
            Self::Risk => "RISK ABOVE 20",
            Self::Trust => "TRUST BELOW 90",
        }
    }
}

// ---------------------------------------------------------------------------
// Events (per-tick, for front-end feedback; not part of the state hash)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Validated(EntityKind),
    Revealed(EntityKind),
    Converted,
    ConvertBlockedConsent,
    ConvertBlockedEvidence,
    QuarantineBlockedUnknown,
    Quarantined(EntityKind),
    FalsePositive(EntityKind),
    ConsentOn,
    ConsentExpired,
    EvidenceUp(EvidenceLevel, u8),
    GatePassed(u8),
    Delivered,
    MissedIntent,
    LostArtifact(EntityKind),
    MinorBreach(EntityKind),
    MajorBreach(EntityKind),
    RawLeakBreach,
    StimBreach,
    ReleaseRejected(ReleaseBlocker),
    ReleaseSealed,
    NoTarget(Action),
    Defeated(DefeatCause),
}

#[derive(Clone, Copy, Debug)]
pub struct EventBuf {
    items: [Option<Event>; MAX_EVENTS],
    len: u8,
}

impl EventBuf {
    pub const fn new() -> Self {
        Self {
            items: [None; MAX_EVENTS],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.items = [None; MAX_EVENTS];
        self.len = 0;
    }

    fn push(&mut self, event: Event) {
        if (self.len as usize) < MAX_EVENTS {
            self.items[self.len as usize] = Some(event);
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Event> + '_ {
        self.items.iter().take(self.len as usize).flatten().copied()
    }

    pub const fn len(&self) -> u8 {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for EventBuf {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Configuration and snapshot
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameConfig {
    pub seed: u64,
    pub difficulty: Difficulty,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            seed: 0x2112,
            difficulty: Difficulty::Standard,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub tick: u32,
    pub status: Status,
    pub selected_lane: u8,
    pub trust: u8,
    pub risk: u8,
    pub integrity: u8,
    pub evidence_points: u8,
    pub evidence_level: EvidenceLevel,
    pub consent_active: bool,
    pub consent_remaining: u32,
    pub gates_mask: u8,
    pub gates_passed: u8,
    pub raw_leaks: u8,
    pub delivered: u8,
    pub quarantined: u8,
    pub validated: u8,
    pub reveals: u8,
    pub cooldown: u16,
    pub active_entities: u8,
    pub last_release_blocker: Option<ReleaseBlocker>,
}

impl Snapshot {
    /// True when every release condition currently holds.
    pub fn release_ready(&self) -> bool {
        self.trust >= WIN_TRUST
            && self.risk <= WIN_MAX_RISK
            && self.integrity >= WIN_INTEGRITY
            && self.evidence_points >= WIN_EVIDENCE_POINTS
            && self.gates_mask == ALL_GATES_MASK
            && self.raw_leaks == 0
    }
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

pub struct GameState {
    config: GameConfig,
    rng: Rng64,
    tick: u32,
    status: Status,
    selected_lane: u8,
    trust: i32,
    risk: i32,
    integrity: i32,
    evidence_points: u8,
    consent_until: u32,
    gates: u8,
    raw_leaks: u8,
    validated_candidates: u8,
    quarantined_hazards: u8,
    delivered: u8,
    reveals: u8,
    misses: u8,
    false_positives: u8,
    minor_breaches: u8,
    major_breaches: u8,
    cooldown: u16,
    spawn_timer: u32,
    spawned: u16,
    next_id: u16,
    entities: [Option<Entity>; MAX_ENTITIES],
    events: EventBuf,
    last_release_blocker: Option<ReleaseBlocker>,
}

impl GameState {
    pub fn new(config: GameConfig) -> Self {
        Self {
            rng: Rng64::new(config.seed),
            config,
            tick: 0,
            status: Status::Running,
            selected_lane: 2,
            trust: START_TRUST,
            risk: START_RISK,
            integrity: START_INTEGRITY,
            evidence_points: 0,
            consent_until: 0,
            gates: 0,
            raw_leaks: 0,
            validated_candidates: 0,
            quarantined_hazards: 0,
            delivered: 0,
            reveals: 0,
            misses: 0,
            false_positives: 0,
            minor_breaches: 0,
            major_breaches: 0,
            cooldown: 0,
            spawn_timer: 40,
            spawned: 0,
            next_id: 1,
            entities: [None; MAX_ENTITIES],
            events: EventBuf::new(),
            last_release_blocker: None,
        }
    }

    pub const fn config(&self) -> GameConfig {
        self.config
    }

    pub const fn status(&self) -> Status {
        self.status
    }

    pub fn entities(&self) -> &[Option<Entity>; MAX_ENTITIES] {
        &self.entities
    }

    pub const fn events(&self) -> &EventBuf {
        &self.events
    }

    /// Advance the simulation by exactly one tick.
    pub fn step(&mut self, input: Input) {
        self.events.clear();

        if self.status != Status::Running {
            return;
        }

        self.tick = self.tick.wrapping_add(1);
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }

        if let Some(lane) = input.select_lane {
            self.selected_lane = if lane >= LANES { LANES - 1 } else { lane };
        }

        if let Some(action) = input.action {
            if self.cooldown == 0 {
                self.apply_action(action);
            }
        }

        if self.status == Status::Running {
            self.spawn();
            self.advance_entities();
            self.expire_consent();
            self.clamp_metrics();
            self.check_defeat();
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            status: self.status,
            selected_lane: self.selected_lane,
            trust: clamp_u8(self.trust),
            risk: clamp_u8(self.risk),
            integrity: clamp_u8(self.integrity),
            evidence_points: self.evidence_points,
            evidence_level: EvidenceLevel::from_points(self.evidence_points),
            consent_active: self.consent_active(),
            consent_remaining: self.consent_until.saturating_sub(self.tick),
            gates_mask: self.gates,
            gates_passed: self.gates.count_ones() as u8,
            raw_leaks: self.raw_leaks,
            delivered: self.delivered,
            quarantined: self.quarantined_hazards,
            validated: self.validated_candidates,
            reveals: self.reveals,
            cooldown: self.cooldown,
            active_entities: self.entities.iter().filter(|slot| slot.is_some()).count() as u8,
            last_release_blocker: self.last_release_blocker,
        }
    }

    /// FNV-1a 64 hash over the complete deterministic state. Two replays of
    /// the same seed, difficulty and input script always produce the same
    /// hash at the same tick, on every platform.
    pub fn state_hash(&self) -> u64 {
        let mut hash = Fnv64::new();
        hash.write_u64(self.config.seed);
        hash.write_u8(match self.config.difficulty {
            Difficulty::Calm => 0,
            Difficulty::Standard => 1,
            Difficulty::Intense => 2,
        });
        hash.write_u64(self.rng.state());
        hash.write_u32(self.tick);
        hash.write_u8(match self.status {
            Status::Running => 0,
            Status::Victory => 1,
            Status::Defeat(cause) => 0x10 | cause as u8,
        });
        hash.write_u8(self.selected_lane);
        hash.write_i32(self.trust);
        hash.write_i32(self.risk);
        hash.write_i32(self.integrity);
        hash.write_u8(self.evidence_points);
        hash.write_u32(self.consent_until);
        hash.write_u8(self.gates);
        hash.write_u8(self.raw_leaks);
        hash.write_u8(self.validated_candidates);
        hash.write_u8(self.quarantined_hazards);
        hash.write_u8(self.delivered);
        hash.write_u8(self.reveals);
        hash.write_u8(self.misses);
        hash.write_u8(self.false_positives);
        hash.write_u8(self.minor_breaches);
        hash.write_u8(self.major_breaches);
        hash.write_u8(self.cooldown as u8);
        hash.write_u32(self.spawn_timer);
        hash.write_u32(self.spawned as u32);
        hash.write_u32(self.next_id as u32);
        for slot in &self.entities {
            match slot {
                None => hash.write_u8(0),
                Some(entity) => {
                    hash.write_u8(1);
                    hash.write_u32(entity.id as u32);
                    hash.write_u8(entity.kind.code());
                    hash.write_u8(entity.lane);
                    hash.write_i32(entity.x);
                    hash.write_i32(entity.speed);
                    hash.write_u8(entity.validated as u8);
                    hash.write_u8(entity.concealed.map_or(0, EntityKind::code));
                }
            }
        }
        hash.finish()
    }

    const fn consent_active(&self) -> bool {
        self.consent_until > self.tick
    }

    // -- actions ------------------------------------------------------------

    fn apply_action(&mut self, action: Action) {
        if action == Action::Release {
            self.try_release();
            self.cooldown = ACTION_COOLDOWN;
            return;
        }

        let Some(index) = self.frontmost_in_window(self.selected_lane) else {
            self.events.push(Event::NoTarget(action));
            self.cooldown = WHIFF_COOLDOWN;
            return;
        };

        let entity = self.entities[index].expect("slot checked above");
        let mut consumed_cooldown = ACTION_COOLDOWN;

        match action {
            Action::Validate => match entity.kind {
                EntityKind::IntentCandidate if !entity.validated => {
                    if let Some(slot) = self.entities[index].as_mut() {
                        slot.validated = true;
                    }
                    self.validated_candidates = self.validated_candidates.saturating_add(1);
                    self.events.push(Event::Validated(entity.kind));
                    if self.validated_candidates >= GATE_SCHEMA_VALIDATIONS {
                        self.pass_gate(0);
                    }
                }
                EntityKind::UnknownPacket => {
                    let revealed = entity.concealed.unwrap_or(EntityKind::Overclaim);
                    if let Some(slot) = self.entities[index].as_mut() {
                        slot.kind = revealed;
                        slot.concealed = None;
                    }
                    self.reveals = self.reveals.saturating_add(1);
                    self.risk += RISK_PER_REVEAL;
                    self.events.push(Event::Revealed(revealed));
                }
                _ => {
                    self.events.push(Event::NoTarget(action));
                    consumed_cooldown = WHIFF_COOLDOWN;
                }
            },
            Action::Convert => match entity.kind {
                EntityKind::IntentCandidate if entity.validated => {
                    if !self.consent_active() {
                        self.events.push(Event::ConvertBlockedConsent);
                        consumed_cooldown = WHIFF_COOLDOWN;
                    } else if EvidenceLevel::from_points(self.evidence_points) < EvidenceLevel::L1 {
                        self.events.push(Event::ConvertBlockedEvidence);
                        consumed_cooldown = WHIFF_COOLDOWN;
                    } else {
                        if let Some(slot) = self.entities[index].as_mut() {
                            slot.kind = EntityKind::TypedIntent;
                        }
                        self.events.push(Event::Converted);
                    }
                }
                _ => {
                    self.events.push(Event::NoTarget(action));
                    consumed_cooldown = WHIFF_COOLDOWN;
                }
            },
            Action::Quarantine => {
                if entity.kind == EntityKind::UnknownPacket {
                    self.events.push(Event::QuarantineBlockedUnknown);
                    consumed_cooldown = WHIFF_COOLDOWN;
                } else if entity.kind.is_hazard() {
                    self.entities[index] = None;
                    self.quarantined_hazards = self.quarantined_hazards.saturating_add(1);
                    self.events.push(Event::Quarantined(entity.kind));
                    if self.quarantined_hazards >= GATE_CONTAINMENT_QUARANTINES {
                        self.pass_gate(3);
                    }
                } else {
                    self.entities[index] = None;
                    self.false_positives = self.false_positives.saturating_add(1);
                    self.trust -= TRUST_LOST_FALSE_POSITIVE;
                    self.events.push(Event::FalsePositive(entity.kind));
                }
            }
            Action::ConsentGate => {
                if entity.kind == EntityKind::ConsentToken {
                    self.entities[index] = None;
                    self.consent_until = self.tick + CONSENT_DURATION;
                    self.events.push(Event::ConsentOn);
                    self.pass_gate(1);
                } else {
                    self.events.push(Event::NoTarget(action));
                    consumed_cooldown = WHIFF_COOLDOWN;
                }
            }
            Action::EvidenceGate => {
                let points = entity.kind.evidence_points();
                if points > 0 {
                    self.entities[index] = None;
                    self.evidence_points = self.evidence_points.saturating_add(points);
                    let level = EvidenceLevel::from_points(self.evidence_points);
                    self.events
                        .push(Event::EvidenceUp(level, self.evidence_points));
                    if level >= EvidenceLevel::L2 {
                        self.pass_gate(2);
                    }
                } else {
                    self.events.push(Event::NoTarget(action));
                    consumed_cooldown = WHIFF_COOLDOWN;
                }
            }
            Action::Release => unreachable!("handled above"),
        }

        self.cooldown = consumed_cooldown;
    }

    fn try_release(&mut self) {
        let snapshot = self.snapshot();
        let blocker = if snapshot.raw_leaks > 0 {
            Some(ReleaseBlocker::RawLeaks)
        } else if snapshot.gates_mask != ALL_GATES_MASK {
            Some(ReleaseBlocker::Gates)
        } else if snapshot.evidence_points < WIN_EVIDENCE_POINTS {
            Some(ReleaseBlocker::Evidence)
        } else if snapshot.integrity < WIN_INTEGRITY {
            Some(ReleaseBlocker::Integrity)
        } else if snapshot.risk > WIN_MAX_RISK {
            Some(ReleaseBlocker::Risk)
        } else if snapshot.trust < WIN_TRUST {
            Some(ReleaseBlocker::Trust)
        } else {
            None
        };

        match blocker {
            None => {
                self.status = Status::Victory;
                self.events.push(Event::ReleaseSealed);
            }
            Some(blocker) => {
                self.risk += RISK_PER_REJECTED_RELEASE;
                self.last_release_blocker = Some(blocker);
                self.events.push(Event::ReleaseRejected(blocker));
            }
        }
    }

    fn pass_gate(&mut self, index: u8) {
        let bit = 1u8 << index;
        if self.gates & bit == 0 {
            self.gates |= bit;
            self.events.push(Event::GatePassed(index));
        }
    }

    fn frontmost_in_window(&self, lane: u8) -> Option<usize> {
        let mut best: Option<(usize, i32)> = None;
        for (index, slot) in self.entities.iter().enumerate() {
            if let Some(entity) = slot {
                if entity.lane == lane
                    && entity.x >= BOUNDARY_X - GATE_WINDOW
                    && entity.x < BOUNDARY_X
                {
                    match best {
                        Some((_, x)) if entity.x <= x => {}
                        _ => best = Some((index, entity.x)),
                    }
                }
            }
        }
        best.map(|(index, _)| index)
    }

    // -- world --------------------------------------------------------------

    fn spawn(&mut self) {
        if self.spawn_timer > 0 {
            self.spawn_timer -= 1;
            return;
        }

        let params = self.config.difficulty.params();

        // Deterministic warm-up wave teaching the core loop, then weighted RNG.
        let (kind, lane, speed, concealed) = if (self.spawned as usize) < WARMUP.len() {
            let (kind, lane) = WARMUP[self.spawned as usize];
            let concealed = if kind == EntityKind::UnknownPacket {
                Some(EntityKind::RawLeak)
            } else {
                None
            };
            (kind, lane, params.base_speed, concealed)
        } else {
            let kind = self.roll_kind();
            let lane = self.rng.range(LANES as u32) as u8;
            let mut speed = params.base_speed + self.rng.range(params.speed_jitter + 1) as i32;
            if matches!(
                kind,
                EntityKind::Overclaim | EntityKind::NoTrace | EntityKind::RoadmapFact
            ) {
                speed += CLAIM_SPEED_BONUS;
            }
            let concealed = if kind == EntityKind::UnknownPacket {
                Some(self.roll_concealed())
            } else {
                None
            };
            (kind, lane, speed, concealed)
        };

        let entity = Entity {
            id: self.next_id,
            kind,
            lane,
            x: SPAWN_X,
            speed,
            validated: false,
            concealed,
        };
        self.next_id = self.next_id.wrapping_add(1);
        self.spawned = self.spawned.saturating_add(1);

        for slot in self.entities.iter_mut() {
            if slot.is_none() {
                *slot = Some(entity);
                break;
            }
        }

        // Schedule the next spawn.
        let shrink = (self.tick / params.interval_shrink_every) * 3;
        let base = params
            .base_interval
            .saturating_sub(shrink)
            .max(params.min_interval);
        let jitter = self.rng.range(params.interval_jitter * 2 + 1);
        self.spawn_timer = base + jitter - params.interval_jitter.min(base + jitter);
    }

    fn roll_kind(&mut self) -> EntityKind {
        // Weighted table (per mille).
        const TABLE: [(EntityKind, u32); 13] = [
            (EntityKind::IntentCandidate, 230),
            (EntityKind::ConsentToken, 90),
            (EntityKind::Evidence, 80),
            (EntityKind::Checksum, 60),
            (EntityKind::CiTest, 60),
            (EntityKind::RawLeak, 90),
            (EntityKind::DirectStim, 40),
            (EntityKind::UnsafeBlock, 60),
            (EntityKind::Unbounded, 50),
            (EntityKind::Overclaim, 80),
            (EntityKind::NoTrace, 50),
            (EntityKind::RoadmapFact, 50),
            (EntityKind::UnknownPacket, 60),
        ];
        let total: u32 = 1000;
        let mut roll = self.rng.range(total);
        for (kind, weight) in TABLE {
            if roll < weight {
                return kind;
            }
            roll -= weight;
        }
        EntityKind::IntentCandidate
    }

    fn roll_concealed(&mut self) -> EntityKind {
        const TABLE: [(EntityKind, u32); 5] = [
            (EntityKind::RawLeak, 25),
            (EntityKind::Overclaim, 20),
            (EntityKind::NoTrace, 15),
            (EntityKind::IntentCandidate, 25),
            (EntityKind::UnsafeBlock, 15),
        ];
        let mut roll = self.rng.range(100);
        for (kind, weight) in TABLE {
            if roll < weight {
                return kind;
            }
            roll -= weight;
        }
        EntityKind::Overclaim
    }

    fn advance_entities(&mut self) {
        for index in 0..MAX_ENTITIES {
            let Some(mut entity) = self.entities[index] else {
                continue;
            };
            let before = entity.x;
            entity.x += entity.speed;

            if before < BOUNDARY_X && entity.x >= BOUNDARY_X {
                // The membrane.
                let kind = if entity.kind == EntityKind::UnknownPacket {
                    entity.concealed.unwrap_or(EntityKind::Overclaim)
                } else {
                    entity.kind
                };

                if kind == EntityKind::TypedIntent {
                    // Typed intent passes the boundary by design.
                    entity.kind = EntityKind::TypedIntent;
                    self.entities[index] = Some(entity);
                    continue;
                }

                self.entities[index] = None;
                match kind {
                    EntityKind::RawLeak => {
                        self.raw_leaks = self.raw_leaks.saturating_add(1);
                        self.integrity -= INTEGRITY_LOST_RAW_LEAK;
                        self.events.push(Event::RawLeakBreach);
                    }
                    EntityKind::DirectStim => {
                        self.status = Status::Defeat(DefeatCause::DirectStim);
                        self.events.push(Event::StimBreach);
                        self.events.push(Event::Defeated(DefeatCause::DirectStim));
                        return;
                    }
                    EntityKind::UnsafeBlock | EntityKind::Unbounded => {
                        self.integrity -= INTEGRITY_LOST_MAJOR;
                        self.risk += RISK_PER_MAJOR_BREACH;
                        self.major_breaches = self.major_breaches.saturating_add(1);
                        self.events.push(Event::MajorBreach(kind));
                    }
                    EntityKind::Overclaim | EntityKind::NoTrace | EntityKind::RoadmapFact => {
                        self.integrity -= INTEGRITY_LOST_MINOR;
                        self.minor_breaches = self.minor_breaches.saturating_add(1);
                        self.events.push(Event::MinorBreach(kind));
                    }
                    EntityKind::IntentCandidate => {
                        self.trust -= TRUST_LOST_PER_MISS;
                        self.misses = self.misses.saturating_add(1);
                        self.events.push(Event::MissedIntent);
                    }
                    EntityKind::ConsentToken
                    | EntityKind::Evidence
                    | EntityKind::Checksum
                    | EntityKind::CiTest => {
                        self.events.push(Event::LostArtifact(kind));
                    }
                    EntityKind::TypedIntent | EntityKind::UnknownPacket => {}
                }
                continue;
            }

            if entity.x >= FIELD_END_X {
                self.entities[index] = None;
                if entity.kind == EntityKind::TypedIntent {
                    self.delivered = self.delivered.saturating_add(1);
                    self.trust += TRUST_PER_DELIVERY;
                    self.events.push(Event::Delivered);
                    if self.delivered >= GATE_DELIVERY_INTENTS {
                        self.pass_gate(4);
                    }
                }
                continue;
            }

            self.entities[index] = Some(entity);
        }
    }

    fn expire_consent(&mut self) {
        if self.consent_until != 0 && self.consent_until == self.tick {
            self.events.push(Event::ConsentExpired);
        }
    }

    fn clamp_metrics(&mut self) {
        self.trust = self.trust.clamp(0, 100);
        self.risk = self.risk.clamp(0, 100);
        self.integrity = self.integrity.clamp(0, 100);
    }

    fn check_defeat(&mut self) {
        let cause = if self.integrity <= 0 {
            Some(DefeatCause::IntegrityCollapse)
        } else if self.risk >= 100 {
            Some(DefeatCause::RiskOverflow)
        } else if self.raw_leaks >= LOSE_RAW_LEAKS {
            Some(DefeatCause::RawLeaks)
        } else {
            None
        };
        if let Some(cause) = cause {
            self.status = Status::Defeat(cause);
            self.events.push(Event::Defeated(cause));
        }
    }

    #[cfg(test)]
    fn inject(&mut self, kind: EntityKind, lane: u8, x: i32, speed: i32) {
        for slot in self.entities.iter_mut() {
            if slot.is_none() {
                *slot = Some(Entity {
                    id: self.next_id,
                    kind,
                    lane,
                    x,
                    speed,
                    validated: false,
                    concealed: None,
                });
                self.next_id = self.next_id.wrapping_add(1);
                return;
            }
        }
    }
}

const WARMUP: [(EntityKind, u8); 6] = [
    (EntityKind::ConsentToken, 2),
    (EntityKind::IntentCandidate, 2),
    (EntityKind::Evidence, 1),
    (EntityKind::IntentCandidate, 3),
    (EntityKind::Checksum, 1),
    (EntityKind::UnknownPacket, 0),
];

const fn clamp_u8(value: i32) -> u8 {
    if value < 0 {
        0
    } else if value > 100 {
        100
    } else {
        value as u8
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn act(state: &mut GameState, lane: u8, action: Action) {
        state.step(Input {
            select_lane: Some(lane),
            action: Some(action),
        });
    }

    fn idle(state: &mut GameState, ticks: u32) {
        for _ in 0..ticks {
            state.step(Input::IDLE);
        }
    }

    fn isolate(state: &mut GameState) {
        // Hermetic tests: suspend natural spawning.
        state.spawn_timer = u32::MAX;
    }

    fn cool(state: &mut GameState) {
        while state.snapshot().cooldown > 0 {
            state.step(Input::IDLE);
        }
    }

    #[test]
    fn rng_sequence_is_locked() {
        // Lock the algorithm: xorshift64* with the documented multiplier.
        let mut reference = 42u64;
        reference ^= reference << 13;
        reference ^= reference >> 7;
        reference ^= reference << 17;
        let expected = reference.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let mut rng = Rng64::new(42);
        assert_eq!(rng.next_u64(), expected);
    }

    #[test]
    fn deterministic_replay_hashes_match() {
        let config = GameConfig {
            seed: 777,
            difficulty: Difficulty::Standard,
        };
        let mut a = GameState::new(config);
        let mut b = GameState::new(config);
        for tick in 1..=900u32 {
            let input = if tick % 37 == 0 {
                Input {
                    select_lane: Some((tick % LANES as u32) as u8),
                    action: Some(Action::Validate),
                }
            } else if tick % 191 == 0 {
                Input {
                    select_lane: None,
                    action: Some(Action::Quarantine),
                }
            } else {
                Input::IDLE
            };
            a.step(input);
            b.step(input);
            assert_eq!(a.state_hash(), b.state_hash(), "hash diverged at {tick}");
        }
        assert_eq!(a.snapshot(), b.snapshot());
    }

    #[test]
    fn warmup_wave_is_fixed() {
        let mut state = GameState::new(GameConfig::default());
        idle(&mut state, 60);
        let first = state
            .entities()
            .iter()
            .flatten()
            .min_by_key(|entity| entity.id)
            .copied()
            .expect("first warm-up entity spawned");
        assert_eq!(first.kind, EntityKind::ConsentToken);
        assert_eq!(first.lane, 2);
    }

    #[test]
    fn validate_then_convert_requires_consent_and_evidence() {
        let mut state = GameState::new(GameConfig {
            seed: 9,
            difficulty: Difficulty::Standard,
        });
        isolate(&mut state);
        state.inject(EntityKind::IntentCandidate, 1, BOUNDARY_X - 200, 0);
        act(&mut state, 1, Action::Validate);
        assert!(state
            .events()
            .iter()
            .any(|e| matches!(e, Event::Validated(_))));
        cool(&mut state);

        act(&mut state, 1, Action::Convert);
        assert!(state
            .events()
            .iter()
            .any(|e| e == Event::ConvertBlockedConsent));
        cool(&mut state);

        state.inject(EntityKind::ConsentToken, 0, BOUNDARY_X - 150, 0);
        act(&mut state, 0, Action::ConsentGate);
        assert!(state.snapshot().consent_active);
        cool(&mut state);

        act(&mut state, 1, Action::Convert);
        assert!(state
            .events()
            .iter()
            .any(|e| e == Event::ConvertBlockedEvidence));
        cool(&mut state);

        state.inject(EntityKind::Evidence, 0, BOUNDARY_X - 150, 0);
        act(&mut state, 0, Action::EvidenceGate);
        cool(&mut state);

        act(&mut state, 1, Action::Convert);
        assert!(state.events().iter().any(|e| e == Event::Converted));
    }

    #[test]
    fn unknown_packet_must_be_classified_first() {
        let mut state = GameState::new(GameConfig::default());
        isolate(&mut state);
        state.inject(EntityKind::UnknownPacket, 2, BOUNDARY_X - 100, 0);
        if let Some(slot) = state.entities[0].as_mut() {
            slot.concealed = Some(EntityKind::RawLeak);
        }

        act(&mut state, 2, Action::Quarantine);
        assert!(state
            .events()
            .iter()
            .any(|e| e == Event::QuarantineBlockedUnknown));
        cool(&mut state);

        let risk_before = state.snapshot().risk;
        act(&mut state, 2, Action::Validate);
        assert!(state
            .events()
            .iter()
            .any(|e| e == Event::Revealed(EntityKind::RawLeak)));
        assert_eq!(state.snapshot().risk, risk_before + RISK_PER_REVEAL as u8);
        cool(&mut state);

        act(&mut state, 2, Action::Quarantine);
        assert!(state
            .events()
            .iter()
            .any(|e| matches!(e, Event::Quarantined(EntityKind::RawLeak))));
    }

    #[test]
    fn direct_stim_crossing_is_an_instant_breach() {
        let mut state = GameState::new(GameConfig::default());
        state.inject(EntityKind::DirectStim, 3, BOUNDARY_X - 1, 4);
        state.step(Input::IDLE);
        assert_eq!(state.status(), Status::Defeat(DefeatCause::DirectStim));
        assert_eq!(state.status().boundary(), "BREACHED");
    }

    #[test]
    fn three_raw_leaks_end_the_run() {
        let mut state = GameState::new(GameConfig::default());
        isolate(&mut state);
        for _ in 0..3 {
            state.inject(EntityKind::RawLeak, 4, BOUNDARY_X - 1, 4);
            state.step(Input::IDLE);
        }
        assert_eq!(state.status(), Status::Defeat(DefeatCause::RawLeaks));
        assert_eq!(state.snapshot().raw_leaks, 3);
    }

    #[test]
    fn premature_release_is_rejected_and_adds_risk() {
        let mut state = GameState::new(GameConfig::default());
        let risk_before = state.snapshot().risk;
        act(&mut state, 2, Action::Release);
        let snapshot = state.snapshot();
        assert_eq!(snapshot.status, Status::Running);
        assert_eq!(snapshot.risk, risk_before + RISK_PER_REJECTED_RELEASE as u8);
        assert!(state
            .events()
            .iter()
            .any(|e| matches!(e, Event::ReleaseRejected(_))));
    }

    #[test]
    fn full_clean_run_reaches_victory_and_seals_the_boundary() {
        let mut state = GameState::new(GameConfig {
            seed: 31,
            difficulty: Difficulty::Calm,
        });
        isolate(&mut state);

        // Consent.
        state.inject(EntityKind::ConsentToken, 0, BOUNDARY_X - 60, 0);
        act(&mut state, 0, Action::ConsentGate);
        cool(&mut state);

        // Evidence to L2 (2 + 2 points).
        for _ in 0..2 {
            state.inject(EntityKind::Evidence, 0, BOUNDARY_X - 60, 0);
            act(&mut state, 0, Action::EvidenceGate);
            cool(&mut state);
        }
        assert!(state.snapshot().evidence_level >= EvidenceLevel::L2);

        // Quarantine three hazards.
        for _ in 0..3 {
            state.inject(EntityKind::Overclaim, 1, BOUNDARY_X - 60, 0);
            act(&mut state, 1, Action::Quarantine);
            cool(&mut state);
        }

        // Validate + convert + deliver seven typed intents (trust 50 -> 92).
        for _ in 0..7 {
            state.inject(EntityKind::IntentCandidate, 2, BOUNDARY_X - 120, 0);
            act(&mut state, 2, Action::Validate);
            cool(&mut state);
            act(&mut state, 2, Action::Convert);
            cool(&mut state);
            if let Some(slot) = state
                .entities
                .iter_mut()
                .flatten()
                .find(|entity| entity.kind == EntityKind::TypedIntent)
            {
                slot.speed = 40;
            }
            idle(&mut state, 40);
        }

        let snapshot = state.snapshot();
        assert_eq!(snapshot.trust, 92, "trust after seven deliveries");
        assert_eq!(snapshot.gates_passed, GATE_COUNT);
        assert!(snapshot.release_ready(), "blocker: {:?}", snapshot);

        act(&mut state, 2, Action::Release);
        assert_eq!(state.status(), Status::Victory);
        assert_eq!(state.status().boundary(), "SEALED");
    }

    #[test]
    fn cooldown_blocks_rapid_actions() {
        let mut state = GameState::new(GameConfig::default());
        isolate(&mut state);
        state.inject(EntityKind::Overclaim, 2, BOUNDARY_X - 60, 0);
        state.inject(EntityKind::Overclaim, 2, BOUNDARY_X - 90, 0);
        act(&mut state, 2, Action::Quarantine);
        assert_eq!(state.snapshot().quarantined, 1);
        act(&mut state, 2, Action::Quarantine);
        assert_eq!(state.snapshot().quarantined, 1, "cooldown must gate input");
        cool(&mut state);
        act(&mut state, 2, Action::Quarantine);
        assert_eq!(state.snapshot().quarantined, 2);
    }

    #[test]
    fn metric_clamps_hold() {
        let mut state = GameState::new(GameConfig::default());
        state.trust = 250;
        state.risk = -40;
        state.integrity = 180;
        state.clamp_metrics();
        let snapshot = state.snapshot();
        assert_eq!(snapshot.trust, 100);
        assert_eq!(snapshot.risk, 0);
        assert_eq!(snapshot.integrity, 100);
    }

    #[test]
    fn names_round_trip() {
        for action in [
            Action::Validate,
            Action::Convert,
            Action::Quarantine,
            Action::ConsentGate,
            Action::EvidenceGate,
            Action::Release,
        ] {
            assert_eq!(Action::from_name(action.name()), Some(action));
        }
        for difficulty in [Difficulty::Calm, Difficulty::Standard, Difficulty::Intense] {
            assert_eq!(Difficulty::from_name(difficulty.name()), Some(difficulty));
        }
        assert_eq!(Action::from_name("Yolo"), None);
    }

    #[test]
    fn labels_stay_short_for_cards() {
        let kinds = [
            EntityKind::IntentCandidate,
            EntityKind::ConsentToken,
            EntityKind::Evidence,
            EntityKind::Checksum,
            EntityKind::CiTest,
            EntityKind::TypedIntent,
            EntityKind::RawLeak,
            EntityKind::DirectStim,
            EntityKind::UnsafeBlock,
            EntityKind::Unbounded,
            EntityKind::Overclaim,
            EntityKind::NoTrace,
            EntityKind::RoadmapFact,
            EntityKind::UnknownPacket,
        ];
        for kind in kinds {
            assert!(kind.label().len() <= 8, "{} label too long", kind.label());
        }
    }
}
