//! Deterministic reference policy ("clean") used to generate conformance
//! replay vectors. The policy plays through the public core API only, so a
//! recorded script replays identically inside `verify`.

use neural_boundary_core::{
    Action, Difficulty, EntityKind, EvidenceLevel, GameConfig, GameState, Input, Snapshot, Status,
    BOUNDARY_X, GATE_WINDOW, LANES,
};

#[derive(Clone, Copy, Debug)]
pub struct RecordedAction {
    pub tick: u32,
    pub lane: u8,
    pub action: Action,
}

#[derive(Clone, Copy, Debug)]
pub struct RunSummary {
    pub final_tick: u32,
    pub status: Status,
    pub trust: u8,
    pub risk: u8,
    pub integrity: u8,
    pub evidence_points: u8,
    pub evidence_level: EvidenceLevel,
    pub raw_leaks: u8,
    pub gates_passed: u8,
    pub delivered: u8,
    pub state_hash: u64,
}

pub struct RunResult {
    pub actions: Vec<RecordedAction>,
    pub summary: RunSummary,
}

/// Play a full run with the clean policy. Deterministic for a given config.
pub fn run_clean_policy(config: GameConfig, max_ticks: u32) -> RunResult {
    let mut state = GameState::new(config);
    let mut actions = Vec::new();

    for tick in 1..=max_ticks {
        let input = decide(&state);
        if let (Some(lane), Some(action)) = (input.select_lane, input.action) {
            actions.push(RecordedAction { tick, lane, action });
        }
        state.step(input);
        if state.status() != Status::Running {
            break;
        }
    }

    RunResult {
        summary: summarize(&state),
        actions,
    }
}

/// Replay a recorded script. Deterministic for a given config + script.
pub fn replay_script(
    config: GameConfig,
    actions: &[RecordedAction],
    final_tick: u32,
) -> RunSummary {
    let mut state = GameState::new(config);
    let mut cursor = 0usize;
    for tick in 1..=final_tick {
        let mut input = Input::IDLE;
        if cursor < actions.len() && actions[cursor].tick == tick {
            input.select_lane = Some(actions[cursor].lane);
            input.action = Some(actions[cursor].action);
            cursor += 1;
        }
        state.step(input);
    }
    summarize(&state)
}

pub fn summarize(state: &GameState) -> RunSummary {
    let snapshot = state.snapshot();
    RunSummary {
        final_tick: snapshot.tick,
        status: snapshot.status,
        trust: snapshot.trust,
        risk: snapshot.risk,
        integrity: snapshot.integrity,
        evidence_points: snapshot.evidence_points,
        evidence_level: snapshot.evidence_level,
        raw_leaks: snapshot.raw_leaks,
        gates_passed: snapshot.gates_passed,
        delivered: snapshot.delivered,
        state_hash: state.state_hash(),
    }
}

const WINDOW_START: i32 = BOUNDARY_X - GATE_WINDOW;

pub fn decide(state: &GameState) -> Input {
    let snapshot = state.snapshot();
    if snapshot.cooldown > 0 {
        return Input::IDLE;
    }

    // Per-lane frontmost entity inside the action window: the only entity an
    // action in that lane can reach.
    let mut frontmost: [Option<&neural_boundary_core::Entity>; LANES as usize] =
        [None; LANES as usize];
    for entity in state.entities().iter().flatten() {
        if entity.x >= WINDOW_START && entity.x < BOUNDARY_X {
            let slot = &mut frontmost[entity.lane as usize];
            match slot {
                Some(front) if front.x >= entity.x => {}
                _ => *slot = Some(entity),
            }
        }
    }

    let mut best: Option<(i32, u8, Action)> = None;
    for (lane, slot) in frontmost.iter().enumerate() {
        let Some(entity) = slot else { continue };
        let scored = score(entity, &snapshot);
        if let Some((priority, action)) = scored {
            let rank = priority * 2_000 + entity.x;
            match best {
                Some((existing, _, _)) if existing >= rank => {}
                _ => best = Some((rank, lane as u8, action)),
            }
        }
    }

    if let Some((_, lane, action)) = best {
        return Input {
            select_lane: Some(lane),
            action: Some(action),
        };
    }

    if snapshot.release_ready() {
        return Input {
            select_lane: Some(snapshot.selected_lane),
            action: Some(Action::Release),
        };
    }

    Input::IDLE
}

fn score(entity: &neural_boundary_core::Entity, snapshot: &Snapshot) -> Option<(i32, Action)> {
    let kind = entity.kind;
    if kind.is_hazard() {
        return Some((9, Action::Quarantine));
    }
    if kind == EntityKind::UnknownPacket {
        // Classify concealed traffic before it reaches the membrane.
        if entity.x >= BOUNDARY_X - 260 {
            return Some((8, Action::Validate));
        }
        return None;
    }
    if kind == EntityKind::ConsentToken
        && (!snapshot.consent_active || snapshot.consent_remaining < 420)
    {
        return Some((7, Action::ConsentGate));
    }
    if kind.evidence_points() > 0 && snapshot.evidence_points < 7 {
        return Some((6, Action::EvidenceGate));
    }
    if kind == EntityKind::IntentCandidate {
        if entity.validated {
            if snapshot.consent_active && snapshot.evidence_level >= EvidenceLevel::L1 {
                return Some((5, Action::Convert));
            }
            return None;
        }
        return Some((4, Action::Validate));
    }
    None
}

/// Play a full run with no input at all. Hazards cross freely; the run ends
/// in a boundary breach. Used for the breach demonstration vector.
pub fn run_idle_policy(config: GameConfig, max_ticks: u32) -> RunResult {
    let mut state = GameState::new(config);
    for _ in 1..=max_ticks {
        state.step(Input::IDLE);
        if state.status() != Status::Running {
            break;
        }
    }
    RunResult {
        summary: summarize(&state),
        actions: Vec::new(),
    }
}

/// Search seeds whose clean-policy run lands exactly on the canonical
/// Foundation Grande finals.
pub fn search_seed(
    difficulty: Difficulty,
    from: u64,
    to: u64,
    target: (u8, u8, u8),
    max_ticks: u32,
) -> Option<(u64, RunResult)> {
    for seed in from..=to {
        let result = run_clean_policy(GameConfig { seed, difficulty }, max_ticks);
        let summary = result.summary;
        if summary.status == Status::Victory
            && summary.trust == target.0
            && summary.risk == target.1
            && summary.integrity == target.2
            && summary.raw_leaks == 0
        {
            return Some((seed, result));
        }
    }
    None
}
