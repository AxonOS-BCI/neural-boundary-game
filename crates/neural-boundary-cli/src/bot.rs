// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
// See LICENSE and IP_NOTICE.md for details.

//! Deterministic reference policies used to record canonical replay vectors.
//! The bot only ever calls `GameState::advance`, so any run it produces is a
//! valid, reproducible action stream.

use neural_boundary_core::{EventKind, GameState, PlayerAction, ReplayAction, RunStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Disciplined sovereignty play: audit, guard, contain, release clean.
    Clean,
    /// Do nothing — let the run time out.
    Idle,
    /// Authorize everything blindly — provoke breaches and unsafe escapes.
    Lapse,
}

impl Policy {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "clean" => Self::Clean,
            "idle" => Self::Idle,
            "lapse" => Self::Lapse,
            _ => return None,
        })
    }
}

fn choose_clean(g: &GameState) -> PlayerAction {
    if let Some(ev) = g.focused_event() {
        // Reveal hidden risk before deciding.
        if ev.requires_audit && !ev.audited {
            return PlayerAction::Audit;
        }
        match ev.kind {
            EventKind::UnsafeStimulation => {
                if !g.stimulation.throttled {
                    PlayerAction::Throttle
                } else {
                    PlayerAction::Quarantine
                }
            }
            EventKind::RawSignalExposure => {
                if !g.vault.sealed && g.vault.capacity > 0 {
                    PlayerAction::SealVault
                } else {
                    PlayerAction::Quarantine
                }
            }
            EventKind::PermissionEscalation | EventKind::AdversarialProbe => {
                PlayerAction::Quarantine
            }
            EventKind::LatencySpike | EventKind::VaultPressure => PlayerAction::Quarantine,
            EventKind::ConsentRequest | EventKind::BenignFlow => PlayerAction::Authorize,
            EventKind::StimulationRequest => {
                if g.stimulation.throttled {
                    PlayerAction::Authorize
                } else {
                    PlayerAction::Throttle
                }
            }
            EventKind::AuditCheckpoint => PlayerAction::Audit,
        }
    } else if g.permissions.has_sensitive() {
        // Drop stale sensitive scope (counts as a correct revocation).
        PlayerAction::Revoke
    } else if g.release_available() {
        PlayerAction::Release
    } else {
        PlayerAction::None
    }
}

fn choose(g: &GameState, policy: Policy) -> PlayerAction {
    match policy {
        Policy::Idle => PlayerAction::None,
        Policy::Lapse => {
            if g.focused_event().is_some() {
                PlayerAction::Authorize
            } else {
                PlayerAction::None
            }
        }
        Policy::Clean => choose_clean(g),
    }
}

/// Run a scenario under a policy to termination, recording the action stream.
pub fn play(scenario_id: u32, seed: u64, policy: Policy) -> Option<(Vec<ReplayAction>, GameState)> {
    let mut g = GameState::new(scenario_id, seed)?;
    let mut actions: Vec<ReplayAction> = Vec::new();
    let cap = 3600u32 + 64;
    while g.status() == RunStatus::Running && g.tick() < cap {
        let action = choose(&g, policy);
        let at_tick = g.tick() + 1;
        g.advance(action);
        if action != PlayerAction::None {
            actions.push(ReplayAction {
                tick: at_tick,
                action,
            });
        }
    }
    Some((actions, g))
}
