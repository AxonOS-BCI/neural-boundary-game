// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.9.812).
// See LICENSE and IP_NOTICE.md for details.

use neural_boundary_core::{EndReason, GameState, PermissionScope, PlayerAction, SovereigntyGrade};

#[test]
fn boundary_collapse_minus_1000_breached() {
    let mut g = GameState::new(9, 1).unwrap();
    g.metrics.boundary_integrity = 0;
    g.advance(PlayerAction::None);
    assert_eq!(g.end_reason(), EndReason::BoundaryCollapse);
    assert_eq!(g.grade(), SovereigntyGrade::Breached);
    assert_eq!(g.score(), -1000);
}

#[test]
fn raw_leak_limit_minus_1000_breached() {
    let mut g = GameState::new(2, 1).unwrap();
    g.metrics.raw_leak_risk = 100;
    g.advance(PlayerAction::None);
    assert_eq!(g.end_reason(), EndReason::RawLeakLimit);
    assert_eq!(g.grade(), SovereigntyGrade::Breached);
    assert_eq!(g.score(), -1000);
}

#[test]
fn stimulation_overflow_minus_1000_unsafe() {
    let mut g = GameState::new(4, 1).unwrap();
    g.metrics.stimulation_risk = 100;
    g.advance(PlayerAction::None);
    assert_eq!(g.end_reason(), EndReason::UnsafeStimulationEscape);
    assert_eq!(g.grade(), SovereigntyGrade::Unsafe);
    assert_eq!(g.score(), -1000);
}

#[test]
fn consent_collapse_minus_900_breached() {
    let mut g = GameState::new(3, 1).unwrap();
    g.permissions.active_scopes = PermissionScope::RAW; // sensitive permission active
    g.metrics.consent_coherence = 0;
    g.advance(PlayerAction::None);
    assert_eq!(g.end_reason(), EndReason::ConsentCollapse);
    assert_eq!(g.grade(), SovereigntyGrade::Breached);
    assert_eq!(g.score(), -900);
}

#[test]
fn consent_zero_without_sensitive_is_not_collapse() {
    let mut g = GameState::new(1, 1).unwrap();
    // Only FLOW scope is active → consent at 0 is not an immediate collapse.
    g.metrics.consent_coherence = 0;
    g.advance(PlayerAction::None);
    assert_ne!(g.end_reason(), EndReason::ConsentCollapse);
}

#[test]
fn terminal_state_hash_is_immutable() {
    let mut g = GameState::new(2, 1).unwrap();
    g.metrics.boundary_integrity = 0;
    g.advance(PlayerAction::None);
    let h = g.state_hash();
    let t = g.tick();
    g.advance(PlayerAction::Release);
    g.advance(PlayerAction::Audit);
    assert_eq!(g.tick(), t);
    assert_eq!(g.state_hash(), h);
}
