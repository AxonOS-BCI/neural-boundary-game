#![forbid(unsafe_code)]

use neural_boundary_core::{GameConfig, GameState, PlayerAction};

pub const VERSION_MAJOR: u32 = 1;
pub const VERSION_MINOR: u32 = 4;
pub const VERSION_PATCH: u32 = 0;

pub fn nbg_version_tuple() -> (u32, u32, u32) {
    (VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH)
}

pub fn nbg_seed_default() -> u32 {
    GameConfig::default().seed
}

pub fn nbg_score_after_demo_path() -> u32 {
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
    fn wasm_adapter_functions_are_stable() {
        assert_eq!(nbg_version_tuple(), (1, 4, 0));
        assert_eq!(nbg_seed_default(), 1707);
        assert!(nbg_score_after_demo_path() > 0);
    }
}
