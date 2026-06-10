use neural_boundary_core::{GameConfig, GameState, PlayerAction};
use std::{env, fs, process};

const EXPECTED_SCHEMA: &str = "neural-boundary-replay-v2.0.0";

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: neural-boundary-cli vectors/replay-v2.0.0.json");
        process::exit(2);
    });

    let raw = fs::read_to_string(&path).unwrap_or_else(|err| {
        eprintln!("failed to read {path}: {err}");
        process::exit(2);
    });

    if !raw.contains(EXPECTED_SCHEMA) {
        eprintln!("unsupported replay schema");
        process::exit(2);
    }

    let mut game = GameState::new(GameConfig::default());

    for action in extract_actions(&raw) {
        game.step(action);
    }

    for _ in 0..32 {
        game.step(PlayerAction::Idle);
    }

    let snapshot = game.snapshot();
    println!("Replay OK");
    println!("Final trust: {}", snapshot.trust);
    println!("Final risk: {}", snapshot.risk);
    println!("Final integrity: {}", snapshot.integrity);
    let summary = game.review_summary();

    println!(
        "Boundary status: {}",
        if snapshot.raw_leaks == 0 {
            "SEALED"
        } else {
            "BREACHED"
        }
    );
    println!("Release ready: {}", summary.release_ready);
    println!("Gates passed: {}", summary.gates_passed);
    println!("Gates remaining: {}", summary.gates_remaining);
    println!("Progress percent: {}", summary.progress_percent);
    println!("Release blockers: {}", summary.release_blocker_count);
}

fn extract_actions(raw: &str) -> Vec<PlayerAction> {
    raw.lines()
        .filter_map(|line| {
            let marker = "\"action\":";
            let idx = line.find(marker)?;
            let tail = &line[idx + marker.len()..];
            let first = tail.find('"')?;
            let rest = &tail[first + 1..];
            let second = rest.find('"')?;
            Some(parse_action(&rest[..second]))
        })
        .collect()
}

fn parse_action(value: &str) -> PlayerAction {
    match value {
        "move_up" => PlayerAction::MoveUp,
        "move_down" => PlayerAction::MoveDown,
        "validate" => PlayerAction::Validate,
        "convert" => PlayerAction::Convert,
        "quarantine" => PlayerAction::Quarantine,
        "consent_gate" => PlayerAction::ConsentGate,
        "evidence_gate" => PlayerAction::EvidenceGate,
        "release" => PlayerAction::Release,
        "restart" => PlayerAction::Restart,
        _ => PlayerAction::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_parser_accepts_known_actions() {
        assert_eq!(parse_action("validate"), PlayerAction::Validate);
        assert_eq!(parse_action("release"), PlayerAction::Release);
        assert_eq!(parse_action("unknown"), PlayerAction::Idle);
    }
}
