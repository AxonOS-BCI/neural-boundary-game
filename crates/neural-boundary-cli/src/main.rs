<<<<<<< HEAD
//! neural-boundary-cli — replay verifier and vector toolkit for the
//! Neural Boundary Game (v2.1.2, Foundation Grande AxonOS Standard Edition).
//!
//! Subcommands:
//!   verify [path]   Verify a replay vector (default: vectors/replay-v2.1.2.json)
//!   record ...      Record a deterministic policy run into a vector file
//!   search ...      Search seeds for the canonical clean-run finals
//!   trace ...       Print per-tick events of a clean-policy run (dev tool)

mod bot;

use bot::{RecordedAction, RunSummary};
use neural_boundary_core::{Action, Difficulty, GameConfig, GameState, Status};
use serde::{Deserialize, Serialize};
use std::{env, fs, process};

const SCHEMA: &str = "neural-boundary-replay-v2.1.2";
const DEFAULT_VECTOR: &str = "vectors/replay-v2.1.2.json";
const DEFAULT_MAX_TICKS: u32 = 14_000;

#[derive(Debug, Serialize, Deserialize)]
struct ReplayFile {
    schema: String,
    title: String,
    generated_by: String,
    seed: u64,
    difficulty: String,
    actions: Vec<ActionEntry>,
    expected: Expected,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActionEntry {
    tick: u32,
    lane: u8,
    action: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Expected {
    final_tick: u32,
    trust: u8,
    risk: u8,
    integrity: u8,
    evidence_level: String,
    raw_leaks: u8,
    gates_passed: u8,
    status: String,
    boundary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<String>,
    state_hash: String,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        None | Some("verify") => cmd_verify(args.get(1).map(String::as_str)),
        Some("record") => cmd_record(&args[1..]),
        Some("search") => cmd_search(&args[1..]),
        Some("trace") => cmd_trace(&args[1..]),
        Some("--help") | Some("-h") | Some("help") => {
            print_usage();
            0
        }
        Some(other) => {
            eprintln!("unknown subcommand: {other}");
            print_usage();
            2
        }
    };
    process::exit(code);
}

fn print_usage() {
    println!("neural-boundary-cli {}", env!("CARGO_PKG_VERSION"));
    println!("Replay verifier for the Neural Boundary Game deterministic core.\n");
    println!("USAGE:");
    println!("  neural-boundary-cli verify [path]");
    println!("      Verify a replay vector (default: {DEFAULT_VECTOR}).");
    println!("  neural-boundary-cli record --seed N [--difficulty calm|standard|intense]");
    println!("      [--policy clean|idle] [--max-ticks N] [--title S] --out PATH");
    println!("      Record a deterministic policy run into a vector file.");
    println!("  neural-boundary-cli search --from A --to B [--difficulty D]");
    println!("      [--target trust,risk,integrity] [--max-ticks N]");
    println!("      Search seeds for the canonical clean-run finals.");
    println!("  neural-boundary-cli trace --seed N [--difficulty D] [--ticks N]");
    println!("      Print per-tick events of a clean-policy run.");
}

// ---------------------------------------------------------------------------
// verify
// ---------------------------------------------------------------------------

fn cmd_verify(path: Option<&str>) -> i32 {
    let path = path.unwrap_or(DEFAULT_VECTOR);
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => {
            eprintln!("failed to read {path}: {err}");
            return 2;
        }
    };
    let replay: ReplayFile = match serde_json::from_str(&raw) {
        Ok(replay) => replay,
        Err(err) => {
            eprintln!("failed to parse {path}: {err}");
            return 2;
        }
    };

    if replay.schema != SCHEMA {
        eprintln!(
            "unsupported replay schema: {} (expected {SCHEMA})",
            replay.schema
        );
        return 2;
    }
    let Some(difficulty) = Difficulty::from_name(&replay.difficulty) else {
        eprintln!("unknown difficulty: {}", replay.difficulty);
        return 2;
    };

    // Structural validation of the action script.
    let mut actions = Vec::with_capacity(replay.actions.len());
    let mut last_tick = 0u32;
    for entry in &replay.actions {
        if entry.tick == 0 || entry.tick <= last_tick {
            eprintln!(
                "action ticks must be strictly increasing (at tick {})",
                entry.tick
            );
            return 2;
        }
        if entry.tick > replay.expected.final_tick {
            eprintln!("action at tick {} is past final_tick", entry.tick);
            return 2;
        }
        if entry.lane >= neural_boundary_core::LANES {
            eprintln!("lane out of range at tick {}", entry.tick);
            return 2;
        }
        let Some(action) = Action::from_name(&entry.action) else {
            eprintln!("unknown action {:?} at tick {}", entry.action, entry.tick);
            return 2;
        };
        actions.push(RecordedAction {
            tick: entry.tick,
            lane: entry.lane,
            action,
        });
        last_tick = entry.tick;
    }

    let config = GameConfig {
        seed: replay.seed,
        difficulty,
    };
    let summary = bot::replay_script(config, &actions, replay.expected.final_tick);

    let mut mismatches = Vec::new();
    let expected = &replay.expected;
    check(
        &mut mismatches,
        "final_tick",
        expected.final_tick,
        summary.final_tick,
    );
    check(&mut mismatches, "trust", expected.trust, summary.trust);
    check(&mut mismatches, "risk", expected.risk, summary.risk);
    check(
        &mut mismatches,
        "integrity",
        expected.integrity,
        summary.integrity,
    );
    check(
        &mut mismatches,
        "evidence_level",
        expected.evidence_level.as_str(),
        summary.evidence_level.as_str(),
    );
    check(
        &mut mismatches,
        "raw_leaks",
        expected.raw_leaks,
        summary.raw_leaks,
    );
    check(
        &mut mismatches,
        "gates_passed",
        expected.gates_passed,
        summary.gates_passed,
    );
    check(
        &mut mismatches,
        "status",
        expected.status.as_str(),
        summary.status.as_str(),
    );
    check(
        &mut mismatches,
        "boundary",
        expected.boundary.as_str(),
        summary.status.boundary(),
    );
    if let Some(cause) = &expected.cause {
        let actual = match summary.status {
            Status::Defeat(cause) => cause.as_str(),
            _ => "none",
        };
        check(&mut mismatches, "cause", cause.as_str(), actual);
    }
    check(
        &mut mismatches,
        "state_hash",
        expected.state_hash.to_ascii_lowercase().as_str(),
        format!("0x{:016x}", summary.state_hash).as_str(),
    );

    if mismatches.is_empty() {
        println!("Replay OK");
        println!("Final trust: {}", summary.trust);
        println!("Final risk: {}", summary.risk);
        println!("Final integrity: {}", summary.integrity);
        println!("Boundary status: {}", summary.status.boundary());
        0
    } else {
        eprintln!("Replay FAILED ({path})");
        for line in mismatches {
            eprintln!("  {line}");
        }
        1
    }
}

fn check<T: PartialEq + std::fmt::Display>(
    out: &mut Vec<String>,
    field: &str,
    expected: T,
    actual: T,
) {
    if expected != actual {
        out.push(format!("{field}: expected {expected}, got {actual}"));
    }
}

// ---------------------------------------------------------------------------
// record
// ---------------------------------------------------------------------------

fn cmd_record(args: &[String]) -> i32 {
    let mut seed = 0x2112u64;
    let mut difficulty = Difficulty::Standard;
    let mut policy = "clean".to_string();
    let mut max_ticks = DEFAULT_MAX_TICKS;
    let mut title = String::new();
    let mut out_path = String::new();

    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--seed" => seed = parse_or_exit(iter.next(), "--seed"),
            "--difficulty" => difficulty = parse_difficulty(iter.next()),
            "--policy" => policy = required(iter.next(), "--policy"),
            "--max-ticks" => max_ticks = parse_or_exit(iter.next(), "--max-ticks"),
            "--title" => title = required(iter.next(), "--title"),
            "--out" => out_path = required(iter.next(), "--out"),
            other => {
                eprintln!("unknown flag: {other}");
                process::exit(2);
            }
        }
    }
    if out_path.is_empty() {
        eprintln!("record requires --out PATH");
        return 2;
    }

    let config = GameConfig { seed, difficulty };
    let result = match policy.as_str() {
        "clean" => bot::run_clean_policy(config, max_ticks),
        "idle" => bot::run_idle_policy(config, max_ticks),
        other => {
            eprintln!("unknown policy: {other} (expected clean|idle)");
            return 2;
        }
    };
    let summary = result.summary;
    if summary.status == Status::Running {
        eprintln!(
            "policy run did not terminate within {max_ticks} ticks (status running); not writing"
        );
        return 1;
    }

    if title.is_empty() {
        title = match summary.status {
            Status::Victory => "Canonical clean run — boundary sealed".to_string(),
            Status::Defeat(_) => "Idle run — boundary breach demonstration".to_string(),
            Status::Running => unreachable!(),
        };
    }

    let file = ReplayFile {
        schema: SCHEMA.to_string(),
        title,
        generated_by: format!(
            "neural-boundary-cli record --policy {policy} --seed {seed} --difficulty {}",
            difficulty.name()
        ),
        seed,
        difficulty: difficulty.name().to_string(),
        actions: result
            .actions
            .iter()
            .map(|action| ActionEntry {
                tick: action.tick,
                lane: action.lane,
                action: action.action.name().to_string(),
            })
            .collect(),
        expected: expected_from(&summary),
    };

    let mut json = serde_json::to_string_pretty(&file).expect("serialize replay");
    json.push('\n');
    if let Err(err) = fs::write(&out_path, json) {
        eprintln!("failed to write {out_path}: {err}");
        return 2;
    }

    println!("Recorded {} actions to {out_path}", file.actions.len());
    print_summary(&summary);
    0
}

fn expected_from(summary: &RunSummary) -> Expected {
    Expected {
        final_tick: summary.final_tick,
        trust: summary.trust,
        risk: summary.risk,
        integrity: summary.integrity,
        evidence_level: summary.evidence_level.as_str().to_string(),
        raw_leaks: summary.raw_leaks,
        gates_passed: summary.gates_passed,
        status: summary.status.as_str().to_string(),
        boundary: summary.status.boundary().to_string(),
        cause: match summary.status {
            Status::Defeat(cause) => Some(cause.as_str().to_string()),
            _ => None,
        },
        state_hash: format!("0x{:016x}", summary.state_hash),
    }
}

fn print_summary(summary: &RunSummary) {
    println!(
        "Status: {} ({})",
        summary.status.as_str(),
        summary.status.boundary()
    );
    println!("Final tick: {}", summary.final_tick);
    println!(
        "Trust {} | Risk {} | Integrity {} | Evidence {} ({} pts) | Gates {}/5 | Leaks {} | Delivered {}",
        summary.trust,
        summary.risk,
        summary.integrity,
        summary.evidence_level.as_str(),
        summary.evidence_points,
        summary.gates_passed,
        summary.raw_leaks,
        summary.delivered
    );
    println!("State hash: 0x{:016x}", summary.state_hash);
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

fn cmd_search(args: &[String]) -> i32 {
    let mut from = 1u64;
    let mut to = 50_000u64;
    let mut difficulty = Difficulty::Standard;
    let mut target = (92u8, 12u8, 88u8);
    let mut max_ticks = DEFAULT_MAX_TICKS;

    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--from" => from = parse_or_exit(iter.next(), "--from"),
            "--to" => to = parse_or_exit(iter.next(), "--to"),
            "--difficulty" => difficulty = parse_difficulty(iter.next()),
            "--max-ticks" => max_ticks = parse_or_exit(iter.next(), "--max-ticks"),
            "--target" => {
                let value = required(iter.next(), "--target");
                let parts: Vec<u8> = value
                    .split(',')
                    .map(|part| part.trim().parse().expect("target u8"))
                    .collect();
                if parts.len() != 3 {
                    eprintln!("--target expects trust,risk,integrity");
                    return 2;
                }
                target = (parts[0], parts[1], parts[2]);
            }
            other => {
                eprintln!("unknown flag: {other}");
                process::exit(2);
            }
        }
    }

    eprintln!(
        "searching seeds {from}..={to} on {} for finals trust={} risk={} integrity={} ...",
        difficulty.name(),
        target.0,
        target.1,
        target.2
    );
    match bot::search_seed(difficulty, from, to, target, max_ticks) {
        Some((seed, result)) => {
            println!("Seed found: {seed} (0x{seed:x})");
            println!("Actions: {}", result.actions.len());
            print_summary(&result.summary);
            0
        }
        None => {
            eprintln!("no seed in range produced the target finals");
            1
        }
    }
}

// ---------------------------------------------------------------------------
// trace (developer tool)
// ---------------------------------------------------------------------------

fn cmd_trace(args: &[String]) -> i32 {
    let mut seed = 0x2112u64;
    let mut difficulty = Difficulty::Standard;
    let mut ticks = 3_000u32;

    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--seed" => seed = parse_or_exit(iter.next(), "--seed"),
            "--difficulty" => difficulty = parse_difficulty(iter.next()),
            "--ticks" => ticks = parse_or_exit(iter.next(), "--ticks"),
            other => {
                eprintln!("unknown flag: {other}");
                process::exit(2);
            }
        }
    }

    let mut state = GameState::new(GameConfig { seed, difficulty });
    for tick in 1..=ticks {
        let input = bot::decide(&state);
        state.step(input);
        for event in state.events().iter() {
            println!("{tick:>6}  {event:?}");
        }
        if state.status() != Status::Running {
            break;
        }
    }
    print_summary(&bot::summarize(&state));
    0
}

// ---------------------------------------------------------------------------
// arg helpers
// ---------------------------------------------------------------------------

fn required(value: Option<&String>, flag: &str) -> String {
    value.cloned().unwrap_or_else(|| {
        eprintln!("{flag} requires a value");
        process::exit(2);
    })
}

fn parse_or_exit<T: std::str::FromStr>(value: Option<&String>, flag: &str) -> T {
    let raw = required(value, flag);
    raw.parse().unwrap_or_else(|_| {
        eprintln!("{flag}: invalid value {raw:?}");
        process::exit(2);
    })
}

fn parse_difficulty(value: Option<&String>) -> Difficulty {
    let raw = required(value, "--difficulty");
    Difficulty::from_name(&raw).unwrap_or_else(|| {
        eprintln!("--difficulty: expected calm|standard|intense, got {raw:?}");
        process::exit(2);
    })
=======
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
>>>>>>> origin/main
}
