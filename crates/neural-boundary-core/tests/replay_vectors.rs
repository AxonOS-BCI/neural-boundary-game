// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.9.812).
// See LICENSE and IP_NOTICE.md for details.

use neural_boundary_core::{
    daily_seed, outcome, replay_run, run_builtin, verify_vector, ExpectedOutcome, PlayerAction,
    ReplayAction, BUILTIN_VECTORS,
};

#[test]
fn builtin_vectors_are_deterministic() {
    for i in 0..BUILTIN_VECTORS.len() {
        let a = run_builtin(i).expect("builtin index valid");
        let b = run_builtin(i).expect("builtin index valid");
        assert_eq!(a, b, "builtin vector {i}");
    }
}

#[test]
fn replay_matches_self_derived_expected() {
    // Derive the expected outcome from one run, then verify a fresh run.
    let actions = [
        ReplayAction {
            tick: 60,
            action: PlayerAction::Throttle,
        },
        ReplayAction {
            tick: 245,
            action: PlayerAction::Audit,
        },
        ReplayAction {
            tick: 260,
            action: PlayerAction::Throttle,
        },
    ];
    let derived = outcome(4, 4, &actions).expect("scenario exists");
    let expected = ExpectedOutcome {
        grade: derived.grade,
        reason: derived.reason,
        score_min: derived.score,
        state_hash: derived.state_hash,
    };
    let g = replay_run(4, 4, &actions).expect("scenario exists");
    assert!(verify_vector(&g, &expected));
}

#[test]
fn empty_action_stream_reaches_a_terminal() {
    // An idle run must still terminate deterministically (timeout or failure).
    let g = replay_run(1, 1, &[]).expect("scenario exists");
    assert!(g.status().is_terminal());
}

#[test]
fn daily_seed_crosscheck() {
    // Stable for the same date, sensitive to the date, never zero.
    assert_eq!(daily_seed(2026, 6, 16), daily_seed(2026, 6, 16));
    assert_ne!(daily_seed(2026, 6, 16), daily_seed(2026, 6, 17));
    assert_ne!(daily_seed(2026, 6, 16), 0);
}

#[test]
fn invalid_scenario_is_none() {
    assert!(replay_run(99, 1, &[]).is_none());
}
