// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
// See LICENSE and IP_NOTICE.md for details.

//! Player actions, the action gate, and action results (§4.3, §4.4).

/// Player actions (§4.3). `None` is the per-tick no-action sentinel used by the
/// replay runner; `Pause` is presentation-only and never reaches the core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerAction {
    None = 0,
    Audit = 1,
    Authorize = 2,
    Revoke = 3,
    Quarantine = 4,
    SealVault = 5,
    Throttle = 6,
    Release = 7,
}

impl PlayerAction {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::None,
            1 => Self::Audit,
            2 => Self::Authorize,
            3 => Self::Revoke,
            4 => Self::Quarantine,
            5 => Self::SealVault,
            6 => Self::Throttle,
            7 => Self::Release,
            _ => return None,
        })
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "NONE" | "None" => Self::None,
            "AUDIT" | "Audit" => Self::Audit,
            "AUTHORIZE" | "Authorize" => Self::Authorize,
            "REVOKE" | "Revoke" => Self::Revoke,
            "QUARANTINE" | "Quarantine" => Self::Quarantine,
            "SEAL_VAULT" | "SealVault" => Self::SealVault,
            "THROTTLE" | "Throttle" => Self::Throttle,
            "RELEASE" | "Release" => Self::Release,
            _ => return None,
        })
    }
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Audit => "AUDIT",
            Self::Authorize => "AUTHORIZE",
            Self::Revoke => "REVOKE",
            Self::Quarantine => "QUARANTINE",
            Self::SealVault => "SEAL_VAULT",
            Self::Throttle => "THROTTLE",
            Self::Release => "RELEASE",
        }
    }
}

/// Result of an attempted action (§4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ActionResult {
    Accepted = 0,
    RejectedCooldown = 1,
    RejectedTerminalState = 2,
    RejectedInvalidForEvent = 3,
    RejectedReleaseLocked = 4,
    NoOp = 5,
}

impl ActionResult {
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "ACCEPTED",
            Self::RejectedCooldown => "REJECTED_COOLDOWN",
            Self::RejectedTerminalState => "REJECTED_TERMINAL_STATE",
            Self::RejectedInvalidForEvent => "REJECTED_INVALID_FOR_EVENT",
            Self::RejectedReleaseLocked => "REJECTED_RELEASE_LOCKED",
            Self::NoOp => "NO_OP",
        }
    }
    pub const fn is_rejected(self) -> bool {
        matches!(
            self,
            Self::RejectedCooldown
                | Self::RejectedTerminalState
                | Self::RejectedInvalidForEvent
                | Self::RejectedReleaseLocked
        )
    }
}

/// Action gate (§4.4): max 1 action per tick, 1-tick cooldown, duplicate reject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ActionGate {
    pub last_action_tick: u32,
    pub accepted_actions_this_tick: u8,
    pub rejected_actions_total: u32,
    pub has_acted: bool,
}

pub const MAX_ACTIONS_PER_TICK: u8 = 1;
pub const ACTION_COOLDOWN_TICKS: u32 = 1;

impl ActionGate {
    /// Reset the per-tick acceptance counter at the start of each tick.
    pub fn begin_tick(&mut self) {
        self.accepted_actions_this_tick = 0;
    }

    /// Decide whether a real (non-None) action may be accepted this tick.
    /// Returns `None` if it passes the gate, or `Some(reject)` otherwise.
    pub fn gate(&self, tick: u32) -> Option<ActionResult> {
        if self.accepted_actions_this_tick >= MAX_ACTIONS_PER_TICK {
            return Some(ActionResult::RejectedCooldown);
        }
        if self.has_acted && tick < self.last_action_tick + ACTION_COOLDOWN_TICKS {
            return Some(ActionResult::RejectedCooldown);
        }
        None
    }

    pub fn record_accept(&mut self, tick: u32) {
        self.last_action_tick = tick;
        self.accepted_actions_this_tick = self.accepted_actions_this_tick.saturating_add(1);
        self.has_acted = true;
    }

    pub fn record_reject(&mut self) {
        self.rejected_actions_total = self.rejected_actions_total.saturating_add(1);
    }
}
