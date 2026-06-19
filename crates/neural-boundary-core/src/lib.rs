// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.0.1).
// See LICENSE and IP_NOTICE.md for details.

//! `neural-boundary-core` — Cognitive Sovereignty Console deterministic engine
//! (v8.0.1). Canonical source of truth (§16.2): the browser never computes
//! grade, score, or state hash independently.
//!
//! Module layout follows §25:
//! `hash` (RNG + fnv1a64), `metrics`, `events`, `actions`, `scenario`,
//! `scoring`, `state`, `replay`.
//!
//! Stable Rust only — no nightly features, no `RUSTC_BOOTSTRAP` (§16.3, §28).

#![no_std]
#![forbid(unsafe_code)]

pub mod actions;
pub mod combo;
pub mod events;
pub mod grade;
pub mod hash;
pub mod metrics;
pub mod proof;
pub mod replay;
pub mod scenario;
pub mod scoring;
pub mod state;
pub mod threat;

// ── Identity (§0, §17.1) ─────────────────────────────────────────────────────

pub const CORE_VERSION: &str = "8.0.1";
pub const CODENAME: &str = "Cognitive Sovereignty Console";
pub const STYLE_SYSTEM: &str = "AxonOS Standard Foundation Grande Style Elite";
pub const REPLAY_SCHEMA: &str = "neural-boundary-replay-v4";

/// ABI v3 (§17.1).
pub const ABI_VERSION: u32 = 4;
/// (major<<16)|(minor<<8)|patch == 0x080001 == 524289 (§17.1).
pub const PRODUCT_VERSION_PACKED: u32 = (8 << 16) | 1;

// ── Fixed bounds (§18) ───────────────────────────────────────────────────────

pub const MAX_ACTIVE_EVENTS: usize = 8;
pub const MAX_PERMISSION_SLOTS: usize = 16;
pub const TIMELINE_RING_SIZE: usize = 256;
pub const MAX_SCENARIOS: usize = 16;

// ── Re-exports ───────────────────────────────────────────────────────────────

pub use actions::{ActionGate, ActionResult, PlayerAction};
pub use events::{EventKind, NeuralEvent, PermissionScope};
pub use hash::{daily_seed, Fnv64, Rng, HASH_ALGORITHM, RNG_ALGORITHM};
pub use metrics::{clamp_metric, Metrics};
pub use replay::{
    outcome, run as replay_run, run_builtin, verify as verify_vector, BuiltinVector,
    ExpectedOutcome, ReplayAction, VectorOutcome, BUILTIN_VECTORS,
};
pub use scenario::{
    scenario_by_id, Difficulty, Scenario, ScheduledEvent, SuccessThresholds, SCENARIOS,
    SCENARIO_COUNT, TICK_RATE_HZ,
};
pub use scoring::{compute_score, terminal_score_override, ScoreCounters, SCORE_MAX, SCORE_MIN};
pub use state::{
    EndReason, GameState, PermissionState, PrivacyVaultState, RunStatus, SovereigntyGrade,
    StimulationState,
};

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn version_constants_match_spec() {
        assert_eq!(PRODUCT_VERSION_PACKED, 0x080001);
        assert_eq!(PRODUCT_VERSION_PACKED, 524289);
        assert_eq!(ABI_VERSION, 4);
        assert_eq!(CORE_VERSION, "8.0.1");
        assert_eq!(REPLAY_SCHEMA, "neural-boundary-replay-v4");
        assert_eq!(TICK_RATE_HZ, 20);
    }

    #[test]
    fn nine_scenarios_with_ascending_ids() {
        assert_eq!(SCENARIO_COUNT, 9);
        assert_eq!(SCENARIOS.len(), 9);
        for (i, s) in SCENARIOS.iter().enumerate() {
            assert_eq!(s.id, (i as u32) + 1);
            assert!(scenario_by_id(s.id).is_some());
            // Each schedule must fit within the run budget.
            for e in s.event_schedule {
                assert!(
                    e.at_tick < s.max_ticks,
                    "scenario {} event past max_ticks",
                    s.id
                );
            }
        }
    }

    #[test]
    fn bounds_constants() {
        assert_eq!(MAX_ACTIVE_EVENTS, 8);
        assert_eq!(MAX_PERMISSION_SLOTS, 16);
        assert_eq!(TIMELINE_RING_SIZE, 256);
        assert_eq!(MAX_SCENARIOS, 16);
    }
}
