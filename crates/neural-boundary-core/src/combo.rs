// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Tactical Rust/WASM Boundary Defense.
// See LICENSE and IP_NOTICE.md for details.

//! Action combos (TZ §3.6) — deterministic two-step tactical sequences.
//!
//! A combo completes when a second accepted action follows the right first
//! action within the combo window. Combos live in the Rust core, are fully
//! deterministic, and the combo window is folded into `compute_hash` (hash v4).

use crate::actions::PlayerAction;
use crate::hash::Fnv64;

/// Combo window length in ticks. TZ §3.6 requires 6–12; we fix the midpoint so
/// the value is deterministic and identical across compilers and architectures.
pub const COMBO_WINDOW_TICKS: u8 = 8;

/// The five tactical combos plus the `None` sentinel (TZ §3.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ComboKind {
    None = 0,
    /// Audit -> Authorize: safe typed intent when audit confidence is high.
    AuditAuthorize = 1,
    /// Quarantine -> SealVault: strong raw-exposure containment.
    QuarantineSeal = 2,
    /// Throttle -> Audit: stabilize a high-pressure state before analysis.
    ThrottleAudit = 3,
    /// Revoke -> Quarantine: strong defense against shadow-app probes.
    RevokeQuarantine = 4,
    /// Audit -> Release: raises proof quality if the release window is open.
    AuditRelease = 5,
}

impl ComboKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::None,
            1 => Self::AuditAuthorize,
            2 => Self::QuarantineSeal,
            3 => Self::ThrottleAudit,
            4 => Self::RevokeQuarantine,
            5 => Self::AuditRelease,
            _ => return None,
        })
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::AuditAuthorize => "AUDIT_AUTHORIZE",
            Self::QuarantineSeal => "QUARANTINE_SEAL",
            Self::ThrottleAudit => "THROTTLE_AUDIT",
            Self::RevokeQuarantine => "REVOKE_QUARANTINE",
            Self::AuditRelease => "AUDIT_RELEASE",
        }
    }

    /// Player-facing combo name for the UI flash (TZ §3.6).
    pub const fn title(self) -> &'static str {
        match self {
            Self::None => "",
            Self::AuditAuthorize => "Verified Intent",
            Self::QuarantineSeal => "Containment Seal",
            Self::ThrottleAudit => "Stabilize & Analyze",
            Self::RevokeQuarantine => "Probe Lockout",
            Self::AuditRelease => "Proven Release",
        }
    }

    /// Detect a combo from the previous and current accepted action (TZ §3.6).
    pub fn detect(prev: PlayerAction, cur: PlayerAction) -> Self {
        use PlayerAction::*;
        match (prev, cur) {
            (Audit, Authorize) => Self::AuditAuthorize,
            (Quarantine, SealVault) => Self::QuarantineSeal,
            (Throttle, Audit) => Self::ThrottleAudit,
            (Revoke, Quarantine) => Self::RevokeQuarantine,
            (Audit, Release) => Self::AuditRelease,
            _ => Self::None,
        }
    }

    /// Score bonus awarded when this combo completes (TZ §3.7).
    pub const fn score_bonus(self) -> i32 {
        match self {
            Self::None => 0,
            Self::AuditAuthorize => 40,
            Self::QuarantineSeal => 60,
            Self::ThrottleAudit => 35,
            Self::RevokeQuarantine => 55,
            Self::AuditRelease => 50,
        }
    }

    pub fn feed_hash(&self, h: &mut Fnv64) {
        h.feed_u8(self.code());
    }
}

/// Trailing accepted action within the combo window (TZ §3.6). Hashed in full.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ComboWindow {
    /// `PlayerAction` code of the last accepted action; 0 = none in window.
    pub prev_action: u8,
    /// Ticks remaining in which a follow-up can complete a combo.
    pub ticks_left: u8,
    /// `ComboKind` code of the most recently completed combo (0 = none).
    pub last_combo: u8,
}

impl ComboWindow {
    /// Advance one tick; the window closes (and clears) when it expires.
    pub fn tick(&mut self) {
        if self.ticks_left > 0 {
            self.ticks_left -= 1;
            if self.ticks_left == 0 {
                self.prev_action = 0;
            }
        }
    }

    /// Register an accepted action; returns the combo completed, if any. The
    /// window then reopens around the just-played action for chaining.
    pub fn on_action(&mut self, action: PlayerAction) -> ComboKind {
        let prev = PlayerAction::from_u8(self.prev_action).unwrap_or(PlayerAction::None);
        let combo = if self.ticks_left > 0 {
            ComboKind::detect(prev, action)
        } else {
            ComboKind::None
        };
        self.last_combo = combo.code();
        self.prev_action = action.code();
        self.ticks_left = COMBO_WINDOW_TICKS;
        combo
    }

    /// The combo completed on the most recent action (for UI / exports).
    pub fn last(&self) -> ComboKind {
        ComboKind::from_u8(self.last_combo).unwrap_or(ComboKind::None)
    }

    pub fn feed_hash(&self, h: &mut Fnv64) {
        h.feed_u8(self.prev_action);
        h.feed_u8(self.ticks_left);
        h.feed_u8(self.last_combo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_roundtrips() {
        for v in 0..=5u8 {
            assert_eq!(ComboKind::from_u8(v).unwrap().code(), v);
        }
        assert_eq!(ComboKind::from_u8(6), None);
    }

    #[test]
    fn detect_matches_all_five_combos() {
        use PlayerAction::*;
        assert_eq!(
            ComboKind::detect(Audit, Authorize),
            ComboKind::AuditAuthorize
        );
        assert_eq!(
            ComboKind::detect(Quarantine, SealVault),
            ComboKind::QuarantineSeal
        );
        assert_eq!(ComboKind::detect(Throttle, Audit), ComboKind::ThrottleAudit);
        assert_eq!(
            ComboKind::detect(Revoke, Quarantine),
            ComboKind::RevokeQuarantine
        );
        assert_eq!(ComboKind::detect(Audit, Release), ComboKind::AuditRelease);
    }

    #[test]
    fn non_combo_pairs_are_none() {
        use PlayerAction::*;
        assert_eq!(ComboKind::detect(Audit, Audit), ComboKind::None);
        assert_eq!(ComboKind::detect(SealVault, Quarantine), ComboKind::None);
        assert_eq!(ComboKind::detect(None, Authorize), ComboKind::None);
    }

    #[test]
    fn window_completes_combo_then_reopens() {
        let mut w = ComboWindow::default();
        // First action: nothing to combo with yet.
        assert_eq!(w.on_action(PlayerAction::Audit), ComboKind::None);
        assert_eq!(w.ticks_left, COMBO_WINDOW_TICKS);
        // Follow-up within window: combo completes.
        assert_eq!(
            w.on_action(PlayerAction::Authorize),
            ComboKind::AuditAuthorize
        );
        assert_eq!(w.last(), ComboKind::AuditAuthorize);
    }

    #[test]
    fn expired_window_yields_no_combo() {
        let mut w = ComboWindow::default();
        assert_eq!(w.on_action(PlayerAction::Quarantine), ComboKind::None);
        // Let the window expire.
        for _ in 0..COMBO_WINDOW_TICKS {
            w.tick();
        }
        assert_eq!(w.ticks_left, 0);
        assert_eq!(w.prev_action, 0);
        // Follow-up after expiry: no combo.
        assert_eq!(w.on_action(PlayerAction::SealVault), ComboKind::None);
    }

    #[test]
    fn hash_is_stable_for_equal_windows() {
        let mut a = ComboWindow::default();
        let mut b = ComboWindow::default();
        a.on_action(PlayerAction::Throttle);
        b.on_action(PlayerAction::Throttle);
        let mut ha = Fnv64::new();
        let mut hb = Fnv64::new();
        a.feed_hash(&mut ha);
        b.feed_hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }
}
