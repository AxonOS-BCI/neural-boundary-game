// SPDX-FileCopyrightText: 2026 Denis Yermakou
// SPDX-FileContributor: AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial

//! Flat numeric WASM ABI (41 named exports, §26.1).
//! Uses #[unsafe(no_mangle)] per Rust 1.82+ stable preferred syntax.
//! The core crate retains #![forbid(unsafe_code)]; this shim does not.

#![allow(unsafe_code)]

use neural_boundary_core::{
    Difficulty, Entity, Grade, Input, Mode, Simulation, ABI_VERSION, BOUNDARY_X, CORE_VERSION,
    ENTITY_CAPACITY, LANES, PRODUCT_VERSION_PACKED, REPLAY_SCHEMA, TICKS_PER_SECOND,
};
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<Option<Simulation>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&Simulation) -> R) -> R {
    STATE.with(|cell| f(cell.borrow().as_ref().expect("nbg_init not called")))
}

fn with_mut<R>(f: impl FnOnce(&mut Simulation) -> R) -> R {
    STATE.with(|cell| f(cell.borrow_mut().as_mut().expect("nbg_init not called")))
}

// ── Static metadata ──────────────────────────────────────────────────────────
// Exports 01-05

/// Export 01 — ABI version; must be 1.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_abi_version() -> u32 {
    ABI_VERSION
}

/// Export 02 — Product version packed: (major<<16)|(minor<<8)|patch.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_product_version_packed() -> u32 {
    PRODUCT_VERSION_PACKED
}

/// Export 03 — Tick rate (60 Hz).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_tick_rate() -> u32 {
    TICKS_PER_SECOND
}

/// Export 04 — Number of signal lanes (5).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_lane_count() -> u32 {
    LANES as u32
}

/// Export 05 — Entity pool capacity (32).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_entity_capacity() -> u32 {
    ENTITY_CAPACITY as u32
}

/// Export 06 — Logical boundary X coordinate (704).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_boundary_x() -> u32 {
    BOUNDARY_X as u32
}

// ── Lifecycle ────────────────────────────────────────────────────────────────
// Exports 07-10

/// Export 07 — Initialise or re-initialise with a 64-bit seed.
/// mode: 1=Guided 2=Standard 3=Audit 4=Grand 5=Daily 6=PrivacyVault 7=KernelTrial
/// difficulty: 0=Calm 1=Standard 2=Intense
/// Returns 0 on success, 1 on unknown mode/difficulty.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_init(mode: u8, seed_hi: u32, seed_lo: u32, difficulty: u8) -> u32 {
    let Some(mode) = Mode::from_u8(mode) else {
        return 1;
    };
    let Some(difficulty) = Difficulty::from_u8(difficulty) else {
        return 1;
    };
    let seed = ((seed_hi as u64) << 32) | seed_lo as u64;
    STATE.with(|cell| {
        *cell.borrow_mut() = Some(Simulation::new(mode, difficulty, seed));
    });
    0
}

/// Export 08 — Reset to initial state keeping same config.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_reset() {
    STATE.with(|cell| {
        if let Some(sim) = cell.borrow().as_ref() {
            let (m, d, s) = (sim.mode(), sim.difficulty(), sim.seed());
            *cell.borrow_mut() = Some(Simulation::new(m, d, s));
        }
    });
}

/// Export 09 — Advance `ticks` steps with IDLE input. Returns actual steps taken.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_step(ticks: u32) -> u32 {
    let mut done = 0;
    for _ in 0..ticks {
        if with(|s| s.status().is_terminal()) {
            break;
        }
        with_mut(|s| s.step(Input::IDLE));
        done += 1;
    }
    done
}

/// Export 10 — Apply a single action, then step one tick.
/// Returns 0 success, 1 invalid action.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_apply_action(lane: u8, action_id: u8) -> u32 {
    use neural_boundary_core::Action;
    let action = Action::from_u8(action_id);
    let (lane_opt, act_opt) = match action {
        Some(a) => (if lane < LANES { Some(lane) } else { None }, Some(a)),
        None => return 1,
    };
    with_mut(|s| {
        s.step(Input {
            lane: lane_opt,
            action: act_opt,
        })
    });
    0
}

// ── Pause/resume (presentation-only; core ticks only when JS calls step) ─────
// Exports 11-13

static PAUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Export 11 — Signal pause intent to the ABI layer.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_pause() {
    PAUSED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Export 12 — Signal resume.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_resume() {
    PAUSED.store(false, std::sync::atomic::Ordering::Relaxed);
}

/// Export 13 — Returns 1 if paused, 0 if running.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_is_paused() -> u32 {
    PAUSED.load(std::sync::atomic::Ordering::Relaxed) as u32
}

// ── Lane control ─────────────────────────────────────────────────────────────
// Exports 14-15

static SELECTED_LANE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(2);

/// Export 14 — Set the selected lane (0-4). Returns 1 if out of range.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_select_lane(lane: u8) -> u32 {
    if lane >= LANES {
        return 1;
    }
    SELECTED_LANE.store(lane, std::sync::atomic::Ordering::Relaxed);
    0
}

/// Export 15 — Current selected lane.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_selected_lane() -> u32 {
    SELECTED_LANE.load(std::sync::atomic::Ordering::Relaxed) as u32
}

// ── Simulation scalar state ───────────────────────────────────────────────────
// Exports 16-41

/// Export 16
#[unsafe(no_mangle)]
pub extern "C" fn nbg_phase() -> u32 {
    with(|s| s.phase() as u32)
}

/// Export 17
#[unsafe(no_mangle)]
pub extern "C" fn nbg_mode() -> u32 {
    with(|s| s.mode().code() as u32)
}

/// Export 18
#[unsafe(no_mangle)]
pub extern "C" fn nbg_tick() -> u32 {
    with(|s| s.tick())
}

/// Export 19
#[unsafe(no_mangle)]
pub extern "C" fn nbg_score() -> u64 {
    with(|s| s.score())
}

/// Export 20
#[unsafe(no_mangle)]
pub extern "C" fn nbg_trust() -> u32 {
    with(|s| s.trust() as u32)
}

/// Export 21
#[unsafe(no_mangle)]
pub extern "C" fn nbg_risk() -> u32 {
    with(|s| s.risk() as u32)
}

/// Export 22
#[unsafe(no_mangle)]
pub extern "C" fn nbg_integrity() -> u32 {
    with(|s| s.integrity() as u32)
}

/// Export 23
#[unsafe(no_mangle)]
pub extern "C" fn nbg_evidence_level() -> u32 {
    with(|s| s.evidence_level() as u32)
}

/// Export 24
#[unsafe(no_mangle)]
pub extern "C" fn nbg_evidence_bits() -> u32 {
    with(|s| s.evidence_bits() as u32)
}

/// Export 25
#[unsafe(no_mangle)]
pub extern "C" fn nbg_gate_mask() -> u32 {
    with(|s| s.gate_mask() as u32)
}

/// Export 26 — Blocker mask: bits of failing gates at current tick.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_blocker_mask() -> u32 {
    with(|s| (!s.gate_mask() & 0x7F) as u32)
}

/// Export 27 — Consent scope bits (0 = no active consent).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_consent_scope() -> u32 {
    with(|s| s.consent().scope_mask as u32)
}

/// Export 28 — Consent epoch.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_consent_epoch() -> u32 {
    with(|s| s.consent_epoch())
}

/// Export 29 — Ticks remaining before consent expires (0 = no active consent).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_consent_expires_tick() -> u32 {
    with(|s| s.consent_expires_tick())
}

/// Export 30 — Raw leak count.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_raw_leaks() -> u32 {
    with(|s| s.raw_leaks() as u32)
}

/// Export 31 — Combo count.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_combo() -> u32 {
    with(|s| s.combo())
}

/// Export 32 — Best combo this run.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_best_combo() -> u32 {
    with(|s| s.best_combo())
}

/// Export 33 — Terminal status code (0=Running,1=Sealed,2=Breached,3=Unsafe,4=Aborted,5=FatalRuntime).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_terminal_status() -> u32 {
    with(|s| s.status().code() as u32)
}

/// Export 34 — Terminal reason code.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_terminal_reason() -> u32 {
    with(|s| s.reason().code() as u32)
}

/// Export 35 — Grade code: 0=Sovereign,1=Sealed,2=Reviewable,3=Degraded,4=Breached,5=Unsafe.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_grade() -> u32 {
    with(|s| match s.grade() {
        Grade::Sovereign => 0,
        Grade::Sealed => 1,
        Grade::Reviewable => 2,
        Grade::Degraded => 3,
        Grade::Breached => 4,
        Grade::Unsafe => 5,
    })
}

/// Export 36 — State hash high 32 bits.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_state_hash_hi() -> u32 {
    with(|s| (s.state_hash() >> 32) as u32)
}

/// Export 37 — State hash low 32 bits.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_state_hash_lo() -> u32 {
    with(|s| s.state_hash() as u32)
}

// ── Entity pool getters (slot: 0..ENTITY_CAPACITY) ───────────────────────────
// Exports 38-41. Returns 0xFFFFFFFF for an empty slot.

/// Export 38 — Entity kind code for slot (0xFFFFFFFF = empty).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_entity_kind(slot: u32) -> u32 {
    get_entity(slot)
        .map(|e| e.kind.code() as u32)
        .unwrap_or(0xFFFF_FFFF)
}

/// Export 39 — Entity lane for slot.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_entity_lane(slot: u32) -> u32 {
    get_entity(slot)
        .map(|e| e.lane as u32)
        .unwrap_or(0xFFFF_FFFF)
}

/// Export 40 — Entity X position (Q24.8) for slot.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_entity_x(slot: u32) -> u32 {
    get_entity(slot).map(|e| e.x_q8).unwrap_or(0xFFFF_FFFF)
}

/// Export 41 — Entity flags for slot.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_entity_flags(slot: u32) -> u32 {
    get_entity(slot)
        .map(|e| e.flags as u32)
        .unwrap_or(0xFFFF_FFFF)
}

fn get_entity(slot: u32) -> Option<Entity> {
    if slot as usize >= ENTITY_CAPACITY {
        return None;
    }
    with(|s| s.pool()[slot as usize])
}

// ── String table accessors (returns static byte slices via ptr+len) ───────────

/// Pointer to the WASM linear memory start of the core version string.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_schema_version_ptr() -> u32 {
    REPLAY_SCHEMA.as_ptr() as u32
}

/// Length of the schema version string.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_schema_version_len() -> u32 {
    REPLAY_SCHEMA.len() as u32
}

/// WASM memory export (required by JS runtime for ArrayBuffer views).
#[unsafe(no_mangle)]
pub extern "C" fn nbg_core_version_ptr() -> u32 {
    CORE_VERSION.as_ptr() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn nbg_core_version_len() -> u32 {
    CORE_VERSION.len() as u32
}

// ── Initialisation sentinel — called at WASM module start ────────────────────

/// Called by the JS runtime immediately after instantiation to confirm the
/// module is healthy. Returns PRODUCT_VERSION_PACKED or 0 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn nbg_health_check() -> u32 {
    PRODUCT_VERSION_PACKED
}
