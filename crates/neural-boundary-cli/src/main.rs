//! neural-boundary-cli — replay verifier and conformance toolkit for the
//! Neural Boundary Game v3.0.1 (Sovereign Boundary Edition).
//!
//! Commands: `version`, `schema`, `verify <path>`, `verify-all`, `record`,
//! `trace <path>`, `search`.
//!
//! Exit-code contract (documented in `docs/REPLAY_SPEC.md`):
//! `0` success · `2` invalid CLI usage · `3` malformed replay/schema
//! structure · `4` version or compatibility mismatch · `5` deterministic
//! state/hash mismatch · `6` checksum mismatch · `7` internal invariant
//! failure.

mod bot;

use bot::{Policy, RecordedInput, RunSummary, SearchGoal};
use neural_boundary_core::{
    daily_seed, Action, Difficulty, Grade, RunMode, Simulation, SimulationConfig, Status,
    TerminalReason, CORE_VERSION, HASH_ALGORITHM, LANES, REPLAY_SCHEMA, TICKS_PER_SECOND,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, fs, path::Path, path::PathBuf, process};

const EXIT_OK: i32 = 0;
const EXIT_USAGE: i32 = 2;
const EXIT_MALFORMED: i32 = 3;
const EXIT_COMPAT: i32 = 4;
const EXIT_STATE_MISMATCH: i32 = 5;
const EXIT_CHECKSUM: i32 = 6;
#[allow(dead_code)]
const EXIT_INTERNAL: i32 = 7;

const VECTOR_DIR: &str = "vectors";
const CHECKSUM_FILE: &str = "vectors/checksums.sha256";
const DEFAULT_MAX_TICKS: u32 = 12_000;

// ---------------------------------------------------------------------------
// Replay schema (serde)
// ---------------------------------------------------------------------------

/// Unknown top-level and nested fields are ignored for forward
/// compatibility; missing required fields are malformed (exit 3).
#[derive(Debug, Serialize, Deserialize)]
struct ReplayFile {
    schema: String,
    product_version: String,
    core_version: String,
    hash_algorithm: String,
    title: String,
    generated_by: String,
    seed: u64,
    mode: String,
    difficulty: String,
    tick_rate: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    inputs: Vec<InputEntry>,
    expected: Expected,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InputEntry {
    tick: u32,
    lane: u8,
    action: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Expected {
    terminal_tick: u32,
    status: String,
    terminal_reason: String,
    boundary: String,
    grade: String,
    trust: i32,
    risk: i32,
    integrity: i32,
    evidence_level: String,
    evidence_points: u8,
    gates_passed: u8,
    raw_leaks: u8,
    delivered: u8,
    score: u32,
    best_streak: u32,
    revocations: u8,
    state_hash: String,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("version") => {
            println!(
                "neural-boundary-cli {} (core {CORE_VERSION})",
                env!("CARGO_PKG_VERSION")
            );
            EXIT_OK
        }
        Some("schema") => {
            println!("{REPLAY_SCHEMA}");
            println!("hash-algorithm: {HASH_ALGORITHM}");
            EXIT_OK
        }
        Some("verify") => match args.get(1) {
            Some(path) => cmd_verify(Path::new(path), true).code,
            None => {
                eprintln!("verify requires a vector path");
                EXIT_USAGE
            }
        },
        Some("verify-all") => cmd_verify_all(),
        Some("record") => cmd_record(&args[1..]),
        Some("trace") => match args.get(1) {
            Some(path) => cmd_trace(Path::new(path)),
            None => {
                eprintln!("trace requires a vector path");
                EXIT_USAGE
            }
        },
        Some("search") => cmd_search(&args[1..]),
        Some("--help") | Some("-h") | Some("help") | None => {
            print_usage();
            EXIT_OK
        }
        Some(other) => {
            eprintln!("unknown subcommand: {other}");
            print_usage();
            EXIT_USAGE
        }
    };
    process::exit(code);
}

fn print_usage() {
    println!("neural-boundary-cli {}", env!("CARGO_PKG_VERSION"));
    println!("Replay verifier and conformance toolkit for the Neural Boundary Game.\n");
    println!("USAGE:");
    println!("  neural-boundary-cli version");
    println!("  neural-boundary-cli schema");
    println!("  neural-boundary-cli verify <path>");
    println!("  neural-boundary-cli verify-all");
    println!("  neural-boundary-cli record --seed N --mode M [--difficulty D]");
    println!("      [--policy clean|idle|lapse] [--date YYYY-MM-DD] [--max-ticks N]");
    println!("      [--title S] [--summary S] --out PATH");
    println!("  neural-boundary-cli trace <path>");
    println!("  neural-boundary-cli search --mode M --want REASON [--policy P]");
    println!("      [--difficulty D] [--from A] [--to B] [--target T,R,I]");
    println!("      [--min-revocations N] [--max-ticks N]\n");
    println!("EXIT CODES: 0 ok · 2 usage · 3 malformed · 4 compatibility ·");
    println!("            5 state mismatch · 6 checksum mismatch · 7 internal");
}

// ---------------------------------------------------------------------------
// verify
// ---------------------------------------------------------------------------

struct VerifyOutcome {
    code: i32,
}

fn cmd_verify(path: &Path, print_block: bool) -> VerifyOutcome {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            return VerifyOutcome { code: EXIT_USAGE };
        }
    };
    let replay: ReplayFile = match serde_json::from_str(&raw) {
        Ok(replay) => replay,
        Err(error) => {
            eprintln!("malformed replay {}: {error}", path.display());
            return VerifyOutcome {
                code: EXIT_MALFORMED,
            };
        }
    };

    // Compatibility gates (exit 4).
    if replay.schema != REPLAY_SCHEMA {
        eprintln!(
            "{}: schema {:?} is not {REPLAY_SCHEMA:?}",
            path.display(),
            replay.schema
        );
        return VerifyOutcome { code: EXIT_COMPAT };
    }
    if replay.hash_algorithm != HASH_ALGORITHM {
        eprintln!(
            "{}: hash algorithm {:?} is not {HASH_ALGORITHM:?}",
            path.display(),
            replay.hash_algorithm
        );
        return VerifyOutcome { code: EXIT_COMPAT };
    }
    if replay.core_version != CORE_VERSION {
        eprintln!(
            "{}: core version {:?} is not {CORE_VERSION:?}",
            path.display(),
            replay.core_version
        );
        return VerifyOutcome { code: EXIT_COMPAT };
    }
    if replay.tick_rate != TICKS_PER_SECOND {
        eprintln!("{}: tick rate mismatch", path.display());
        return VerifyOutcome { code: EXIT_COMPAT };
    }
    let Some(mode) = RunMode::from_name(&replay.mode) else {
        eprintln!("{}: unknown mode {:?}", path.display(), replay.mode);
        return VerifyOutcome {
            code: EXIT_MALFORMED,
        };
    };
    let Some(difficulty) = Difficulty::from_name(&replay.difficulty) else {
        eprintln!(
            "{}: unknown difficulty {:?}",
            path.display(),
            replay.difficulty
        );
        return VerifyOutcome {
            code: EXIT_MALFORMED,
        };
    };
    if mode == RunMode::Daily {
        let Some(date) = replay.date.as_deref() else {
            eprintln!("{}: daily replay requires a date field", path.display());
            return VerifyOutcome {
                code: EXIT_MALFORMED,
            };
        };
        match parse_date(date) {
            Some((year, month, day)) => {
                let expected_seed = daily_seed(year, month, day);
                if expected_seed != replay.seed {
                    eprintln!(
                        "{}: seed {} does not match daily_seed({date}) = {expected_seed}",
                        path.display(),
                        replay.seed
                    );
                    return VerifyOutcome { code: EXIT_COMPAT };
                }
            }
            None => {
                eprintln!("{}: invalid date {date:?}", path.display());
                return VerifyOutcome {
                    code: EXIT_MALFORMED,
                };
            }
        }
    }

    // Structural validation of the input script (exit 3).
    let mut inputs = Vec::with_capacity(replay.inputs.len());
    let mut last_tick = 0u32;
    for entry in &replay.inputs {
        if entry.tick == 0 || entry.tick <= last_tick {
            eprintln!(
                "{}: input ticks must be strictly increasing (tick {})",
                path.display(),
                entry.tick
            );
            return VerifyOutcome {
                code: EXIT_MALFORMED,
            };
        }
        if entry.tick > replay.expected.terminal_tick {
            eprintln!(
                "{}: input at tick {} is past terminal_tick",
                path.display(),
                entry.tick
            );
            return VerifyOutcome {
                code: EXIT_MALFORMED,
            };
        }
        if entry.lane >= LANES {
            eprintln!(
                "{}: lane out of range at tick {}",
                path.display(),
                entry.tick
            );
            return VerifyOutcome {
                code: EXIT_MALFORMED,
            };
        }
        let Some(action) = Action::from_name(&entry.action) else {
            eprintln!(
                "{}: unknown action {:?} at tick {}",
                path.display(),
                entry.action,
                entry.tick
            );
            return VerifyOutcome {
                code: EXIT_MALFORMED,
            };
        };
        inputs.push(RecordedInput {
            tick: entry.tick,
            lane: entry.lane,
            action,
        });
        last_tick = entry.tick;
    }
    if TerminalReason::from_schema_str(&replay.expected.terminal_reason).is_none() {
        eprintln!(
            "{}: unknown terminal_reason {:?}",
            path.display(),
            replay.expected.terminal_reason
        );
        return VerifyOutcome {
            code: EXIT_MALFORMED,
        };
    }

    // Deterministic re-execution (exit 5 on any divergence).
    let config = SimulationConfig {
        seed: replay.seed,
        mode,
        difficulty,
    };
    let summary = bot::replay_script(config, &inputs, replay.expected.terminal_tick);

    let mut mismatches = Vec::new();
    compare(
        &mut mismatches,
        "terminal_tick",
        replay.expected.terminal_tick,
        summary.terminal_tick,
    );
    compare(
        &mut mismatches,
        "status",
        replay.expected.status.as_str(),
        summary.status.as_str(),
    );
    let actual_reason = match summary.status {
        Status::Terminal(reason) => reason.schema_str(),
        Status::Running => "running",
    };
    compare(
        &mut mismatches,
        "terminal_reason",
        replay.expected.terminal_reason.as_str(),
        actual_reason,
    );
    compare(
        &mut mismatches,
        "boundary",
        replay.expected.boundary.as_str(),
        summary.status.boundary(),
    );
    compare(
        &mut mismatches,
        "grade",
        replay.expected.grade.as_str(),
        summary.grade.name(),
    );
    compare(
        &mut mismatches,
        "trust",
        replay.expected.trust,
        summary.trust,
    );
    compare(&mut mismatches, "risk", replay.expected.risk, summary.risk);
    compare(
        &mut mismatches,
        "integrity",
        replay.expected.integrity,
        summary.integrity,
    );
    compare(
        &mut mismatches,
        "evidence_level",
        replay.expected.evidence_level.as_str(),
        summary.evidence_level.as_str(),
    );
    compare(
        &mut mismatches,
        "evidence_points",
        replay.expected.evidence_points,
        summary.evidence_points,
    );
    compare(
        &mut mismatches,
        "gates_passed",
        replay.expected.gates_passed,
        summary.gates_passed,
    );
    compare(
        &mut mismatches,
        "raw_leaks",
        replay.expected.raw_leaks,
        summary.raw_leaks,
    );
    compare(
        &mut mismatches,
        "delivered",
        replay.expected.delivered,
        summary.delivered,
    );
    compare(
        &mut mismatches,
        "score",
        replay.expected.score,
        summary.score,
    );
    compare(
        &mut mismatches,
        "best_streak",
        replay.expected.best_streak,
        summary.best_streak,
    );
    compare(
        &mut mismatches,
        "revocations",
        replay.expected.revocations,
        summary.revocations,
    );
    compare(
        &mut mismatches,
        "state_hash",
        replay.expected.state_hash.to_ascii_lowercase().as_str(),
        format!("0x{:016x}", summary.state_hash).as_str(),
    );

    if mismatches.is_empty() {
        if print_block {
            print_verify_block(&replay, &summary);
        }
        VerifyOutcome { code: EXIT_OK }
    } else {
        eprintln!("Replay FAILED ({})", path.display());
        for line in mismatches {
            eprintln!("  {line}");
        }
        VerifyOutcome {
            code: EXIT_STATE_MISMATCH,
        }
    }
}

fn print_verify_block(replay: &ReplayFile, summary: &RunSummary) {
    println!("Replay OK");
    println!("Mode: {} ({})", replay.mode, replay.difficulty);
    println!(
        "Terminal: {} at tick {}",
        replay.expected.terminal_reason, summary.terminal_tick
    );
    println!("Grade: {}", summary.grade.name());
    println!(
        "Trust {} | Risk {} | Integrity {} | Evidence {} | Gates {}/5 | Leaks {} | Score {}",
        summary.trust,
        summary.risk,
        summary.integrity,
        summary.evidence_level.as_str(),
        summary.gates_passed,
        summary.raw_leaks,
        summary.score
    );
    println!("Boundary status: {}", summary.status.boundary());
    println!("State hash: 0x{:016x}", summary.state_hash);
}

fn compare<T: PartialEq + std::fmt::Display>(
    out: &mut Vec<String>,
    field: &str,
    expected: T,
    actual: T,
) {
    if expected != actual {
        out.push(format!("{field}: expected {expected}, got {actual}"));
    }
}

fn parse_date(date: &str) -> Option<(u16, u8, u8)> {
    let mut parts = date.split('-');
    let year: u16 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

// ---------------------------------------------------------------------------
// verify-all
// ---------------------------------------------------------------------------

fn cmd_verify_all() -> i32 {
    // 1. File integrity: SHA-256 checksums, verified in Rust.
    let checksum_text = match fs::read_to_string(CHECKSUM_FILE) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("failed to read {CHECKSUM_FILE}: {error}");
            return EXIT_CHECKSUM;
        }
    };
    let mut listed: Vec<(String, String)> = Vec::new();
    for line in checksum_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((digest, name)) = line.split_once(char::is_whitespace) else {
            eprintln!("{CHECKSUM_FILE}: malformed line {line:?}");
            return EXIT_CHECKSUM;
        };
        listed.push((digest.to_ascii_lowercase(), name.trim().to_string()));
    }

    let mut vector_paths: Vec<PathBuf> = match fs::read_dir(VECTOR_DIR) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect(),
        Err(error) => {
            eprintln!("failed to read {VECTOR_DIR}: {error}");
            return EXIT_USAGE;
        }
    };
    vector_paths.sort();

    let mut checksum_failures = 0u32;
    for path in &vector_paths {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("failed to read {}: {error}", path.display());
                return EXIT_CHECKSUM;
            }
        };
        let digest = hex_digest(&bytes);
        match listed.iter().find(|(_, listed_name)| *listed_name == name) {
            None => {
                eprintln!("checksum FAIL  {name}: missing from {CHECKSUM_FILE}");
                checksum_failures += 1;
            }
            Some((expected, _)) if *expected != digest => {
                eprintln!("checksum FAIL  {name}: digest mismatch");
                checksum_failures += 1;
            }
            Some(_) => println!("checksum OK    {name}"),
        }
    }
    for (_, name) in &listed {
        if !vector_paths
            .iter()
            .any(|path| path.file_name().unwrap_or_default().to_string_lossy() == *name)
        {
            eprintln!("checksum FAIL  {name}: listed but absent");
            checksum_failures += 1;
        }
    }
    if checksum_failures > 0 {
        eprintln!("verify-all: {checksum_failures} checksum failure(s)");
        return EXIT_CHECKSUM;
    }

    // 2. Deterministic verification of every vector.
    let mut worst = EXIT_OK;
    for path in &vector_paths {
        let outcome = cmd_verify(path, false);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if outcome.code == EXIT_OK {
            println!("replay   OK    {name}");
        } else {
            println!("replay   FAIL  {name} (exit {})", outcome.code);
            worst = worst.max(outcome.code);
        }
    }
    if worst == EXIT_OK {
        println!(
            "verify-all: {} vector(s) verified against {REPLAY_SCHEMA}",
            vector_paths.len()
        );
    }
    worst
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

// ---------------------------------------------------------------------------
// record
// ---------------------------------------------------------------------------

fn cmd_record(args: &[String]) -> i32 {
    let mut seed: Option<u64> = None;
    let mut mode = RunMode::Standard;
    let mut difficulty = Difficulty::Standard;
    let mut policy = Policy::Clean;
    let mut max_ticks = DEFAULT_MAX_TICKS;
    let mut title = String::new();
    let mut summary_text: Option<String> = None;
    let mut date: Option<String> = None;
    let mut out_path = String::new();

    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--seed" => seed = Some(parse_or_exit(iter.next(), "--seed")),
            "--mode" => mode = parse_mode(iter.next()),
            "--difficulty" => difficulty = parse_difficulty(iter.next()),
            "--policy" => {
                let raw = required(iter.next(), "--policy");
                policy = Policy::from_name(&raw).unwrap_or_else(|| {
                    eprintln!("--policy: expected clean|idle|lapse, got {raw:?}");
                    process::exit(EXIT_USAGE);
                });
            }
            "--max-ticks" => max_ticks = parse_or_exit(iter.next(), "--max-ticks"),
            "--title" => title = required(iter.next(), "--title"),
            "--summary" => summary_text = Some(required(iter.next(), "--summary")),
            "--date" => date = Some(required(iter.next(), "--date")),
            "--out" => out_path = required(iter.next(), "--out"),
            other => {
                eprintln!("unknown flag: {other}");
                return EXIT_USAGE;
            }
        }
    }
    if out_path.is_empty() {
        eprintln!("record requires --out PATH");
        return EXIT_USAGE;
    }
    if mode == RunMode::Daily {
        let Some(date_str) = date.as_deref() else {
            eprintln!("daily mode requires --date YYYY-MM-DD");
            return EXIT_USAGE;
        };
        let Some((year, month, day)) = parse_date(date_str) else {
            eprintln!("--date: invalid value {date_str:?}");
            return EXIT_USAGE;
        };
        seed = Some(daily_seed(year, month, day));
    }
    let Some(seed) = seed else {
        eprintln!("record requires --seed (or --mode daily with --date)");
        return EXIT_USAGE;
    };

    let config = SimulationConfig {
        seed,
        mode,
        difficulty,
    };
    let result = bot::run_policy(config, policy, max_ticks);
    let summary = result.summary;
    if summary.status == Status::Running {
        eprintln!("policy run did not terminate within {max_ticks} ticks; not writing");
        return EXIT_USAGE;
    }

    if title.is_empty() {
        title = format!(
            "{} {} run — {}",
            mode.name(),
            policy.name(),
            summary.status.boundary()
        );
    }

    let file = ReplayFile {
        schema: REPLAY_SCHEMA.to_string(),
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        core_version: CORE_VERSION.to_string(),
        hash_algorithm: HASH_ALGORITHM.to_string(),
        title,
        generated_by: format!(
            "neural-boundary-cli record --policy {} --mode {} --difficulty {} --seed {seed}",
            policy.name(),
            mode.name(),
            difficulty.name()
        ),
        seed,
        mode: mode.name().to_string(),
        difficulty: difficulty.name().to_string(),
        tick_rate: TICKS_PER_SECOND,
        date,
        inputs: result
            .inputs
            .iter()
            .map(|input| InputEntry {
                tick: input.tick,
                lane: input.lane,
                action: input.action.name().to_string(),
            })
            .collect(),
        expected: expected_from(&summary),
        summary: summary_text,
    };

    let mut json = serde_json::to_string_pretty(&file).expect("serialize replay");
    json.push('\n');
    if let Err(error) = fs::write(&out_path, json) {
        eprintln!("failed to write {out_path}: {error}");
        return EXIT_USAGE;
    }

    println!("Recorded {} input(s) to {out_path}", file.inputs.len());
    print_summary(&summary);
    EXIT_OK
}

fn expected_from(summary: &RunSummary) -> Expected {
    let reason = match summary.status {
        Status::Terminal(reason) => reason.schema_str(),
        Status::Running => "running",
    };
    Expected {
        terminal_tick: summary.terminal_tick,
        status: summary.status.as_str().to_string(),
        terminal_reason: reason.to_string(),
        boundary: summary.status.boundary().to_string(),
        grade: summary.grade.name().to_string(),
        trust: summary.trust,
        risk: summary.risk,
        integrity: summary.integrity,
        evidence_level: summary.evidence_level.as_str().to_string(),
        evidence_points: summary.evidence_points,
        gates_passed: summary.gates_passed,
        raw_leaks: summary.raw_leaks,
        delivered: summary.delivered,
        score: summary.score,
        best_streak: summary.best_streak,
        revocations: summary.revocations,
        state_hash: format!("0x{:016x}", summary.state_hash),
    }
}

fn print_summary(summary: &RunSummary) {
    let reason = match summary.status {
        Status::Terminal(reason) => reason.schema_str(),
        Status::Running => "running",
    };
    println!(
        "Terminal: {} at tick {} ({})",
        reason,
        summary.terminal_tick,
        summary.status.boundary()
    );
    println!("Grade: {}", summary.grade.name());
    println!(
        "Trust {} | Risk {} | Integrity {} | Evidence {} ({} pts) | Gates {}/5 | Leaks {} | Delivered {} | Score {} | Best streak {} | Revocations {}",
        summary.trust,
        summary.risk,
        summary.integrity,
        summary.evidence_level.as_str(),
        summary.evidence_points,
        summary.gates_passed,
        summary.raw_leaks,
        summary.delivered,
        summary.score,
        summary.best_streak,
        summary.revocations
    );
    println!("State hash: 0x{:016x}", summary.state_hash);
}

// ---------------------------------------------------------------------------
// trace
// ---------------------------------------------------------------------------

fn cmd_trace(path: &Path) -> i32 {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            return EXIT_USAGE;
        }
    };
    let replay: ReplayFile = match serde_json::from_str(&raw) {
        Ok(replay) => replay,
        Err(error) => {
            eprintln!("malformed replay {}: {error}", path.display());
            return EXIT_MALFORMED;
        }
    };
    let (Some(mode), Some(difficulty)) = (
        RunMode::from_name(&replay.mode),
        Difficulty::from_name(&replay.difficulty),
    ) else {
        eprintln!("{}: unknown mode or difficulty", path.display());
        return EXIT_MALFORMED;
    };

    let mut simulation = Simulation::new(SimulationConfig {
        seed: replay.seed,
        mode,
        difficulty,
    });
    let mut cursor = 0usize;
    for tick in 1..=replay.expected.terminal_tick {
        let input = if cursor < replay.inputs.len() && replay.inputs[cursor].tick == tick {
            let entry = &replay.inputs[cursor];
            cursor += 1;
            let Some(action) = Action::from_name(&entry.action) else {
                eprintln!("unknown action {:?} at tick {tick}", entry.action);
                return EXIT_MALFORMED;
            };
            neural_boundary_core::Input {
                select_lane: Some(entry.lane),
                action: Some(action),
            }
        } else {
            neural_boundary_core::Input::IDLE
        };
        simulation.step(input);
        for event in simulation.events() {
            println!("{tick:>6}  {event:?}");
        }
        if simulation.status() != Status::Running {
            break;
        }
    }
    print_summary(&bot::summarize(&simulation, 0));
    EXIT_OK
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

fn cmd_search(args: &[String]) -> i32 {
    let mut from = 1u64;
    let mut to = 50_000u64;
    let mut mode = RunMode::Standard;
    let mut difficulty = Difficulty::Standard;
    let mut policy = Policy::Clean;
    let mut want = TerminalReason::Sealed;
    let mut finals: Option<(i32, i32, i32)> = None;
    let mut min_revocations = 0u8;
    let mut max_ticks = DEFAULT_MAX_TICKS;

    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--from" => from = parse_or_exit(iter.next(), "--from"),
            "--to" => to = parse_or_exit(iter.next(), "--to"),
            "--mode" => mode = parse_mode(iter.next()),
            "--difficulty" => difficulty = parse_difficulty(iter.next()),
            "--max-ticks" => max_ticks = parse_or_exit(iter.next(), "--max-ticks"),
            "--min-revocations" => {
                min_revocations = parse_or_exit(iter.next(), "--min-revocations")
            }
            "--policy" => {
                let raw = required(iter.next(), "--policy");
                policy = Policy::from_name(&raw).unwrap_or_else(|| {
                    eprintln!("--policy: expected clean|idle|lapse, got {raw:?}");
                    process::exit(EXIT_USAGE);
                });
            }
            "--want" => {
                let raw = required(iter.next(), "--want");
                want = TerminalReason::from_schema_str(&raw).unwrap_or_else(|| {
                    eprintln!("--want: unknown terminal reason {raw:?}");
                    process::exit(EXIT_USAGE);
                });
            }
            "--target" => {
                let raw = required(iter.next(), "--target");
                let parts: Vec<i32> = raw
                    .split(',')
                    .filter_map(|part| part.trim().parse().ok())
                    .collect();
                if parts.len() != 3 {
                    eprintln!("--target expects trust,risk,integrity");
                    return EXIT_USAGE;
                }
                finals = Some((parts[0], parts[1], parts[2]));
            }
            other => {
                eprintln!("unknown flag: {other}");
                return EXIT_USAGE;
            }
        }
    }

    eprintln!(
        "searching seeds {from}..={to}: mode {} difficulty {} policy {} want {}",
        mode.name(),
        difficulty.name(),
        policy.name(),
        want.schema_str()
    );
    match bot::search_seed(
        mode,
        difficulty,
        policy,
        from,
        to,
        SearchGoal {
            reason: want,
            finals,
            min_revocations,
        },
        max_ticks,
    ) {
        Some((seed, result)) => {
            println!("Seed found: {seed} (0x{seed:x})");
            println!("Inputs: {}", result.inputs.len());
            print_summary(&result.summary);
            EXIT_OK
        }
        None => {
            eprintln!("no seed in range matched the goal");
            1
        }
    }
}

// ---------------------------------------------------------------------------
// flag helpers
// ---------------------------------------------------------------------------

fn required(value: Option<&String>, flag: &str) -> String {
    value.cloned().unwrap_or_else(|| {
        eprintln!("{flag} requires a value");
        process::exit(EXIT_USAGE);
    })
}

fn parse_or_exit<T: std::str::FromStr>(value: Option<&String>, flag: &str) -> T {
    let raw = required(value, flag);
    raw.parse().unwrap_or_else(|_| {
        eprintln!("{flag}: invalid value {raw:?}");
        process::exit(EXIT_USAGE);
    })
}

fn parse_mode(value: Option<&String>) -> RunMode {
    let raw = required(value, "--mode");
    RunMode::from_name(&raw).unwrap_or_else(|| {
        eprintln!("--mode: expected guided|standard|audit|grand|daily, got {raw:?}");
        process::exit(EXIT_USAGE);
    })
}

fn parse_difficulty(value: Option<&String>) -> Difficulty {
    let raw = required(value, "--difficulty");
    Difficulty::from_name(&raw).unwrap_or_else(|| {
        eprintln!("--difficulty: expected calm|standard|intense, got {raw:?}");
        process::exit(EXIT_USAGE);
    })
}

// Grade is referenced through bot::RunSummary; keep the import meaningful.
#[allow(dead_code)]
fn grade_name(grade: Grade) -> &'static str {
    grade.name()
}
