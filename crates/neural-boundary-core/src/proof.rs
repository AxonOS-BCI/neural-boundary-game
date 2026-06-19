// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Tactical Rust/WASM Boundary Defense.
// See LICENSE and IP_NOTICE.md for details.

//! Replay proof status (TZ §2.3, §5.6).
//!
//! Determinism is a gameplay feature: a run's authoritative proof is
//! `scenario_id + seed + action timeline -> Rust replay -> grade/score/hash`.
//! `ProofStatus` reflects whether that proof chain is intact for the run.

use crate::hash::Fnv64;

/// State of the replay proof chain for the current run (TZ §5.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ProofStatus {
    /// Action stream is consistent and the replay hash chain is intact.
    Clean = 0,
    /// Run in progress; proof not yet finalized.
    Pending = 1,
    /// Action stream anomaly or replay hash mismatch detected.
    Tampered = 2,
}

impl ProofStatus {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Clean,
            1 => Self::Pending,
            2 => Self::Tampered,
            _ => return None,
        })
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Clean => "CLEAN",
            Self::Pending => "PENDING",
            Self::Tampered => "TAMPERED",
        }
    }

    /// Release requires a non-tampered proof chain (TZ §5.5).
    pub const fn is_clean(self) -> bool {
        matches!(self, Self::Clean)
    }

    pub fn feed_hash(&self, h: &mut Fnv64) {
        h.feed_u8(self.code());
    }
}

#[cfg(test)]
#[allow(clippy::derivable_impls)]
impl Default for ProofStatus {
    fn default() -> Self {
        Self::Pending
    }
}

mod tests {

    #[test]
    fn code_roundtrips() {
        for v in 0..=2u8 {
            assert_eq!(ProofStatus::from_u8(v).unwrap().code(), v);
        }
        assert_eq!(ProofStatus::from_u8(3), None);
    }

    #[test]
    fn default_is_pending() {
        assert_eq!(ProofStatus::default(), ProofStatus::Pending);
    }

    #[test]
    fn only_clean_is_clean() {
        assert!(ProofStatus::Clean.is_clean());
        assert!(!ProofStatus::Pending.is_clean());
        assert!(!ProofStatus::Tampered.is_clean());
    }
}
