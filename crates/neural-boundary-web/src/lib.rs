#![forbid(unsafe_code)]

use neural_boundary_core::{GameConfig, GameState, PlayerAction};

#[no_mangle]
pub extern "C" fn nbg_version_major() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn nbg_version_minor() -> u32 {
    3
}

#[no_mangle]
pub extern "C" fn nbg_version_patch() -> u32 {
    0
}

#[no_mangle]
pub extern "C" fn nbg_seed_default() -> u32 {
    GameConfig::default().seed
}

#[no_mangle]
pub extern "C" fn nbg_score_after_demo_path() -> u32 {
    let mut game = GameState::new(GameConfig::default());
    let actions = [
        PlayerAction::Validate,
        PlayerAction::ConsentGate,
        PlayerAction::Validate,
        PlayerAction::EvidenceGate,
        PlayerAction::Quarantine,
    ];

    for action in actions {
        game.step(action);
    }

    let snapshot = game.snapshot();
    ((snapshot.trust as u32) << 16) | ((snapshot.risk as u32) << 8) | snapshot.integrity as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_exports_are_stable() {
        assert_eq!(nbg_version_major(), 1);
        assert_eq!(nbg_version_minor(), 3);
        assert_eq!(nbg_version_patch(), 0);
        assert_eq!(nbg_seed_default(), 1707);
        assert!(nbg_score_after_demo_path() > 0);
    }
}
