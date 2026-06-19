// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.0.1).
// See LICENSE and IP_NOTICE.md for details.

//! Deterministic replay (§19). JSON (de)serialization lives in the CLI (std);
//! this module is `no_std` and operates on already-parsed action slices.
//!
//! Canonical contract (§19.1): `scenario_id + seed + action_stream` ⇒
//! `grade + score + state_hash`. An action recorded at tick `T` is applied on
//! the `advance` that brings the simulation to tick `T`. Ticks ascend; at most
//! one action per tick. This ordering is the field order documented in
//! `docs/REPLAY_VECTORS.md`.

use crate::actions::PlayerAction;
use crate::state::{EndReason, GameState, RunStatus, SovereigntyGrade};

/// One scheduled action in a replay vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayAction {
    pub tick: u32,
    pub action: PlayerAction,
}

/// Expected terminal outcome (§19.2 `expected` block).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedOutcome {
    pub grade: SovereigntyGrade,
    pub reason: EndReason,
    pub score_min: i32,
    pub state_hash: u64,
}

/// Outcome of replaying a vector, for reporting and cross-checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VectorOutcome {
    pub scenario_id: u32,
    pub seed: u64,
    pub final_tick: u32,
    pub grade: SovereigntyGrade,
    pub reason: EndReason,
    pub score: i32,
    pub state_hash: u64,
}

/// Hard safety cap: 180 s at 20 Hz is 3600 ticks; allow a small margin.
const REPLAY_TICK_CAP: u32 = 3600 + 64;

/// Replay an action stream deterministically. `actions` must be sorted by
/// strictly increasing tick (the validator enforces this).
pub fn run(scenario_id: u32, seed: u64, actions: &[ReplayAction]) -> Option<GameState> {
    let mut g = GameState::new(scenario_id, seed)?;
    let mut ai = 0usize;
    while g.status() == RunStatus::Running {
        let next_tick = g.tick() + 1;
        let action = if ai < actions.len() && actions[ai].tick == next_tick {
            let a = actions[ai].action;
            ai += 1;
            a
        } else {
            PlayerAction::None
        };
        g.advance(action);
        if g.tick() >= REPLAY_TICK_CAP {
            break;
        }
    }
    Some(g)
}

pub fn outcome(scenario_id: u32, seed: u64, actions: &[ReplayAction]) -> Option<VectorOutcome> {
    let g = run(scenario_id, seed, actions)?;
    Some(VectorOutcome {
        scenario_id,
        seed,
        final_tick: g.tick(),
        grade: g.grade(),
        reason: g.end_reason(),
        score: g.score(),
        state_hash: g.state_hash(),
    })
}

/// Verify a finished run against an expected outcome (§19.2): grade, reason,
/// `score >= score_min`, and exact state hash must all match.
pub fn verify(g: &GameState, expected: &ExpectedOutcome) -> bool {
    g.grade() == expected.grade
        && g.end_reason() == expected.reason
        && g.score() >= expected.score_min
        && g.state_hash() == expected.state_hash
}

// ── Builtin vectors (for ABI `nbg_run_builtin_vector`) ───────────────────────
//
// A minimal deterministic registry so the WASM ABI can self-check without the
// JSON fixtures. The full 16-vector fixture set (§19.3) lives in `vectors/`
// and is authored in a later phase; these builtins exercise the engine paths.

/// A builtin vector: name, scenario, seed, and its action stream.
pub struct BuiltinVector {
    pub name: &'static str,
    pub scenario_id: u32,
    pub seed: u64,
    pub actions: &'static [ReplayAction],
}

use PlayerAction::*;

const fn a(tick: u32, action: PlayerAction) -> ReplayAction {
    ReplayAction { tick, action }
}

// Idle run on the onboarding scenario → deterministic timeout outcome.
static V_CLEAN_IDLE: [ReplayAction; 0] = [];
// A few audits and a seal on the raw-leak scenario.
static V_RAW_GUARDED: [ReplayAction; 6] = [
    a(225, Audit),
    a(240, SealVault),
    a(250, Authorize),
    a(726, Audit),
    a(740, SealVault),
    a(1245, SealVault),
];
// Throttle-first discipline on the unsafe-stimulation scenario.
static V_STIM_THROTTLED: [ReplayAction; 6] = [
    a(60, Throttle),
    a(245, Audit),
    a(260, Throttle),
    a(726, Audit),
    a(740, Throttle),
    a(1225, Throttle),
];

pub static BUILTIN_VECTORS: [BuiltinVector; 3] = [
    BuiltinVector {
        name: "builtin-clean-idle",
        scenario_id: 1,
        seed: 1,
        actions: &V_CLEAN_IDLE,
    },
    BuiltinVector {
        name: "builtin-raw-guarded",
        scenario_id: 2,
        seed: 2,
        actions: &V_RAW_GUARDED,
    },
    BuiltinVector {
        name: "builtin-stim-throttled",
        scenario_id: 4,
        seed: 4,
        actions: &V_STIM_THROTTLED,
    },
];

pub fn run_builtin(index: usize) -> Option<VectorOutcome> {
    let v = BUILTIN_VECTORS.get(index)?;
    outcome(v.scenario_id, v.seed, v.actions)
}
