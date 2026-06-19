// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
// See LICENSE and IP_NOTICE.md for details.

use neural_boundary_core::{ActionGate, ActionResult, GameState, PlayerAction};

#[test]
fn gate_rejects_second_action_same_tick() {
    let mut gate = ActionGate::default();
    gate.begin_tick();
    assert_eq!(gate.gate(10), None); // first action allowed
    gate.record_accept(10);
    // Second action in the same tick is rejected (max_actions_per_tick = 1).
    assert_eq!(gate.gate(10), Some(ActionResult::RejectedCooldown));
}

#[test]
fn gate_allows_next_tick() {
    let mut gate = ActionGate::default();
    gate.begin_tick();
    gate.record_accept(10);
    gate.begin_tick(); // advance to next tick
    assert_eq!(gate.gate(11), None);
}

#[test]
fn gate_enforces_cooldown_window() {
    let mut gate = ActionGate::default();
    gate.begin_tick();
    gate.record_accept(10);
    gate.begin_tick();
    // A tick that has not yet cleared the cooldown is rejected.
    assert_eq!(gate.gate(10), Some(ActionResult::RejectedCooldown));
}

#[test]
fn authorize_without_event_is_invalid() {
    // Scenario 1 has no event at tick 1 → Authorize is invalid-for-event.
    let mut g = GameState::new(1, 1).unwrap();
    let r = g.advance(PlayerAction::Authorize);
    assert_eq!(r, ActionResult::RejectedInvalidForEvent);
    assert_eq!(
        g.last_action_result(),
        ActionResult::RejectedInvalidForEvent
    );
}

#[test]
fn release_before_stable_is_release_locked() {
    let mut g = GameState::new(9, 1).unwrap();
    // Tick 1, nothing stabilized, < 30 s elapsed.
    let r = g.advance(PlayerAction::Release);
    assert_eq!(r, ActionResult::RejectedReleaseLocked);
    assert!(!g.status().is_terminal());
}

#[test]
fn terminal_state_rejects_actions() {
    let mut g = GameState::new(2, 1).unwrap();
    g.metrics.boundary_integrity = 0;
    g.advance(PlayerAction::None); // ends the run
    assert!(g.status().is_terminal());
    let r = g.advance(PlayerAction::Audit);
    assert_eq!(r, ActionResult::RejectedTerminalState);
}

#[test]
fn rejected_actions_are_counted() {
    let mut g = GameState::new(1, 1).unwrap();
    let before = 0;
    for _ in 0..5 {
        g.advance(PlayerAction::Authorize); // invalid-for-event each time (no event early)
    }
    assert!(g.gate.rejected_actions_total > before);
}
