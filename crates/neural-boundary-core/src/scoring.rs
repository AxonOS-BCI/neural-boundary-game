// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
// See LICENSE and IP_NOTICE.md for details.

//! Deterministic scoring (§5.4) and terminal score overrides (§5.2).
//! No floating point (§16.3).

use crate::metrics::Metrics;
use crate::state::EndReason;

pub const SCORE_MIN: i32 = -1000;
pub const SCORE_MAX: i32 = 1500;

/// Non-metric score contributors accumulated over a run (§5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ScoreCounters {
    pub unresolved_critical_events: u32,
    pub unsafe_actions: u32,
    pub successful_audits: u32,
    pub correct_revocations: u32,
    /// PROVISIONAL §balance — applied on a clean RELEASE.
    pub clean_release_bonus: i32,
}

/// §5.4 score formula. `rejected_actions_total` comes from the action gate.
/// Result is clamped to `[SCORE_MIN, SCORE_MAX]`.
pub fn compute_score(m: &Metrics, c: &ScoreCounters, rejected_actions_total: u32) -> i32 {
    let raw = (m.boundary_integrity as i32) * 3
        + (m.consent_coherence as i32) * 2
        + (m.vault_integrity as i32) * 2
        + (m.cognitive_flow as i32)
        + (m.audit_confidence as i32)
        - (m.raw_leak_risk as i32) * 3
        - (m.stimulation_risk as i32) * 3
        - (m.latency_pressure as i32)
        - (c.unresolved_critical_events as i32) * 50
        - (c.unsafe_actions as i32) * 100
        - (rejected_actions_total as i32) * 5
        + (c.successful_audits as i32) * 12
        + (c.correct_revocations as i32) * 15
        + c.clean_release_bonus;
    raw.clamp(SCORE_MIN, SCORE_MAX)
}

/// §5.2 terminal score override. Terminal failure bypasses normal scoring.
pub fn terminal_score_override(reason: EndReason) -> Option<i32> {
    Some(match reason {
        EndReason::BoundaryCollapse => -1000,
        EndReason::RawLeakLimit => -1000,
        EndReason::UnsafeStimulationEscape => -1000,
        EndReason::ConsentCollapse => -900,
        EndReason::VaultFailure => -900,
        _ => return None,
    })
}
