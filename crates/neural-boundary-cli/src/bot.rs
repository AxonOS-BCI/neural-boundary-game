//! Deterministic reference policies used to record and search replay
//! vectors. The policies are part of the conformance toolkit, not of the
//! authoritative core.

use neural_boundary_core::{
    Action, ConsentState, Entity, EntityKind, Event, EvidenceLevel, Grade, Input, RunMode,
    Simulation, SimulationConfig, Status, TerminalReason, BOUNDARY_X, EVIDENCE_POINTS_MAX,
    GATE_WINDOW, LANES,
};

/// One recorded input event.
#[derive(Clone, Copy, Debug)]
pub struct RecordedInput {
    pub tick: u32,
    pub lane: u8,
    pub action: Action,
}

/// Terminal summary of a run, mirrored into the replay `expected` block.
#[derive(Clone, Copy, Debug)]
pub struct RunSummary {
    pub terminal_tick: u32,
    pub status: Status,
    pub grade: Grade,
    pub trust: i32,
    pub risk: i32,
    pub integrity: i32,
    pub evidence_points: u8,
    pub evidence_level: EvidenceLevel,
    pub gates_passed: u8,
    pub raw_leaks: u8,
    pub delivered: u8,
    pub score: u32,
    pub best_streak: u32,
    pub revocations: u8,
    pub state_hash: u64,
}

pub struct RunResult {
    pub summary: RunSummary,
    pub inputs: Vec<RecordedInput>,
}

/// Recording policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Full boundary discipline: contain, classify, gate, convert, seal.
    Clean,
    /// No input at all: the world crosses freely.
    Idle,
    /// Boundary discipline, except revoked-consent credentials are ignored
    /// and allowed to cross — demonstrating immediate revocation and
    /// recovery through a fresh consent grant.
    Lapse,
}

impl Policy {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Idle => "idle",
            Self::Lapse => "lapse",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "clean" => Some(Self::Clean),
            "idle" => Some(Self::Idle),
            "lapse" => Some(Self::Lapse),
            _ => None,
        }
    }
}

pub fn summarize(simulation: &Simulation, revocations: u8) -> RunSummary {
    let snapshot = simulation.snapshot();
    RunSummary {
        terminal_tick: snapshot.tick,
        status: snapshot.status,
        grade: simulation.grade(),
        trust: snapshot.trust,
        risk: snapshot.risk,
        integrity: snapshot.integrity,
        evidence_points: snapshot.evidence_points,
        evidence_level: snapshot.evidence_level,
        gates_passed: snapshot.gates_passed,
        raw_leaks: snapshot.raw_leaks,
        delivered: snapshot.delivered,
        score: snapshot.score,
        best_streak: snapshot.best_streak,
        revocations,
        state_hash: simulation.state_hash(),
    }
}

/// Decide the next input for a policy. Public so `trace` can reuse it.
pub fn decide(simulation: &Simulation, policy: Policy) -> Input {
    if policy == Policy::Idle {
        return Input::IDLE;
    }
    let snapshot = simulation.snapshot();
    if snapshot.cooldown > 0 {
        return Input::IDLE;
    }
    if simulation.release_blocker().is_none() {
        return Input {
            select_lane: None,
            action: Some(Action::Release),
        };
    }

    let rules = simulation.rules();
    let consent_active = matches!(snapshot.consent, ConsentState::Active { .. });
    let consent_low = snapshot.consent_remaining < 480;
    let evidence_target = rules
        .evidence_points_min
        .saturating_add(2)
        .min(EVIDENCE_POINTS_MAX);

    let mut best: Option<(u32, u8, Action)> = None;
    let mut consider = |rank: u32, lane: u8, action: Action| {
        if best.map(|(current, _, _)| rank > current).unwrap_or(true) {
            best = Some((rank, lane, action));
        }
    };

    for lane in 0..LANES {
        let Some(front) = frontmost(simulation, lane) else {
            continue;
        };
        let priority: Option<(u32, Action)> = match front.kind {
            EntityKind::StimulationCommand => Some((95, Action::Quarantine)),
            EntityKind::RawFrame => Some((90, Action::Quarantine)),
            EntityKind::RevokedConsent => {
                if policy == Policy::Lapse {
                    None
                } else {
                    Some((88, Action::Quarantine))
                }
            }
            EntityKind::Artifact
            | EntityKind::UnsupportedClaim
            | EntityKind::UntraceableClaim
            | EntityKind::RoadmapAsFactClaim => Some((82, Action::Quarantine)),
            EntityKind::UnknownPacket if front.x >= BOUNDARY_X - 260 => {
                Some((76, Action::Validate))
            }
            EntityKind::ConsentToken if !consent_active || consent_low => {
                Some((70, Action::ConsentGate))
            }
            EntityKind::Evidence | EntityKind::Checksum | EntityKind::CiTest
                if snapshot.evidence_points < evidence_target =>
            {
                Some((64, Action::EvidenceGate))
            }
            EntityKind::ValidatedIntent
                if consent_active
                    && EvidenceLevel::from_points(snapshot.evidence_points)
                        >= EvidenceLevel::L1 =>
            {
                Some((58, Action::Convert))
            }
            EntityKind::IntentCandidate => Some((50, Action::Validate)),
            _ => None,
        };
        if let Some((priority, action)) = priority {
            consider(priority * 2_000 + front.x.max(0) as u32, lane, action);
        }
    }

    match best {
        Some((_, lane, action)) => Input {
            select_lane: Some(lane),
            action: Some(action),
        },
        None => Input::IDLE,
    }
}

fn frontmost(simulation: &Simulation, lane: u8) -> Option<Entity> {
    let mut best: Option<Entity> = None;
    for entity in simulation.entities().iter().flatten() {
        if entity.lane != lane {
            continue;
        }
        if entity.x < BOUNDARY_X - GATE_WINDOW || entity.x >= BOUNDARY_X {
            continue;
        }
        if best.map(|current| entity.x > current.x).unwrap_or(true) {
            best = Some(*entity);
        }
    }
    best
}

/// Run a policy to its terminal state (bounded by `max_ticks`), recording
/// every applied input and counting consent revocations.
pub fn run_policy(config: SimulationConfig, policy: Policy, max_ticks: u32) -> RunResult {
    let mut simulation = Simulation::new(config);
    let mut inputs = Vec::new();
    let mut revocations = 0u8;
    for tick in 1..=max_ticks {
        let input = decide(&simulation, policy);
        if input != Input::IDLE {
            if let Some(action) = input.action {
                // Replay sets the lane explicitly, so a recorded input must
                // carry the lane that is actually selected at this tick.
                let lane = input
                    .select_lane
                    .unwrap_or_else(|| simulation.snapshot().selected_lane);
                inputs.push(RecordedInput { tick, lane, action });
            }
        }
        simulation.step(input);
        for event in simulation.events() {
            if matches!(event, Event::ConsentRevoked { .. }) {
                revocations = revocations.saturating_add(1);
            }
        }
        if simulation.status() != Status::Running {
            break;
        }
    }
    RunResult {
        summary: summarize(&simulation, revocations),
        inputs,
    }
}

/// Replay a recorded input script to `terminal_tick`, counting revocations.
pub fn replay_script(
    config: SimulationConfig,
    inputs: &[RecordedInput],
    terminal_tick: u32,
) -> RunSummary {
    let mut simulation = Simulation::new(config);
    let mut cursor = 0usize;
    let mut revocations = 0u8;
    for tick in 1..=terminal_tick {
        let input = if cursor < inputs.len() && inputs[cursor].tick == tick {
            let entry = inputs[cursor];
            cursor += 1;
            Input {
                select_lane: Some(entry.lane),
                action: Some(entry.action),
            }
        } else {
            Input::IDLE
        };
        simulation.step(input);
        for event in simulation.events() {
            if matches!(event, Event::ConsentRevoked { .. }) {
                revocations = revocations.saturating_add(1);
            }
        }
        if simulation.status() != Status::Running {
            break;
        }
    }
    summarize(&simulation, revocations)
}

/// What a seed search is looking for.
#[derive(Clone, Copy, Debug)]
pub struct SearchGoal {
    pub reason: TerminalReason,
    pub finals: Option<(i32, i32, i32)>,
    pub min_revocations: u8,
}

/// Scan seeds in `from..=to` for a run matching the goal under a policy.
pub fn search_seed(
    mode: RunMode,
    difficulty: neural_boundary_core::Difficulty,
    policy: Policy,
    from: u64,
    to: u64,
    goal: SearchGoal,
    max_ticks: u32,
) -> Option<(u64, RunResult)> {
    for seed in from..=to {
        let result = run_policy(
            SimulationConfig {
                seed,
                mode,
                difficulty,
            },
            policy,
            max_ticks,
        );
        let summary = result.summary;
        if summary.status != Status::Terminal(goal.reason) {
            continue;
        }
        if let Some((trust, risk, integrity)) = goal.finals {
            if summary.trust != trust || summary.risk != risk || summary.integrity != integrity {
                continue;
            }
        }
        if summary.revocations < goal.min_revocations {
            continue;
        }
        return Some((seed, result));
    }
    None
}
