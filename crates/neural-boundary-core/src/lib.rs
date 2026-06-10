#![no_std]
#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerAction {
    Idle,
    MoveUp,
    MoveDown,
    Validate,
    Convert,
    Quarantine,
    ConsentGate,
    EvidenceGate,
    Release,
    ToggleHelp,
    Restart,
    Pause,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityKind {
    IntentCandidate,
    ConsentToken,
    Evidence,
    Checksum,
    CiTest,
    TypedIntent,
    RawLeak,
    DirectStim,
    UnsafeBlock,
    Unbounded,
    Overclaim,
    NoTrace,
    RoadmapFact,
    UnknownPacket,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceLevel {
    L0,
    L1,
    L2,
    L3,
}

impl EvidenceLevel {
    pub fn from_points(points: u8) -> Self {
        match points {
            0..=2 => Self::L0,
            3..=5 => Self::L1,
            6..=9 => Self::L2,
            _ => Self::L3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::L0 => "L0",
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::L3 => "L3",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateStatus {
    Locked,
    Ready,
    Passed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GamePhase {
    Menu,
    Running,
    Paused,
    Victory,
    Failure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entity {
    pub id: u32,
    pub kind: EntityKind,
    pub lane: u8,
    pub x: i16,
    pub active: bool,
}

impl Entity {
    pub const fn new(id: u32, kind: EntityKind, lane: u8, x: i16) -> Self {
        Self {
            id,
            kind,
            lane,
            x,
            active: true,
        }
    }

    pub fn is_hazard(self) -> bool {
        matches!(
            self.kind,
            EntityKind::RawLeak
                | EntityKind::DirectStim
                | EntityKind::UnsafeBlock
                | EntityKind::Unbounded
                | EntityKind::Overclaim
                | EntityKind::NoTrace
                | EntityKind::RoadmapFact
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameConfig {
    pub seed: u32,
    pub difficulty: u8,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            seed: 1707,
            difficulty: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub tick: u32,
    pub phase: GamePhase,
    pub player_lane: u8,
    pub trust: u8,
    pub risk: u8,
    pub integrity: u8,
    pub evidence_level: EvidenceLevel,
    pub evidence_points: u8,
    pub raw_leaks: u8,
    pub gates_passed: u8,
    pub active_entities: u8,
    pub consent_active: bool,
    pub help_visible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepStatus {
    Running,
    Paused,
    Victory,
    Failure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StepResult {
    pub status: StepStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    pub const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xA50F_1707 } else { seed },
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    pub fn range(&mut self, max_exclusive: u32) -> u32 {
        if max_exclusive == 0 {
            0
        } else {
            self.next_u32() % max_exclusive
        }
    }
}

const MAX_ENTITIES: usize = 18;
const LANES: u8 = 5;
const APP_ZONE_X: i16 = 1010;

pub struct GameState {
    config: GameConfig,
    rng: XorShift32,
    tick: u32,
    phase: GamePhase,
    player_lane: u8,
    trust: i16,
    risk: i16,
    integrity: i16,
    evidence_points: u8,
    raw_leaks: u8,
    consent_ticks: u8,
    gates: [GateStatus; 5],
    entities: [Option<Entity>; MAX_ENTITIES],
    next_id: u32,
    help_visible: bool,
}

impl GameState {
    pub fn new(config: GameConfig) -> Self {
        Self {
            rng: XorShift32::new(config.seed),
            config,
            tick: 0,
            phase: GamePhase::Menu,
            player_lane: 2,
            trust: 42,
            risk: 22,
            integrity: 100,
            evidence_points: 0,
            raw_leaks: 0,
            consent_ticks: 0,
            gates: [GateStatus::Locked; 5],
            entities: [None; MAX_ENTITIES],
            next_id: 1,
            help_visible: false,
        }
    }

    pub fn restart(&mut self) {
        *self = Self::new(self.config);
        self.phase = GamePhase::Running;
    }

    pub fn step(&mut self, action: PlayerAction) -> StepResult {
        match action {
            PlayerAction::Restart => {
                self.restart();
                return StepResult {
                    status: self.step_status(),
                };
            }
            PlayerAction::ToggleHelp => {
                self.help_visible = !self.help_visible;
                return StepResult {
                    status: self.step_status(),
                };
            }
            PlayerAction::Pause => {
                self.phase = match self.phase {
                    GamePhase::Running => GamePhase::Paused,
                    GamePhase::Paused => GamePhase::Running,
                    other => other,
                };
                return StepResult {
                    status: self.step_status(),
                };
            }
            _ => {}
        }

        if self.phase == GamePhase::Menu && !matches!(action, PlayerAction::Idle) {
            self.phase = GamePhase::Running;
        }

        if self.phase != GamePhase::Running {
            return StepResult {
                status: self.step_status(),
            };
        }

        self.tick = self.tick.saturating_add(1);
        self.apply_action(action);
        self.spawn();
        self.advance();
        self.update_gates();
        self.check_terminal();

        StepResult {
            status: self.step_status(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            phase: self.phase,
            player_lane: self.player_lane,
            trust: clamp(self.trust),
            risk: clamp(self.risk),
            integrity: clamp(self.integrity),
            evidence_level: EvidenceLevel::from_points(self.evidence_points),
            evidence_points: self.evidence_points,
            raw_leaks: self.raw_leaks,
            gates_passed: self
                .gates
                .iter()
                .filter(|gate| **gate == GateStatus::Passed)
                .count() as u8,
            active_entities: self
                .entities
                .iter()
                .filter(|entity| entity.is_some())
                .count() as u8,
            consent_active: self.consent_ticks > 0,
            help_visible: self.help_visible,
        }
    }

    pub fn entities(&self) -> &[Option<Entity>; MAX_ENTITIES] {
        &self.entities
    }

    pub fn gates(&self) -> &[GateStatus; 5] {
        &self.gates
    }

    fn apply_action(&mut self, action: PlayerAction) {
        match action {
            PlayerAction::Idle
            | PlayerAction::Restart
            | PlayerAction::ToggleHelp
            | PlayerAction::Pause => {}
            PlayerAction::MoveUp => {
                if self.player_lane > 0 {
                    self.player_lane -= 1;
                }
            }
            PlayerAction::MoveDown => {
                if self.player_lane + 1 < LANES {
                    self.player_lane += 1;
                }
            }
            PlayerAction::ConsentGate => {
                self.consent_ticks = 22;
                self.trust += 2;
            }
            PlayerAction::EvidenceGate => self.pass_next_ready_gate(),
            PlayerAction::Release => self.try_release(),
            PlayerAction::Validate | PlayerAction::Convert | PlayerAction::Quarantine => {
                self.apply_to_nearest(action);
            }
        }

        self.consent_ticks = self.consent_ticks.saturating_sub(1);
    }

    fn apply_to_nearest(&mut self, action: PlayerAction) {
        let mut target = None;
        let mut best_x = i16::MAX;

        for (idx, item) in self.entities.iter().enumerate() {
            if let Some(entity) = item {
                if entity.lane == self.player_lane && entity.x > 215 && entity.x < best_x {
                    target = Some(idx);
                    best_x = entity.x;
                }
            }
        }

        let Some(idx) = target else {
            if matches!(action, PlayerAction::Convert) {
                self.risk += 1;
            }
            return;
        };

        let entity = self.entities[idx].take().unwrap();

        match action {
            PlayerAction::Quarantine => {
                if entity.is_hazard() {
                    self.trust += 4;
                    self.risk -= 5;
                } else {
                    self.trust -= 2;
                    self.risk += 2;
                }
            }
            PlayerAction::Validate => match entity.kind {
                EntityKind::Evidence | EntityKind::Checksum | EntityKind::CiTest => {
                    self.evidence_points = self.evidence_points.saturating_add(1);
                    self.trust += 5;
                    self.integrity += 2;
                }
                _ if entity.is_hazard() => {
                    self.risk -= 2;
                    self.trust += 1;
                }
                _ => self.trust += 1,
            },
            PlayerAction::Convert => match entity.kind {
                EntityKind::IntentCandidate | EntityKind::TypedIntent if self.consent_ticks > 0 => {
                    self.trust += 8;
                    self.evidence_points = self.evidence_points.saturating_add(1);
                }
                _ => {
                    self.risk += 8;
                    self.integrity -= 4;
                }
            },
            _ => {}
        }
    }

    fn spawn(&mut self) {
        let interval = match self.config.difficulty {
            0 => 22,
            1 => 15,
            _ => 11,
        };

        if self.tick % interval != 0 {
            return;
        }

        let kind = match self.rng.range(13) {
            0 => EntityKind::IntentCandidate,
            1 => EntityKind::ConsentToken,
            2 => EntityKind::Evidence,
            3 => EntityKind::Checksum,
            4 => EntityKind::CiTest,
            5 => EntityKind::RawLeak,
            6 => EntityKind::DirectStim,
            7 => EntityKind::UnsafeBlock,
            8 => EntityKind::Unbounded,
            9 => EntityKind::Overclaim,
            10 => EntityKind::NoTrace,
            11 => EntityKind::RoadmapFact,
            _ => EntityKind::UnknownPacket,
        };

        let lane = self.rng.range(LANES as u32) as u8;
        let entity = Entity::new(self.next_id, kind, lane, 92);
        self.next_id = self.next_id.saturating_add(1);

        for slot in self.entities.iter_mut() {
            if slot.is_none() {
                *slot = Some(entity);
                return;
            }
        }

        self.risk += 2;
    }

    fn advance(&mut self) {
        for idx in 0..self.entities.len() {
            if let Some(mut entity) = self.entities[idx] {
                entity.x += 12;
                if entity.x >= APP_ZONE_X {
                    self.cross_boundary(entity);
                    self.entities[idx] = None;
                } else {
                    self.entities[idx] = Some(entity);
                }
            }
        }
    }

    fn cross_boundary(&mut self, entity: Entity) {
        match entity.kind {
            EntityKind::IntentCandidate | EntityKind::TypedIntent if self.consent_ticks > 0 => {
                self.trust += 4;
            }
            EntityKind::RawLeak => {
                self.raw_leaks = self.raw_leaks.saturating_add(1);
                self.integrity -= 25;
                self.risk += 30;
            }
            EntityKind::DirectStim => {
                self.integrity = 0;
                self.risk = 100;
                self.phase = GamePhase::Failure;
            }
            EntityKind::Overclaim | EntityKind::NoTrace | EntityKind::RoadmapFact => {
                self.risk += 15;
                self.trust -= 5;
            }
            EntityKind::UnsafeBlock | EntityKind::Unbounded => {
                self.integrity -= 15;
                self.risk += 12;
            }
            _ => {}
        }
    }

    fn update_gates(&mut self) {
        for idx in 0..self.gates.len() {
            if self.gates[idx] == GateStatus::Locked && self.evidence_points >= (idx as u8 + 1) * 2
            {
                self.gates[idx] = GateStatus::Ready;
            }
        }
    }

    fn pass_next_ready_gate(&mut self) {
        for gate in &mut self.gates {
            if *gate == GateStatus::Ready {
                *gate = GateStatus::Passed;
                self.trust += 10;
                self.risk -= 3;
                return;
            }
        }

        self.risk += 2;
    }

    fn try_release(&mut self) {
        let snapshot = self.snapshot();
        if snapshot.trust >= 90
            && snapshot.risk <= 20
            && snapshot.integrity >= 80
            && matches!(
                snapshot.evidence_level,
                EvidenceLevel::L2 | EvidenceLevel::L3
            )
            && snapshot.gates_passed == 5
            && snapshot.raw_leaks == 0
        {
            self.phase = GamePhase::Victory;
        } else {
            self.risk += 15;
            self.trust -= 8;
        }
    }

    fn check_terminal(&mut self) {
        if self.integrity <= 0 || self.risk >= 100 || self.raw_leaks >= 3 {
            self.phase = GamePhase::Failure;
        }
    }

    fn step_status(&self) -> StepStatus {
        match self.phase {
            GamePhase::Running | GamePhase::Menu => StepStatus::Running,
            GamePhase::Paused => StepStatus::Paused,
            GamePhase::Victory => StepStatus::Victory,
            GamePhase::Failure => StepStatus::Failure,
        }
    }
}

fn clamp(value: i16) -> u8 {
    if value < 0 {
        0
    } else if value > 100 {
        100
    } else {
        value as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_menu_and_safe_to_start() {
        let game = GameState::new(GameConfig::default());
        let snapshot = game.snapshot();
        assert_eq!(snapshot.phase, GamePhase::Menu);
        assert_eq!(snapshot.trust, 42);
        assert_eq!(snapshot.risk, 22);
        assert_eq!(snapshot.integrity, 100);
    }

    #[test]
    fn movement_is_bounded() {
        let mut game = GameState::new(GameConfig::default());
        game.step(PlayerAction::Validate);

        for _ in 0..10 {
            game.step(PlayerAction::MoveUp);
        }
        assert_eq!(game.snapshot().player_lane, 0);

        for _ in 0..10 {
            game.step(PlayerAction::MoveDown);
        }
        assert_eq!(game.snapshot().player_lane, 4);
    }

    #[test]
    fn deterministic_runs_match() {
        let actions = [
            PlayerAction::Validate,
            PlayerAction::ConsentGate,
            PlayerAction::MoveUp,
            PlayerAction::Validate,
            PlayerAction::MoveDown,
            PlayerAction::Quarantine,
            PlayerAction::EvidenceGate,
        ];
        let mut a = GameState::new(GameConfig::default());
        let mut b = GameState::new(GameConfig::default());

        for action in actions {
            a.step(action);
            b.step(action);
        }

        assert_eq!(a.snapshot(), b.snapshot());
    }

    #[test]
    fn restart_resets_run() {
        let mut game = GameState::new(GameConfig::default());
        game.step(PlayerAction::Validate);
        game.step(PlayerAction::MoveUp);
        assert_ne!(game.snapshot().tick, 0);

        game.step(PlayerAction::Restart);
        let snapshot = game.snapshot();
        assert_eq!(snapshot.phase, GamePhase::Running);
        assert_eq!(snapshot.tick, 0);
        assert_eq!(snapshot.player_lane, 2);
    }
}
