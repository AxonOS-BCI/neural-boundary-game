
#![forbid(unsafe_code)]

use neural_boundary_core::{BoundaryStatus, GameConfig, GameState, PlayerAction, RiskBand};

pub const VERSION_MAJOR: u32 = 1;
pub const VERSION_MINOR: u32 = 9;
pub const VERSION_PATCH: u32 = 8;

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

    game.apply_script(&actions);

    let snapshot = game.snapshot();
    let trust = (snapshot.trust as u32) << 16;
    let risk = (snapshot.risk as u32) << 8;

    trust | risk | snapshot.integrity as u32
}

pub fn nbg_review_flags_after_demo_path() -> u32 {
    let mut game = GameState::new(GameConfig::default());
    let actions = [
        PlayerAction::Validate,
        PlayerAction::ConsentGate,
        PlayerAction::Validate,
        PlayerAction::EvidenceGate,
        PlayerAction::Quarantine,
    ];

    game.apply_script(&actions);

    let summary = game.review_summary();
    let boundary_code = match summary.boundary_status {
        BoundaryStatus::Sealed => 1,
        BoundaryStatus::AtRisk => 2,
        BoundaryStatus::Breached => 3,
    };
    let risk_code = match summary.risk_band {
        RiskBand::Low => 1,
        RiskBand::Elevated => 2,
        RiskBand::Critical => 3,
    };

    let gates = (summary.gates_passed as u32) << 16;
    let boundary = (boundary_code as u32) << 8;

    gates | boundary | risk_code as u32
}

pub fn nbg_release_blockers_after_demo_path() -> u32 {
    let mut game = GameState::new(GameConfig::default());
    let actions = [
        PlayerAction::Validate,
        PlayerAction::ConsentGate,
        PlayerAction::Validate,
        PlayerAction::EvidenceGate,
        PlayerAction::Quarantine,
    ];

    game.apply_script(&actions);
    game.review_summary().release_blocker_count as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_adapter_functions_are_stable() {
        assert_eq!(nbg_version_tuple(), (1, 9, 8));
        assert_eq!(nbg_seed_default(), 1707);
        assert!(nbg_score_after_demo_path() > 0);
        assert!(nbg_review_flags_after_demo_path() > 0);
        assert!(nbg_release_blockers_after_demo_path() > 0);
    }
}
