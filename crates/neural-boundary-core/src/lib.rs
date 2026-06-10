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
    Restart,
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
pub enum GamePhase {
    Menu,
    Running,
    Victory,
    Failure,
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
    pub lane: u8,
    pub trust: u8,
    pub risk: u8,
    pub integrity: u8,
    pub evidence_level: EvidenceLevel,
    pub evidence_points: u8,
    pub gates_passed: u8,
    pub raw_leaks: u8,
    pub consent_active: bool,
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
}

pub struct GameState {
    config: GameConfig,
    rng: XorShift32,
    tick: u32,
    phase: GamePhase,
    lane: u8,
    trust: i16,
    risk: i16,
    integrity: i16,
    evidence_points: u8,
    gates_passed: u8,
    raw_leaks: u8,
    consent_ticks: u8,
}

impl GameState {
    pub fn new(config: GameConfig) -> Self {
        Self {
            rng: XorShift32::new(config.seed),
            config,
            tick: 0,
            phase: GamePhase::Menu,
            lane: 2,
            trust: 42,
            risk: 22,
            integrity: 100,
            evidence_points: 0,
            gates_passed: 0,
            raw_leaks: 0,
            consent_ticks: 0,
        }
    }

    pub fn restart(&mut self) {
        *self = Self::new(self.config);
        self.phase = GamePhase::Running;
    }

    pub fn step(&mut self, action: PlayerAction) -> Snapshot {
        if action == PlayerAction::Restart {
            self.restart();
            return self.snapshot();
        }

        if self.phase == GamePhase::Menu && action != PlayerAction::Idle {
            self.phase = GamePhase::Running;
        }

        if self.phase != GamePhase::Running {
            return self.snapshot();
        }

        self.tick = self.tick.saturating_add(1);
        self.consent_ticks = self.consent_ticks.saturating_sub(1);

        match action {
            PlayerAction::Idle => self.drift(),
            PlayerAction::MoveUp => {
                if self.lane > 0 {
                    self.lane -= 1;
                }
            }
            PlayerAction::MoveDown => {
                if self.lane < 4 {
                    self.lane += 1;
                }
            }
            PlayerAction::Validate => {
                self.trust += 2;
                self.evidence_points = self.evidence_points.saturating_add(1);
            }
            PlayerAction::Convert => {
                if self.consent_ticks > 0 {
                    self.trust += 4;
                    self.risk -= 1;
                } else {
                    self.risk += 5;
                    self.integrity -= 2;
                }
            }
            PlayerAction::Quarantine => {
                self.risk -= 4;
                self.trust += 1;
            }
            PlayerAction::ConsentGate => {
                self.consent_ticks = 24;
                self.trust += 1;
            }
            PlayerAction::EvidenceGate => {
                if self.evidence_points >= (self.gates_passed + 1) * 2 && self.gates_passed < 5 {
                    self.gates_passed += 1;
                    self.trust += 6;
                    self.risk -= 2;
                } else {
                    self.risk += 1;
                }
            }
            PlayerAction::Release => self.try_release(),
            PlayerAction::Restart => {}
        }

        self.check_terminal();
        self.snapshot()
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            phase: self.phase,
            lane: self.lane,
            trust: clamp(self.trust),
            risk: clamp(self.risk),
            integrity: clamp(self.integrity),
            evidence_level: EvidenceLevel::from_points(self.evidence_points),
            evidence_points: self.evidence_points,
            gates_passed: self.gates_passed,
            raw_leaks: self.raw_leaks,
            consent_active: self.consent_ticks > 0,
        }
    }

    fn drift(&mut self) {
        let roll = self.rng.next_u32() % 11;
        match roll {
            0 => self.risk += self.config.difficulty as i16,
            1 => self.trust += 1,
            2 => self.evidence_points = self.evidence_points.saturating_add(1),
            _ => {}
        }
    }

    fn try_release(&mut self) {
        let snapshot = self.snapshot();
        if snapshot.trust >= 90
            && snapshot.risk <= 20
            && snapshot.integrity >= 80
            && matches!(snapshot.evidence_level, EvidenceLevel::L2 | EvidenceLevel::L3)
            && snapshot.gates_passed == 5
            && snapshot.raw_leaks == 0
        {
            self.phase = GamePhase::Victory;
        } else {
            self.risk += 12;
            self.trust -= 5;
        }
    }

    fn check_terminal(&mut self) {
        if self.integrity <= 0 || self.risk >= 100 || self.raw_leaks >= 3 {
            self.phase = GamePhase::Failure;
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
    fn initial_state_is_stable() {
        let game = GameState::new(GameConfig::default());
        let snapshot = game.snapshot();
        assert_eq!(snapshot.tick, 0);
        assert_eq!(snapshot.phase, GamePhase::Menu);
        assert_eq!(snapshot.trust, 42);
        assert_eq!(snapshot.risk, 22);
        assert_eq!(snapshot.integrity, 100);
    }

    #[test]
    fn restart_does_not_add_tick() {
        let mut game = GameState::new(GameConfig::default());
        game.step(PlayerAction::Validate);
        assert_eq!(game.snapshot().tick, 1);

        let snapshot = game.step(PlayerAction::Restart);
        assert_eq!(snapshot.tick, 0);
        assert_eq!(snapshot.phase, GamePhase::Running);
    }

    #[test]
    fn movement_is_bounded() {
        let mut game = GameState::new(GameConfig::default());

        for _ in 0..10 {
            game.step(PlayerAction::MoveUp);
        }
        assert_eq!(game.snapshot().lane, 0);

        for _ in 0..10 {
            game.step(PlayerAction::MoveDown);
        }
        assert_eq!(game.snapshot().lane, 4);
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
            PlayerAction::Idle,
            PlayerAction::Idle,
        ];

        let mut first = GameState::new(GameConfig::default());
        let mut second = GameState::new(GameConfig::default());

        for action in actions {
            first.step(action);
            second.step(action);
        }

        assert_eq!(first.snapshot(), second.snapshot());
    }
}
