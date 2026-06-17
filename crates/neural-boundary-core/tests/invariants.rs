// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
// See LICENSE and IP_NOTICE.md for details.

use neural_boundary_core::{
    EndReason, GameState, Metrics, PlayerAction, Rng, SovereigntyGrade, SCENARIOS,
};

fn drive(scenario_id: u32, seed: u64, ticks: u32, drv_seed: u64) -> GameState {
    let mut g = GameState::new(scenario_id, seed).expect("scenario exists");
    let mut drv = Rng::new(drv_seed);
    for _ in 0..ticks {
        if g.status().is_terminal() {
            break;
        }
        let action = PlayerAction::from_u8(drv.range(8) as u8).unwrap_or(PlayerAction::None);
        g.advance(action);
    }
    g
}

#[test]
fn metrics_stay_in_bounds_all_scenarios() {
    for s in SCENARIOS.iter() {
        for case in 0..6u64 {
            let g = drive(s.id, 0xA11CE ^ case, s.max_ticks + 16, 0x5EED ^ case);
            assert!(g.metrics().in_bounds(), "scenario {} case {case}", s.id);
            // A clamped metric can never exceed 100.
            let m = g.metrics();
            assert!(m.boundary_integrity <= 100 && m.raw_leak_risk <= 100);
        }
    }
}

#[test]
fn determinism_same_inputs_same_hash() {
    for s in SCENARIOS.iter() {
        let a = drive(s.id, 0xC0FFEE, s.max_ticks, 0x1234);
        let b = drive(s.id, 0xC0FFEE, s.max_ticks, 0x1234);
        assert_eq!(a.state_hash(), b.state_hash(), "scenario {}", s.id);
        assert_eq!(a.grade(), b.grade());
        assert_eq!(a.score(), b.score());
    }
}

#[test]
fn different_seed_diverges() {
    let a = drive(9, 100, 1500, 0x42);
    let b = drive(9, 101, 1500, 0x42);
    assert_ne!(a.state_hash(), b.state_hash());
}

#[test]
fn unsafe_breached_never_sealed() {
    // Force each terminal failure and confirm the grade is never SEALED.
    let mut g = GameState::new(5, 3).unwrap();
    g.metrics.boundary_integrity = 0;
    g.advance(PlayerAction::None);
    assert_ne!(g.grade(), SovereigntyGrade::Sealed);
    assert_eq!(g.grade(), SovereigntyGrade::Breached);

    let mut g = GameState::new(2, 3).unwrap();
    g.metrics.raw_leak_risk = 100;
    g.advance(PlayerAction::None);
    assert_eq!(g.grade(), SovereigntyGrade::Breached);

    let mut g = GameState::new(4, 3).unwrap();
    g.metrics.stimulation_risk = 100;
    g.advance(PlayerAction::None);
    assert_eq!(g.grade(), SovereigntyGrade::Unsafe);
    assert_eq!(g.end_reason(), EndReason::UnsafeStimulationEscape);
}

#[test]
fn metrics_helper_clamps() {
    let mut v = 95u8;
    Metrics::adjust(&mut v, 20);
    assert_eq!(v, 100);
    Metrics::adjust(&mut v, -150);
    assert_eq!(v, 0);
}
