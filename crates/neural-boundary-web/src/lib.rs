//! Minimal dependency-free WebAssembly ABI adapter.
//!
//! The JavaScript shell renders state and maps input. All authoritative game
//! transitions remain in `neural-boundary-core`.

use neural_boundary_core::{
    daily_seed, BoundaryAction, Difficulty, RunMode, Simulation, SimulationConfig,
    BOUNDARY_X, ENTITY_CAPACITY, LANE_COUNT, TICK_RATE,
};
use std::cell::RefCell;

/// Packed ABI contract version. `3_000_000` represents ABI 3.0.0.
pub const ABI_VERSION: u32 = 3_000_000;

thread_local! {
    static SIMULATION: RefCell<Simulation> = RefCell::new(Simulation::new(
        SimulationConfig::canonical(58, RunMode::Guided, Difficulty::Standard),
    ));
}

fn with_simulation<T>(operation: impl FnOnce(&Simulation) -> T) -> T {
    SIMULATION.with(|cell| operation(&cell.borrow()))
}

fn with_simulation_mut<T>(operation: impl FnOnce(&mut Simulation) -> T) -> T {
    SIMULATION.with(|cell| operation(&mut cell.borrow_mut()))
}

const fn parse_mode(value: u32) -> RunMode {
    match value {
        1 => RunMode::Guided,
        2 => RunMode::Standard,
        3 => RunMode::Audit,
        4 => RunMode::Grand,
        5 => RunMode::Daily,
        _ => RunMode::Standard,
    }
}

const fn parse_difficulty(value: u32) -> Difficulty {
    match value {
        1 => Difficulty::Assisted,
        2 => Difficulty::Standard,
        3 => Difficulty::Expert,
        _ => Difficulty::Standard,
    }
}

const fn parse_action(value: u32) -> BoundaryAction {
    match value {
        1 => BoundaryAction::Validate,
        2 => BoundaryAction::Convert,
        3 => BoundaryAction::Quarantine,
        4 => BoundaryAction::Consent,
        5 => BoundaryAction::Evidence,
        6 => BoundaryAction::Release,
        _ => BoundaryAction::None,
    }
}

fn entity_at(index: u32) -> Option<neural_boundary_core::Entity> {
    let index = usize::try_from(index).ok()?;
    if index >= ENTITY_CAPACITY {
        return None;
    }
    with_simulation(|simulation| simulation.entity(index))
}

/// Return the packed ABI contract version.
#[no_mangle]
pub extern "C" fn nbg_abi_version() -> u32 {
    ABI_VERSION
}

/// Return the fixed simulation tick rate.
#[no_mangle]
pub extern "C" fn nbg_tick_rate() -> u32 {
    u32::from(TICK_RATE)
}

/// Return the logical lane count.
#[no_mangle]
pub extern "C" fn nbg_lane_count() -> u32 {
    LANE_COUNT as u32
}

/// Return the logical boundary coordinate.
#[no_mangle]
pub extern "C" fn nbg_boundary_x() -> u32 {
    u32::from(BOUNDARY_X)
}

/// Initialize a new run.
#[no_mangle]
pub extern "C" fn nbg_init(seed_low: u32, seed_high: u32, mode: u32, difficulty: u32) {
    let seed = u64::from(seed_low) | (u64::from(seed_high) << 32);
    let config = SimulationConfig::canonical(seed, parse_mode(mode), parse_difficulty(difficulty));
    SIMULATION.with(|cell| *cell.borrow_mut() = Simulation::new(config));
}

/// Derive the low half of the stable Daily Seed for a YYYYMMDD UTC date.
#[no_mangle]
pub extern "C" fn nbg_daily_seed_low(yyyymmdd: u32) -> u32 {
    daily_seed(yyyymmdd) as u32
}

/// Derive the high half of the stable Daily Seed.
#[no_mangle]
pub extern "C" fn nbg_daily_seed_high(yyyymmdd: u32) -> u32 {
    (daily_seed(yyyymmdd) >> 32) as u32
}

/// Advance a bounded number of fixed ticks.
#[no_mangle]
pub extern "C" fn nbg_tick(count: u32) {
    with_simulation_mut(|simulation| simulation.tick_many(count.min(240)));
}

/// Apply a stable action identifier. Unknown values are ignored.
#[no_mangle]
pub extern "C" fn nbg_action(action: u32) {
    with_simulation_mut(|simulation| simulation.apply_action(parse_action(action)));
}

/// Select a lane. Out-of-range values clamp to the final lane without integer wrapping.
#[no_mangle]
pub extern "C" fn nbg_select_lane(lane: u32) {
    let last_lane = (LANE_COUNT - 1) as u32;
    with_simulation_mut(|simulation| simulation.select_lane(lane.min(last_lane) as u8));
}

/// Move lane selection by at most one step.
#[no_mangle]
pub extern "C" fn nbg_move_lane(delta: i32) {
    with_simulation_mut(|simulation| simulation.move_lane(delta.clamp(-1, 1) as i8));
}

/// Return the current tick.
#[no_mangle]
pub extern "C" fn nbg_tick_value() -> u32 {
    with_simulation(|s| s.snapshot().tick)
}

/// Return the selected lane.
#[no_mangle]
pub extern "C" fn nbg_selected_lane() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().selected_lane))
}

/// Return trust.
#[no_mangle]
pub extern "C" fn nbg_trust() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().trust))
}

/// Return risk.
#[no_mangle]
pub extern "C" fn nbg_risk() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().risk))
}

/// Return integrity.
#[no_mangle]
pub extern "C" fn nbg_integrity() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().integrity))
}

/// Return evidence level.
#[no_mangle]
pub extern "C" fn nbg_evidence() -> u32 {
    with_simulation(|s| s.snapshot().evidence as u32)
}

/// Return review-gate mask.
#[no_mangle]
pub extern "C" fn nbg_review_gates() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().review_gates))
}

/// Return raw leak count.
#[no_mangle]
pub extern "C" fn nbg_raw_leaks() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().raw_leaks))
}

/// Return configured raw leak limit.
#[no_mangle]
pub extern "C" fn nbg_raw_leak_limit() -> u32 {
    with_simulation(|s| u32::from(s.config().raw_leak_limit))
}

/// Return score.
#[no_mangle]
pub extern "C" fn nbg_score() -> u32 {
    with_simulation(|s| s.snapshot().score)
}

/// Return current streak.
#[no_mangle]
pub extern "C" fn nbg_streak() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().streak))
}

/// Return best streak.
#[no_mangle]
pub extern "C" fn nbg_best_streak() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().best_streak))
}

/// Return boundary status.
#[no_mangle]
pub extern "C" fn nbg_status() -> u32 {
    with_simulation(|s| s.snapshot().status as u32)
}

/// Return terminal reason.
#[no_mangle]
pub extern "C" fn nbg_terminal_reason() -> u32 {
    with_simulation(|s| s.snapshot().terminal_reason as u32)
}

/// Return presentation feedback code.
#[no_mangle]
pub extern "C" fn nbg_feedback() -> u32 {
    with_simulation(|s| s.snapshot().feedback as u32)
}

/// Return release blocker mask.
#[no_mangle]
pub extern "C" fn nbg_release_blockers() -> u32 {
    with_simulation(|s| u32::from(s.release_blockers()))
}

/// Return one when all release invariants pass.
#[no_mangle]
pub extern "C" fn nbg_release_ready() -> u32 {
    with_simulation(|s| if s.release_ready() { 1 } else { 0 })
}

/// Return one when consent is active.
#[no_mangle]
pub extern "C" fn nbg_consent_active() -> u32 {
    with_simulation(|s| if s.snapshot().consent.active { 1 } else { 0 })
}

/// Return consent scope mask.
#[no_mangle]
pub extern "C" fn nbg_consent_scope() -> u32 {
    with_simulation(|s| u32::from(s.snapshot().consent.scope))
}

/// Return consent expiry tick.
#[no_mangle]
pub extern "C" fn nbg_consent_expiry_tick() -> u32 {
    with_simulation(|s| s.snapshot().consent.expiry_tick)
}

/// Return low half of canonical state hash.
#[no_mangle]
pub extern "C" fn nbg_state_hash_low() -> u32 {
    with_simulation(|s| s.snapshot().state_hash as u32)
}

/// Return high half of canonical state hash.
#[no_mangle]
pub extern "C" fn nbg_state_hash_high() -> u32 {
    with_simulation(|s| (s.snapshot().state_hash >> 32) as u32)
}

/// Return fixed entity capacity.
#[no_mangle]
pub extern "C" fn nbg_entity_capacity() -> u32 {
    ENTITY_CAPACITY as u32
}

/// Return active entity count.
#[no_mangle]
pub extern "C" fn nbg_active_entity_count() -> u32 {
    with_simulation(|s| s.active_entity_count() as u32)
}

/// Return one when an entity slot is active.
#[no_mangle]
pub extern "C" fn nbg_entity_active(index: u32) -> u32 {
    if entity_at(index).is_some() {
        1
    } else {
        0
    }
}

/// Return entity identifier.
#[no_mangle]
pub extern "C" fn nbg_entity_id(index: u32) -> u32 {
    entity_at(index).map_or(0, |entity| entity.id)
}

/// Return entity kind.
#[no_mangle]
pub extern "C" fn nbg_entity_kind(index: u32) -> u32 {
    entity_at(index).map_or(0, |entity| entity.kind as u32)
}

/// Return entity lane.
#[no_mangle]
pub extern "C" fn nbg_entity_lane(index: u32) -> u32 {
    entity_at(index).map_or(0, |entity| u32::from(entity.lane))
}

/// Return entity position.
#[no_mangle]
pub extern "C" fn nbg_entity_position(index: u32) -> u32 {
    entity_at(index).map_or(0, |entity| u32::from(entity.position))
}

/// Return entity flags.
#[no_mangle]
pub extern "C" fn nbg_entity_flags(index: u32) -> u32 {
    entity_at(index).map_or(0, |entity| u32::from(entity.flags))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_metadata_is_stable() {
        assert_eq!(nbg_abi_version(), ABI_VERSION);
        assert_eq!(nbg_tick_rate(), 60);
        assert_eq!(nbg_lane_count(), 5);
        assert_eq!(nbg_boundary_x(), 840);
    }

    #[test]
    fn abi_initializes_and_ticks() {
        nbg_init(58, 0, RunMode::Guided as u32, Difficulty::Standard as u32);
        assert_eq!(nbg_tick_value(), 0);
        nbg_tick(60);
        assert_eq!(nbg_tick_value(), 60);
        assert!(nbg_entity_capacity() >= 16);
    }

    #[test]
    fn invalid_numeric_inputs_do_not_wrap_into_valid_values() {
        nbg_init(58, 0, 258, 259);
        nbg_select_lane(256);
        assert_eq!(nbg_selected_lane(), 4);
        let before = nbg_state_hash_low();
        nbg_action(257);
        assert_eq!(nbg_state_hash_low(), before);
    }

    #[test]
    fn out_of_range_entity_index_is_safe() {
        assert_eq!(nbg_entity_active(u32::MAX), 0);
        assert_eq!(nbg_entity_id(u32::MAX), 0);
    }
}
