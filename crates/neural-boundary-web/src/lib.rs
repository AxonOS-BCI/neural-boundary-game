// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
// See LICENSE and IP_NOTICE.md for details.

//! Flat WASM ABI v3 (§17). Eighty-plus C-ABI exports over the canonical core.
//! No wasm-bindgen; `#[unsafe(no_mangle)] extern "C"` (stable since Rust 1.82).
//! The core crate keeps `#![forbid(unsafe_code)]`; this thin ABI shim does not.
//!
//! Trust boundary (§16.2, §20): the browser reads state through these getters
//! and never computes grade, score, or hash. All authoritative logic is in
//! `neural-boundary-core`.

#![allow(unsafe_code)]

use core::cell::{Cell, RefCell};
use neural_boundary_core::{
    scenario_by_id, ActionResult, GameState, PlayerAction, VectorOutcome, ABI_VERSION,
    BUILTIN_VECTORS, PRODUCT_VERSION_PACKED, SCENARIO_COUNT, TICK_RATE_HZ,
};

thread_local! {
    static STATE: RefCell<Option<GameState>> = const { RefCell::new(None) };
    static LAST: Cell<(u32, u64)> = const { Cell::new((1, 1)) };
    static PENDING: Cell<u8> = const { Cell::new(0) };
    static VECTOR: Cell<Option<VectorOutcome>> = const { Cell::new(None) };
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn read<R>(default: R, f: impl FnOnce(&GameState) -> R) -> R {
    STATE.with(|c| match c.borrow().as_ref() {
        Some(g) => f(g),
        None => default,
    })
}

fn write<R>(default: R, f: impl FnOnce(&mut GameState) -> R) -> R {
    STATE.with(|c| match c.borrow_mut().as_mut() {
        Some(g) => f(g),
        None => default,
    })
}

#[allow(clippy::cast_possible_truncation)]
fn ptr_of(s: &str) -> u32 {
    s.as_ptr() as usize as u32
}

#[allow(clippy::cast_possible_truncation)]
fn len_of(s: &str) -> u32 {
    s.len() as u32
}

fn start(scenario_id: u32, seed: u64) -> u32 {
    match GameState::new(scenario_id, seed) {
        Some(g) => {
            STATE.with(|c| *c.borrow_mut() = Some(g));
            LAST.with(|l| l.set((scenario_id, seed)));
            PENDING.with(|p| p.set(0));
            1
        }
        None => 0,
    }
}

// ── version / health (§17.3) ─────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_abi_version() -> u32 {
    ABI_VERSION
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_product_version_packed() -> u32 {
    PRODUCT_VERSION_PACKED
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_health_check() -> u32 {
    PRODUCT_VERSION_PACKED
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_tick_rate() -> u32 {
    TICK_RATE_HZ
}

// ── lifecycle ────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_new(scenario_id: u32, seed_hi: u32, seed_lo: u32) -> u32 {
    let seed = ((seed_hi as u64) << 32) | (seed_lo as u64);
    start(scenario_id, seed)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_reset() -> u32 {
    let (sid, seed) = LAST.with(|l| l.get());
    start(sid, seed)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_reset_with_seed(seed_hi: u32, seed_lo: u32) -> u32 {
    let sid = LAST.with(|l| l.get().0);
    let seed = ((seed_hi as u64) << 32) | (seed_lo as u64);
    start(sid, seed)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_set_scenario(scenario_id: u32) -> u32 {
    let seed = LAST.with(|l| l.get().1);
    start(scenario_id, seed)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_is_initialized() -> u32 {
    read(0, |_| 1)
}

// ── tick / action ────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_set_action(action_code: u32) -> u32 {
    let code = u8::try_from(action_code)
        .ok()
        .filter(|c| *c <= 7)
        .unwrap_or(0);
    PENDING.with(|p| p.set(code));
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_pending_action() -> u32 {
    PENDING.with(|p| p.get()) as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_tick() -> u32 {
    let code = PENDING.with(|p| {
        let c = p.get();
        p.set(0);
        c
    });
    let action = PlayerAction::from_u8(code).unwrap_or(PlayerAction::None);
    write(ActionResult::NoOp.code() as u32, |g| {
        g.advance(action).code() as u32
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_step(action_code: u32) -> u32 {
    let action = u8::try_from(action_code)
        .ok()
        .and_then(PlayerAction::from_u8)
        .unwrap_or(PlayerAction::None);
    write(ActionResult::NoOp.code() as u32, |g| {
        g.advance(action).code() as u32
    })
}

// ── status / grade ───────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_status() -> u32 {
    read(0, |g| g.status().code() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_is_terminal() -> u32 {
    read(0, |g| g.status().is_terminal() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_grade() -> u32 {
    read(0, |g| g.grade().code() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_end_reason() -> u32 {
    read(0, |g| g.end_reason().code() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_score() -> i32 {
    read(0, |g| g.score())
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_tick_count() -> u32 {
    read(0, |g| g.tick())
}

// ── metrics ──────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_boundary_integrity() -> u32 {
    read(0, |g| g.metrics().boundary_integrity as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_consent_coherence() -> u32 {
    read(0, |g| g.metrics().consent_coherence as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_vault_integrity() -> u32 {
    read(0, |g| g.metrics().vault_integrity as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_cognitive_flow() -> u32 {
    read(0, |g| g.metrics().cognitive_flow as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_raw_leak_risk() -> u32 {
    read(0, |g| g.metrics().raw_leak_risk as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_stimulation_risk() -> u32 {
    read(0, |g| g.metrics().stimulation_risk as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_latency_pressure() -> u32 {
    read(0, |g| g.metrics().latency_pressure as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_audit_confidence() -> u32 {
    read(0, |g| g.metrics().audit_confidence as u32)
}

// ── active event (focused) ───────────────────────────────────────────────────

const NO_EVENT: u32 = 0xFFFF_FFFF;

#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_count() -> u32 {
    read(0, |g| g.active_event_count() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_present() -> u32 {
    read(0, |g| g.focused_event().is_some() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_id() -> u32 {
    read(NO_EVENT, |g| {
        g.focused_event().map(|e| e.id).unwrap_or(NO_EVENT)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_kind() -> u32 {
    read(NO_EVENT, |g| {
        g.focused_event()
            .map(|e| e.kind.code() as u32)
            .unwrap_or(NO_EVENT)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_severity() -> u32 {
    read(0, |g| {
        g.focused_event().map(|e| e.severity as u32).unwrap_or(0)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_visible_risk() -> u32 {
    read(0, |g| {
        g.focused_event()
            .map(|e| e.visible_risk as u32)
            .unwrap_or(0)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_perceived_risk() -> u32 {
    read(0, |g| {
        g.focused_event()
            .map(|e| e.perceived_risk() as u32)
            .unwrap_or(0)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_scope() -> u32 {
    read(0, |g| {
        g.focused_event()
            .map(|e| e.permission_scope.bits() as u32)
            .unwrap_or(0)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_requires_audit() -> u32 {
    read(0, |g| {
        g.focused_event()
            .map(|e| e.requires_audit as u32)
            .unwrap_or(0)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_audited() -> u32 {
    read(0, |g| {
        g.focused_event().map(|e| e.audited as u32).unwrap_or(0)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_expires_at() -> u32 {
    read(0, |g| {
        g.focused_event().map(|e| e.expires_at_tick).unwrap_or(0)
    })
}

// ── permission / vault / stimulation ─────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_permission_scopes() -> u32 {
    read(0, |g| g.permissions.active_scopes.bits() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_permission_count() -> u32 {
    read(0, |g| g.permissions.count as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_permission_sensitive() -> u32 {
    read(0, |g| g.permissions.has_sensitive() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_vault_sealed() -> u32 {
    read(0, |g| g.vault.sealed as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_vault_capacity() -> u32 {
    read(0, |g| g.vault.capacity as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_stimulation_level() -> u32 {
    read(0, |g| g.stimulation.level as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_stimulation_throttled() -> u32 {
    read(0, |g| g.stimulation.throttled as u32)
}

// ── release / action gate ────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_release_available() -> u32 {
    read(0, |g| g.release_available() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_release_blockers() -> u32 {
    read(0, |g| g.release_blockers() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_last_action_result() -> u32 {
    read(ActionResult::NoOp.code() as u32, |g| {
        g.last_action_result().code() as u32
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_gate_last_action_tick() -> u32 {
    read(0, |g| g.gate.last_action_tick)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_gate_accepted_this_tick() -> u32 {
    read(0, |g| g.gate.accepted_actions_this_tick as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_gate_rejected_total() -> u32 {
    read(0, |g| g.gate.rejected_actions_total)
}

// ── hash / seed ──────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_state_hash_hi() -> u32 {
    read(0, |g| (g.state_hash() >> 32) as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_state_hash_lo() -> u32 {
    read(0, |g| (g.state_hash() & 0xFFFF_FFFF) as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_seed_hi() -> u32 {
    read(0, |g| (g.seed >> 32) as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_seed_lo() -> u32 {
    read(0, |g| (g.seed & 0xFFFF_FFFF) as u32)
}

// ── labels (ptr/len into wasm memory) ────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_grade_label_ptr() -> u32 {
    ptr_of(read("", |g| g.grade().label()))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_grade_label_len() -> u32 {
    len_of(read("", |g| g.grade().label()))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_grade_public_label_ptr() -> u32 {
    ptr_of(read("", |g| g.grade().public_label()))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_grade_public_label_len() -> u32 {
    len_of(read("", |g| g.grade().public_label()))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_end_reason_label_ptr() -> u32 {
    ptr_of(read("", |g| g.end_reason().label()))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_end_reason_label_len() -> u32 {
    len_of(read("", |g| g.end_reason().label()))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_status_label_ptr() -> u32 {
    ptr_of(read("", |g| g.status().as_str()))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_status_label_len() -> u32 {
    len_of(read("", |g| g.status().as_str()))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_label_ptr() -> u32 {
    ptr_of(read("", |g| {
        g.focused_event().map(|e| e.kind.label()).unwrap_or("")
    }))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_active_event_label_len() -> u32 {
    len_of(read("", |g| {
        g.focused_event().map(|e| e.kind.label()).unwrap_or("")
    }))
}

// ── scenario metadata ────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_id() -> u32 {
    read(0, |g| g.scenario_id)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_count() -> u32 {
    SCENARIO_COUNT
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_difficulty() -> u32 {
    read(0, |g| g.scenario().difficulty.code() as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_max_ticks() -> u32 {
    read(0, |g| g.scenario().max_ticks)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_name_ptr(scenario_id: u32) -> u32 {
    ptr_of(scenario_by_id(scenario_id).map(|s| s.name).unwrap_or(""))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_name_len(scenario_id: u32) -> u32 {
    len_of(scenario_by_id(scenario_id).map(|s| s.name).unwrap_or(""))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_objective_ptr(scenario_id: u32) -> u32 {
    ptr_of(
        scenario_by_id(scenario_id)
            .map(|s| s.objective)
            .unwrap_or(""),
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_objective_len(scenario_id: u32) -> u32 {
    len_of(
        scenario_by_id(scenario_id)
            .map(|s| s.objective)
            .unwrap_or(""),
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_scenario_difficulty_by_id(scenario_id: u32) -> u32 {
    scenario_by_id(scenario_id)
        .map(|s| s.difficulty.code() as u32)
        .unwrap_or(NO_EVENT)
}

// ── replay / vector ──────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_builtin_vector_count() -> u32 {
    BUILTIN_VECTORS.len() as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_run_builtin_vector(index: u32) -> u32 {
    let idx = index as usize;
    match neural_boundary_core::run_builtin(idx) {
        Some(o) => {
            VECTOR.with(|v| v.set(Some(o)));
            o.grade.code() as u32
        }
        None => {
            VECTOR.with(|v| v.set(None));
            NO_EVENT
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_vector_hash_hi() -> u32 {
    VECTOR.with(|v| v.get().map(|o| (o.state_hash >> 32) as u32).unwrap_or(0))
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_vector_hash_lo() -> u32 {
    VECTOR.with(|v| {
        v.get()
            .map(|o| (o.state_hash & 0xFFFF_FFFF) as u32)
            .unwrap_or(0)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_vector_score() -> i32 {
    VECTOR.with(|v| v.get().map(|o| o.score).unwrap_or(0))
}

// ── performance counters ─────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn nbg_counter_unresolved_critical() -> u32 {
    read(0, |g| g.counters.unresolved_critical_events)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_counter_unsafe_actions() -> u32 {
    read(0, |g| g.counters.unsafe_actions)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_counter_successful_audits() -> u32 {
    read(0, |g| g.counters.successful_audits)
}
#[unsafe(no_mangle)]
pub extern "C" fn nbg_counter_correct_revocations() -> u32 {
    read(0, |g| g.counters.correct_revocations)
}
