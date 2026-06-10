use neural_boundary_core::{GameConfig, GameState, PlayerAction};
use serde::Deserialize;
use std::{env, fs, process};

#[derive(Debug, Deserialize)]
struct Replay {
    schema: String,
    seed: u32,
    config: ReplayConfig,
    actions: Vec<ActionAtTick>,
    expected: Expected,
}

#[derive(Debug, Deserialize)]
struct ReplayConfig {
    difficulty: u8,
}

#[derive(Debug, Deserialize)]
struct ActionAtTick {
    tick: u32,
    action: String,
}

#[derive(Debug, Deserialize)]
struct Expected {
    min_trust: u8,
    max_risk: u8,
    min_integrity: u8,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: neural-boundary-cli <vectors/replay-v1.0.3.json>");
        process::exit(2);
    });

    let raw = fs::read_to_string(&path).unwrap_or_else(|err| {
        eprintln!("failed to read {path}: {err}");
        process::exit(2);
    });

    let replay: Replay = serde_json::from_str(&raw).unwrap_or_else(|err| {
        eprintln!("failed to parse replay JSON: {err}");
        process::exit(2);
    });

    if replay.schema != "neural-boundary-replay-v1.0.3" {
        eprintln!("unsupported replay schema: {}", replay.schema);
        process::exit(2);
    }

    let mut game = GameState::new(GameConfig {
        seed: replay.seed,
        difficulty: replay.config.difficulty,
    });

    let mut next_action = 0usize;

    for tick in 1..=420 {
        let mut action = PlayerAction::Idle;
        if next_action < replay.actions.len() && replay.actions[next_action].tick == tick {
            action = parse_action(&replay.actions[next_action].action);
            next_action += 1;
        }
        game.step(action);
    }

    let snapshot = game.snapshot();
    let ok = snapshot.trust >= replay.expected.min_trust
        && snapshot.risk <= replay.expected.max_risk
        && snapshot.integrity >= replay.expected.min_integrity;

    if ok {
        println!("Replay OK");
        println!("Final trust: {}", snapshot.trust);
        println!("Final risk: {}", snapshot.risk);
        println!("Final integrity: {}", snapshot.integrity);
        println!(
            "Boundary status: {}",
            if snapshot.raw_leaks == 0 {
                "SEALED"
            } else {
                "BREACHED"
            }
        );
    } else {
        eprintln!("Replay FAILED");
        eprintln!("Final trust: {}", snapshot.trust);
        eprintln!("Final risk: {}", snapshot.risk);
        eprintln!("Final integrity: {}", snapshot.integrity);
        process::exit(1);
    }
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
        "toggle_help" => PlayerAction::ToggleHelp,
        "restart" => PlayerAction::Restart,
        "pause" => PlayerAction::Pause,
        _ => PlayerAction::Idle,
    }
}
