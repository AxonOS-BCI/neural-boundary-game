// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.0.1).
// See LICENSE and IP_NOTICE.md for details.

//! WASM ABI v3 conformance (§17). These run on the host via the `rlib`
//! crate type; the same symbols are exported from the `cdylib` for wasm32.

use neural_boundary_core::{GameState, PlayerAction};
use neural_boundary_web::*;

#[test]
fn health_values_match_spec_17() {
    assert_eq!(nbg_abi_version(), 4);
    assert_eq!(nbg_product_version_packed(), 524_289);
    assert_eq!(nbg_health_check(), 524_289);
    assert_eq!(nbg_health_check(), nbg_product_version_packed());
    assert_eq!(nbg_tick_rate(), 20);
}

#[test]
fn lifecycle_and_scenario_metadata() {
    assert_eq!(nbg_scenario_count(), 9);
    assert_eq!(nbg_new(1, 0, 1), 1);
    assert_eq!(nbg_is_initialized(), 1);
    assert_eq!(nbg_scenario_id(), 1);
    assert_eq!(nbg_tick_count(), 0);
    // Unknown scenario fails to start.
    assert_eq!(nbg_new(99, 0, 1), 0);
}

#[test]
fn tick_advances_and_metrics_readable() {
    assert_eq!(nbg_new(2, 0, 7), 1);
    let before = nbg_tick_count();
    nbg_set_action(PlayerAction::Audit.code() as u32);
    let _ = nbg_tick();
    assert_eq!(nbg_tick_count(), before + 1);
    // Metrics are within range.
    assert!(nbg_boundary_integrity() <= 100);
    assert!(nbg_raw_leak_risk() <= 100);
    assert!(nbg_audit_confidence() <= 100);
}

#[test]
fn abi_matches_core_hash_exactly() {
    // Drive the ABI and a core GameState with the identical action stream and
    // confirm the authoritative state hash is identical (no JS-only logic).
    assert_eq!(nbg_new(4, 0, 4), 1);
    let mut g = GameState::new(4, 4).unwrap();
    let pattern = [6u32, 1, 0, 4, 0, 0, 3, 0]; // Throttle, Audit, None, Quarantine, ...
    for (i, _) in (0..400).enumerate() {
        let code = pattern[i % pattern.len()];
        let _ = nbg_step(code);
        g.advance(PlayerAction::from_u8(code as u8).unwrap_or(PlayerAction::None));
        if nbg_is_terminal() == 1 {
            break;
        }
    }
    let abi_hash = ((nbg_state_hash_hi() as u64) << 32) | (nbg_state_hash_lo() as u64);
    assert_eq!(abi_hash, g.state_hash());
    assert_eq!(nbg_grade(), g.grade().code() as u32);
    assert_eq!(nbg_score(), g.score());
}

#[test]
fn labels_are_nonempty_after_run() {
    assert_eq!(nbg_new(1, 0, 1), 1);
    assert!(nbg_grade_label_len() > 0);
    assert!(nbg_grade_public_label_len() > 0);
    assert!(nbg_scenario_name_len(1) > 0);
    assert!(nbg_scenario_objective_len(9) > 0);
    // Unknown scenario name is empty.
    assert_eq!(nbg_scenario_name_len(99), 0);
}

#[test]
fn builtin_vectors_runnable() {
    assert_eq!(nbg_builtin_vector_count(), 3);
    let grade = nbg_run_builtin_vector(0);
    assert!(grade <= 4); // valid grade code
                         // Hash populated.
    let h = ((nbg_vector_hash_hi() as u64) << 32) | (nbg_vector_hash_lo() as u64);
    assert_ne!(h, 0);
    // Out-of-range index → sentinel.
    assert_eq!(nbg_run_builtin_vector(999), 0xFFFF_FFFF);
}

#[test]
fn release_blockers_exposed() {
    assert_eq!(nbg_new(9, 0, 1), 1);
    // Early on, release is locked and blockers are nonzero.
    let _ = nbg_step(PlayerAction::None.code() as u32);
    assert_eq!(nbg_release_available(), 0);
    assert_ne!(nbg_release_blockers(), 0);
}
