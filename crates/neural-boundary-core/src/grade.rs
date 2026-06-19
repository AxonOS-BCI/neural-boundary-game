// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Tactical Rust/WASM Boundary Defense.
// See LICENSE and IP_NOTICE.md for details.

//! Grade system (TZ §3.8) and the release window gate (TZ §5.5).
//!
//! Grades are authoritative outcome tiers computed by the Rust core. The grade
//! tells the player *what happened*, not just pass/fail. Release-window opening
//! is a pure function of the boundary state so it can be unit-tested and proven.

use crate::hash::Fnv64;

/// Outcome tiers, best to worst (TZ §3.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Grade {
    /// S+ — perfect or near-perfect boundary defense.
    SovereignSeal = 0,
    /// S — strong defense; release safely completed.
    Sealed = 1,
    /// A — safe but imperfect; needs audit review.
    Reviewable = 2,
    /// B — system survived with damage.
    Contained = 3,
    /// C — boundary partially breached.
    Compromised = 4,
    /// F — terminal failure.
    Unsafe = 5,
}

impl Grade {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::SovereignSeal,
            1 => Self::Sealed,
            2 => Self::Reviewable,
            3 => Self::Contained,
            4 => Self::Compromised,
            5 => Self::Unsafe,
            _ => return None,
        })
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Short badge label shown on the proof screen (TZ §3.8).
    pub const fn label(self) -> &'static str {
        match self {
            Self::SovereignSeal => "S+",
            Self::Sealed => "S",
            Self::Reviewable => "A",
            Self::Contained => "B",
            Self::Compromised => "C",
            Self::Unsafe => "F",
        }
    }

    /// Full name shown beside the badge (TZ §3.8).
    pub const fn name(self) -> &'static str {
        match self {
            Self::SovereignSeal => "Sovereign Seal",
            Self::Sealed => "Sealed",
            Self::Reviewable => "Reviewable",
            Self::Contained => "Contained",
            Self::Compromised => "Compromised",
            Self::Unsafe => "Unsafe",
        }
    }

    /// True when the run ended in a player-positive outcome (released safely).
    pub const fn is_success(self) -> bool {
        matches!(self, Self::SovereignSeal | Self::Sealed | Self::Reviewable)
    }

    pub fn feed_hash(&self, h: &mut Fnv64) {
        h.feed_u8(self.code());
    }
}

/// Score thresholds for the released tiers (TZ §3.7/§3.8).
pub const SCORE_SOVEREIGN: i32 = 900;
pub const SCORE_SEALED: i32 = 700;
/// Core-metric floor for the S+ tier.
pub const SOVEREIGN_METRIC_FLOOR: u8 = 90;
/// Core-metric floor distinguishing "contained" from "compromised" on timeout.
pub const CONTAINED_METRIC_FLOOR: u8 = 50;

/// Inputs needed to assign a final grade. Keeping this a plain value type lets
/// the grading rule be tested without constructing a whole `GameState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GradeInputs {
    pub terminal_failure: bool,
    pub breached: bool,
    pub released_safely: bool,
    pub score: i32,
    pub boundary_integrity: u8,
    pub consent_coherence: u8,
    pub vault_integrity: u8,
}

/// Assign the final grade (TZ §3.8). Order is significant: terminal failure
/// dominates, then breach, then the released vs. survived split.
pub fn grade_for(g: GradeInputs) -> Grade {
    if g.terminal_failure {
        return Grade::Unsafe;
    }
    if !g.released_safely {
        // Survived to timeout without a safe release.
        return if g.breached {
            Grade::Compromised
        } else if min3(g.boundary_integrity, g.consent_coherence, g.vault_integrity)
            >= CONTAINED_METRIC_FLOOR
        {
            Grade::Contained
        } else {
            Grade::Compromised
        };
    }
    // Released safely — distinguish by quality.
    let floor = min3(g.boundary_integrity, g.consent_coherence, g.vault_integrity);
    if g.score >= SCORE_SOVEREIGN && floor >= SOVEREIGN_METRIC_FLOOR {
        Grade::SovereignSeal
    } else if g.score >= SCORE_SEALED {
        Grade::Sealed
    } else {
        Grade::Reviewable
    }
}

/// Boundary thresholds that gate the release window (TZ §5.5).
pub const RELEASE_CORE_FLOOR: u8 = 70;
pub const RELEASE_RISK_CEIL: u8 = 25;

/// Whether the release window is open (TZ §5.5). Pure and total so it can be a
/// Kani proof target later. Release is rejected by the core unless this holds.
#[allow(clippy::too_many_arguments)]
pub fn release_window_open(
    boundary_integrity: u8,
    consent_coherence: u8,
    vault_integrity: u8,
    raw_leak_risk: u8,
    stimulation_risk: u8,
    active_critical_threat: bool,
    proof_clean: bool,
) -> bool {
    boundary_integrity >= RELEASE_CORE_FLOOR
        && consent_coherence >= RELEASE_CORE_FLOOR
        && vault_integrity >= RELEASE_CORE_FLOOR
        && raw_leak_risk <= RELEASE_RISK_CEIL
        && stimulation_risk <= RELEASE_RISK_CEIL
        && !active_critical_threat
        && proof_clean
}

const fn min3(a: u8, b: u8, c: u8) -> u8 {
    let m = if a < b { a } else { b };
    if m < c {
        m
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> GradeInputs {
        GradeInputs {
            terminal_failure: false,
            breached: false,
            released_safely: true,
            score: 1000,
            boundary_integrity: 100,
            consent_coherence: 100,
            vault_integrity: 100,
        }
    }

    #[test]
    fn code_roundtrips() {
        for v in 0..=5u8 {
            assert_eq!(Grade::from_u8(v).unwrap().code(), v);
        }
        assert_eq!(Grade::from_u8(6), None);
    }

    #[test]
    fn labels_are_spec_badges() {
        assert_eq!(Grade::SovereignSeal.label(), "S+");
        assert_eq!(Grade::Unsafe.label(), "F");
        assert_eq!(Grade::SovereignSeal.name(), "Sovereign Seal");
    }

    #[test]
    fn terminal_failure_is_f() {
        let g = GradeInputs {
            terminal_failure: true,
            ..base()
        };
        assert_eq!(grade_for(g), Grade::Unsafe);
    }

    #[test]
    fn perfect_release_is_s_plus() {
        assert_eq!(grade_for(base()), Grade::SovereignSeal);
    }

    #[test]
    fn strong_release_is_s() {
        let g = GradeInputs {
            score: 750,
            boundary_integrity: 80,
            consent_coherence: 80,
            vault_integrity: 80,
            ..base()
        };
        assert_eq!(grade_for(g), Grade::Sealed);
    }

    #[test]
    fn imperfect_release_is_a() {
        let g = GradeInputs {
            score: 400,
            ..base()
        };
        assert_eq!(grade_for(g), Grade::Reviewable);
    }

    #[test]
    fn timeout_with_health_is_contained_else_compromised() {
        let survived = GradeInputs {
            released_safely: false,
            score: 300,
            ..base()
        };
        assert_eq!(grade_for(survived), Grade::Contained);
        let damaged = GradeInputs {
            released_safely: false,
            boundary_integrity: 30,
            consent_coherence: 30,
            vault_integrity: 30,
            ..base()
        };
        assert_eq!(grade_for(damaged), Grade::Compromised);
        let breached = GradeInputs {
            released_safely: false,
            breached: true,
            ..base()
        };
        assert_eq!(grade_for(breached), Grade::Compromised);
    }

    #[test]
    fn release_window_respects_all_thresholds() {
        // All conditions met -> open.
        assert!(release_window_open(70, 70, 70, 25, 25, false, true));
        // Any single failing condition -> closed.
        assert!(!release_window_open(69, 70, 70, 25, 25, false, true));
        assert!(!release_window_open(70, 70, 70, 26, 25, false, true));
        assert!(!release_window_open(70, 70, 70, 25, 25, true, true));
        assert!(!release_window_open(70, 70, 70, 25, 25, false, false));
    }
}
