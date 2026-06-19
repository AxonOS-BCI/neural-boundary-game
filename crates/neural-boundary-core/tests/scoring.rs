// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
// See LICENSE and IP_NOTICE.md for details.

use neural_boundary_core::{
    compute_score, terminal_score_override, EndReason, Metrics, ScoreCounters, SCORE_MAX, SCORE_MIN,
};

#[test]
fn formula_matches_spec_5_4() {
    let m = Metrics::new(80, 70, 60, 50, 10, 5, 8, 40);
    let c = ScoreCounters {
        unresolved_critical_events: 1,
        unsafe_actions: 0,
        successful_audits: 3,
        correct_revocations: 2,
        clean_release_bonus: 0,
    };
    let rejected = 4u32;
    // Manual §5.4 computation.
    // boundary*3 + consent*2 + vault*2 + flow + audit
    //   - raw*3 - stim*3 - latency
    //   - unresolved*50 - unsafe*100 - rejected*5 + audits*12 + revokes*15 + bonus
    let expected_raw = 240 + 140 + 120 + 50 + 40 - 30 - 15 - 8 - 50 - 20 + 36 + 30;
    let expected = expected_raw.clamp(SCORE_MIN, SCORE_MAX);
    assert_eq!(compute_score(&m, &c, rejected), expected);
}

#[test]
fn score_is_clamped_high() {
    let m = Metrics::new(100, 100, 100, 100, 0, 0, 0, 100);
    let c = ScoreCounters {
        clean_release_bonus: 1000,
        ..ScoreCounters::default()
    };
    assert_eq!(compute_score(&m, &c, 0), SCORE_MAX);
}

#[test]
fn score_is_clamped_low() {
    let m = Metrics::new(0, 0, 0, 0, 100, 100, 100, 0);
    let c = ScoreCounters {
        unresolved_critical_events: 50,
        unsafe_actions: 50,
        ..ScoreCounters::default()
    };
    assert_eq!(compute_score(&m, &c, 100), SCORE_MIN);
}

#[test]
fn rejections_lower_score() {
    let m = Metrics::new(60, 60, 60, 60, 0, 0, 0, 60);
    let c = ScoreCounters::default();
    let base = compute_score(&m, &c, 0);
    let with_rejects = compute_score(&m, &c, 10);
    assert_eq!(base - with_rejects, 10 * 5);
}

#[test]
fn terminal_overrides_are_pinned() {
    assert_eq!(
        terminal_score_override(EndReason::BoundaryCollapse),
        Some(-1000)
    );
    assert_eq!(
        terminal_score_override(EndReason::RawLeakLimit),
        Some(-1000)
    );
    assert_eq!(
        terminal_score_override(EndReason::UnsafeStimulationEscape),
        Some(-1000)
    );
    assert_eq!(
        terminal_score_override(EndReason::ConsentCollapse),
        Some(-900)
    );
    assert_eq!(terminal_score_override(EndReason::VaultFailure), Some(-900));
    assert_eq!(terminal_score_override(EndReason::SuccessRelease), None);
    assert_eq!(terminal_score_override(EndReason::Timeout), None);
}
