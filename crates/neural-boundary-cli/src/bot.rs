// SPDX-FileCopyrightText: 2026 Denis Yermakou
// SPDX-FileContributor: AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//! Reference conformance policy — record/search vectors, not shipped to users.

use neural_boundary_core::{
    Action, Difficulty, Input, Kind, Mode, Rng, Simulation, Status, TerminalReason,
    ACTION_WINDOW_START, BOUNDARY_X,
};

/// Recorded action event for replay files.
#[derive(Clone, Copy, Debug)]
pub struct RecordedInput {
    pub tick: u32,
    pub lane: u8,
    pub action: Action,
}

/// Run outcome carried into the replay `expected` block.
#[derive(Clone, Copy, Debug)]
pub struct RunSummary {
    pub terminal_tick: u32,
    pub status: Status,
    pub reason: TerminalReason,
    pub trust: i32,
    pub risk: i32,
    pub integrity: i32,
    pub evidence_bits: u8,
    pub gate_mask: u8,
    pub raw_leaks: u8,
    pub typed_intents: u8,
    pub quarantined: u32,
    pub wrong_actions: u32,
    pub score: u64,
    pub best_combo: u32,
    pub revocations: u32,
    pub state_hash: u64,
}

pub struct RunResult {
    pub summary: RunSummary,
    pub inputs: Vec<RecordedInput>,
}

/// Policy determines which action the bot takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Full boundary discipline.
    Clean,
    /// No inputs whatsoever.
    Idle,
    /// Ignore ConsentRevoke entities (let them cross).
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

/// Decide the next action for the given policy.
pub fn decide(sim: &Simulation, policy: Policy) -> Input {
    use neural_boundary_core::EvidenceLevel;
    if policy == Policy::Idle {
        return Input::IDLE;
    }
    let snap = sim.snapshot();

    // Attempt release first when all gates pass.
    if snap.gate_mask == neural_boundary_core::ALL_GATES_MASK
        && snap.consent.has_release(snap.consent_epoch, snap.tick)
    {
        return Input {
            lane: None,
            action: Some(Action::Release),
        };
    }

    // Scan frontmost entity in each lane, pick highest-priority action.
    let mut best: Option<(u32, u8, Action)> = None;
    for lane in 0..neural_boundary_core::LANES {
        let Some(entity) = frontmost(sim, lane) else {
            continue;
        };
        let x = entity.logical_x();
        let urgency = (BOUNDARY_X - x).max(0) as u32; // lower = closer to boundary
        let priority: Option<(u32, Action)> = match entity.kind {
            Kind::StimulationCommand => Some((10_000, Action::Quarantine)),
            Kind::RawFrame | Kind::VaultRecord | Kind::RawExportRequest => {
                Some((9_000 + urgency, Action::Quarantine))
            }
            Kind::Artifact => Some((8_000 + urgency, Action::Quarantine)),
            Kind::ConsentRevoke if policy != Policy::Lapse => {
                Some((7_500 + urgency, Action::Consent))
            }
            Kind::UnsupportedClaim | Kind::UntraceableClaim | Kind::RoadmapAsFact => {
                Some((7_000 + urgency, Action::Quarantine))
            }
            Kind::ConsentGrant if snap.consent.scope_mask == 0 => {
                Some((6_500 + urgency, Action::Consent))
            }
            Kind::EvidenceTrace
                if snap.evidence_bits & neural_boundary_core::EVIDENCE_TRACE == 0 =>
            {
                Some((6_000 + urgency, Action::Evidence))
            }
            Kind::ChecksumProof
                if snap.evidence_bits & neural_boundary_core::EVIDENCE_CHECKSUM == 0
                    && snap.evidence_bits & neural_boundary_core::EVIDENCE_TRACE != 0 =>
            {
                Some((5_800 + urgency, Action::Evidence))
            }
            Kind::CiProof
                if snap.evidence_bits & neural_boundary_core::EVIDENCE_CI == 0
                    && snap.evidence_bits & neural_boundary_core::EVIDENCE_CHECKSUM != 0 =>
            {
                Some((5_600 + urgency, Action::Evidence))
            }
            Kind::ValidatedIntent
                if snap.consent.has_convert(snap.consent_epoch, snap.tick)
                    && EvidenceLevel::from_bits(snap.evidence_bits)
                        >= sim.mode().convert_evidence() =>
            {
                Some((5_000 + urgency, Action::Convert))
            }
            Kind::CandidateIntent | Kind::UnknownPacket | Kind::DeadlineHazard => {
                Some((4_500 + urgency, Action::Validate))
            }
            _ => None,
        };
        if let Some((p, action)) = priority {
            if best.map(|(bp, _, _)| p > bp).unwrap_or(true) {
                best = Some((p, lane, action));
            }
        }
    }
    match best {
        Some((_, lane, action)) => Input {
            lane: Some(lane),
            action: Some(action),
        },
        None => Input::IDLE,
    }
}

fn frontmost(sim: &Simulation, lane: u8) -> Option<&neural_boundary_core::Entity> {
    let mut best: Option<&neural_boundary_core::Entity> = None;
    for slot in sim.pool().iter().flatten() {
        if slot.lane != lane {
            continue;
        }
        let x = slot.logical_x();
        if !(ACTION_WINDOW_START..BOUNDARY_X).contains(&x) {
            continue;
        }
        if best.map(|b| x > b.logical_x()).unwrap_or(true) {
            best = Some(slot);
        }
    }
    best
}

/// Run a policy to terminal, recording all inputs.
pub fn run_policy(
    mode: Mode,
    difficulty: Difficulty,
    seed: u64,
    policy: Policy,
    max_ticks: u32,
) -> RunResult {
    let mut sim = Simulation::new(mode, difficulty, seed);
    let mut inputs = Vec::new();
    let mut revocations = 0u32;
    let mut prev_epoch = sim.consent_epoch();
    for t in 1..=max_ticks {
        let input = decide(&sim, policy);
        if let Some(action) = input.action {
            inputs.push(RecordedInput {
                tick: t,
                lane: input.lane.unwrap_or(sim.selected_lane()),
                action,
            });
        }
        sim.step(input);
        let new_epoch = sim.consent_epoch();
        revocations += new_epoch - prev_epoch;
        prev_epoch = new_epoch;
        if sim.status().is_terminal() {
            break;
        }
    }
    RunResult {
        summary: summarize(&sim, revocations),
        inputs,
    }
}

pub fn summarize(sim: &Simulation, revocations: u32) -> RunSummary {
    let snap = sim.snapshot();
    RunSummary {
        terminal_tick: snap.tick,
        status: snap.status,
        reason: snap.reason,
        trust: snap.trust,
        risk: snap.risk,
        integrity: snap.integrity,
        evidence_bits: snap.evidence_bits,
        gate_mask: snap.gate_mask,
        raw_leaks: snap.raw_leaks,
        typed_intents: snap.typed_intents,
        quarantined: snap.quarantined,
        wrong_actions: snap.wrong_actions,
        score: snap.score,
        best_combo: snap.best_combo,
        revocations,
        state_hash: sim.state_hash(),
    }
}

/// Replay a recorded script up to terminal_tick.
pub fn replay_script(
    mode: Mode,
    difficulty: Difficulty,
    seed: u64,
    inputs: &[RecordedInput],
    terminal_tick: u32,
) -> RunSummary {
    let mut sim = Simulation::new(mode, difficulty, seed);
    let mut cursor = 0usize;
    let mut revocations = 0u32;
    let mut prev_epoch = sim.consent_epoch();
    for t in 1..=terminal_tick {
        let input = if cursor < inputs.len() && inputs[cursor].tick == t {
            let e = inputs[cursor];
            cursor += 1;
            Input {
                lane: Some(e.lane),
                action: Some(e.action),
            }
        } else {
            Input::IDLE
        };
        sim.step(input);
        let new_epoch = sim.consent_epoch();
        revocations += new_epoch - prev_epoch;
        prev_epoch = new_epoch;
        if sim.status().is_terminal() {
            break;
        }
    }
    summarize(&sim, revocations)
}

pub struct SearchGoal {
    pub from: u64,
    pub to: u64,
    pub want_reason: TerminalReason,
    pub min_revocations: u32,
    pub max_ticks: u32,
}

/// Search seeds for a run matching goal criteria.
pub fn search_seed(
    mode: Mode,
    difficulty: Difficulty,
    policy: Policy,
    goal: SearchGoal,
) -> Option<(u64, RunResult)> {
    for seed in goal.from..=goal.to {
        let result = run_policy(mode, difficulty, seed, policy, goal.max_ticks);
        if result.summary.reason != goal.want_reason {
            continue;
        }
        if result.summary.revocations < goal.min_revocations {
            continue;
        }
        return Some((seed, result));
    }
    None
}

/// Generate a seed using wall-clock mix — for record command.
pub fn mix_seed(base: u64, counter: u64) -> u64 {
    let mut r = Rng::new(base ^ (counter << 48) ^ 0x9E37_79B9_7F4A_7C15);
    let v = r.next_u64();
    if v == 0 {
        0x3001
    } else {
        v
    }
}
