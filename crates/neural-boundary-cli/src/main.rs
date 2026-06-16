// SPDX-FileCopyrightText: 2026 Denis Yermakou
// SPDX-FileContributor: AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//! neural-boundary-cli — replay verifier and conformance toolkit.
//!
//! Schema: neural-boundary-replay-v5.5.12 · Exit codes: 0 ok | 2 usage |
//! 3 malformed | 4 compatibility | 5 hash mismatch | 6 checksum fail | 7 internal

mod bot;

use bot::{Policy, RecordedInput, RunSummary};
use neural_boundary_core::{
    daily_seed, Action, Difficulty, EvidenceLevel, Grade, Mode, Status, TerminalReason,
    ABI_VERSION, ALL_GATES_MASK, CORE_VERSION, HASH_ALGORITHM, LANES, REPLAY_SCHEMA, RNG_ALGORITHM,
    TICKS_PER_SECOND,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, fs, path::Path, path::PathBuf, process};

// Exit codes
const OK: i32 = 0;
const USAGE: i32 = 2;
const MALFORMED: i32 = 3;
const COMPAT: i32 = 4;
const STATE: i32 = 5;
const CHECKSUM: i32 = 6;

const VECTOR_DIR: &str = "vectors";
const CHECKSUM_FILE: &str = "vectors/checksums.sha256";
const MAX_TICKS: u32 = 14_400;

// ── Replay schema (§24) ─────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct ReplayFile {
    schema: String,
    product_version: String,
    core_version: String,
    abi_version: u32,
    hash_algorithm: String,
    rng_algorithm: String,
    tick_rate_hz: u32,
    mode: String,
    difficulty: u8,
    /// Lowercase 16 hex digits (no 0x prefix) — §24.4
    seed: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    title: String,
    generated_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    inputs: Vec<InputEntry>,
    expected: Expected,
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
    grade: String,
    trust: i32,
    risk: i32,
    integrity: i32,
    evidence_level: String,
    evidence_bits: u8,
    gate_mask: u8,
    gates_passed: u8,
    raw_leaks: u8,
    typed_intents: u8,
    quarantined: u32,
    wrong_actions: u32,
    score: u64,
    best_combo: u32,
    revocations: u32,
    /// "0x" + 16 lowercase hex digits
    state_hash: String,
}

// ── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("version") => {
            println!(
                "neural-boundary-cli {} (core {CORE_VERSION})",
                env!("CARGO_PKG_VERSION")
            );
            OK
        }
        Some("schema") => {
            println!("{REPLAY_SCHEMA}");
            println!("hash: {HASH_ALGORITHM}");
            println!("rng: {RNG_ALGORITHM}");
            println!("abi_version: {ABI_VERSION}");
            OK
        }
        Some("verify") => match args.get(1) {
            Some(p) => verify_cmd(Path::new(p), true).0,
            None => {
                eprintln!("verify requires a path");
                USAGE
            }
        },
        Some("verify-all") => verify_all_cmd(),
        Some("record") => record_cmd(&args[1..]),
        Some("trace") => match args.get(1) {
            Some(p) => trace_cmd(Path::new(p)),
            None => {
                eprintln!("trace requires a path");
                USAGE
            }
        },
        Some("search") => search_cmd(&args[1..]),
        Some("--help") | Some("-h") | Some("help") | None => {
            print_usage();
            OK
        }
        Some(other) => {
            eprintln!("unknown subcommand: {other}");
            USAGE
        }
    };
    process::exit(code);
}

fn print_usage() {
    println!("neural-boundary-cli {}", env!("CARGO_PKG_VERSION"));
    println!("Neural Boundary Game v5.5.12 conformance toolkit\n");
    println!("USAGE:");
    println!("  version | schema | verify <path> | verify-all");
    println!("  record --seed HEXSEED --mode MODE --difficulty 0|1|2 [--policy P]");
    println!("         [--date YYYY-MM-DD] [--title S] [--out PATH]");
    println!("  trace <path>");
    println!("  search --mode M --want REASON [--from N] [--to N] [--policy P]");
    println!("         [--difficulty D] [--min-revocations N]\n");
    println!("EXIT: 0 ok  2 usage  3 malformed  4 compat  5 hash  6 checksum  7 internal");
}

// ── verify ──────────────────────────────────────────────────────────────────

fn verify_cmd(path: &Path, print: bool) -> (i32, Option<RunSummary>) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return (USAGE, None);
        }
    };
    let replay: ReplayFile = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: malformed: {e}", path.display());
            return (MALFORMED, None);
        }
    };

    // Compatibility gates (→ exit 4)
    for (field, expected, got) in [
        ("schema", REPLAY_SCHEMA, replay.schema.as_str()),
        (
            "hash_algorithm",
            HASH_ALGORITHM,
            replay.hash_algorithm.as_str(),
        ),
        (
            "rng_algorithm",
            RNG_ALGORITHM,
            replay.rng_algorithm.as_str(),
        ),
        ("core_version", CORE_VERSION, replay.core_version.as_str()),
    ] {
        if got != expected {
            eprintln!("{}: {field} {:?} != {expected:?}", path.display(), got);
            return (COMPAT, None);
        }
    }
    if replay.abi_version != ABI_VERSION {
        eprintln!(
            "{}: abi_version {} != {ABI_VERSION}",
            path.display(),
            replay.abi_version
        );
        return (COMPAT, None);
    }
    if replay.tick_rate_hz != TICKS_PER_SECOND {
        eprintln!("{}: tick_rate_hz mismatch", path.display());
        return (COMPAT, None);
    }

    // Parse mode and difficulty
    let Some(mode) = Mode::from_name(&replay.mode) else {
        eprintln!("{}: unknown mode {:?}", path.display(), replay.mode);
        return (MALFORMED, None);
    };
    let Some(difficulty) = Difficulty::from_u8(replay.difficulty) else {
        eprintln!(
            "{}: difficulty out of range {}",
            path.display(),
            replay.difficulty
        );
        return (MALFORMED, None);
    };

    // Parse seed — hex string (no 0x prefix)
    let seed = match u64::from_str_radix(&replay.seed, 16) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("{}: seed must be 16 hex digits", path.display());
            return (MALFORMED, None);
        }
    };

    // Daily seed verification
    if mode == Mode::Daily {
        let Some(date) = replay.date.as_deref() else {
            eprintln!("{}: daily requires date field", path.display());
            return (MALFORMED, None);
        };
        match parse_date(date) {
            Some((y, m, d)) => {
                let expected_seed = daily_seed(y, m, d);
                if seed != expected_seed {
                    eprintln!(
                        "{}: seed {seed:016x} != daily_seed({date}) = {expected_seed:016x}",
                        path.display()
                    );
                    return (COMPAT, None);
                }
            }
            None => {
                eprintln!("{}: invalid date {date:?}", path.display());
                return (MALFORMED, None);
            }
        }
    }

    // Parse and validate input list
    let mut inputs: Vec<RecordedInput> = Vec::new();
    let mut last_tick = 0u32;
    let terminal_tick = replay.expected.terminal_tick;
    for (i, entry) in replay.inputs.iter().enumerate() {
        if entry.tick == 0 || entry.tick <= last_tick {
            eprintln!(
                "{}: inputs[{i}].tick must be strictly increasing ≥1",
                path.display()
            );
            return (MALFORMED, None);
        }
        if entry.tick > terminal_tick + 600 {
            eprintln!(
                "{}: inputs[{i}].tick {} beyond terminal_tick+600",
                path.display(),
                entry.tick
            );
            return (MALFORMED, None);
        }
        if entry.lane >= LANES {
            eprintln!(
                "{}: inputs[{i}].lane {} ≥ {LANES}",
                path.display(),
                entry.lane
            );
            return (MALFORMED, None);
        }
        let Some(action) = Action::from_name(&entry.action) else {
            eprintln!(
                "{}: inputs[{i}].action {:?} unknown",
                path.display(),
                entry.action
            );
            return (MALFORMED, None);
        };
        inputs.push(RecordedInput {
            tick: entry.tick,
            lane: entry.lane,
            action,
        });
        last_tick = entry.tick;
    }

    if TerminalReason::from_schema_str(&replay.expected.terminal_reason).is_none() {
        eprintln!("{}: unknown terminal_reason", path.display());
        return (MALFORMED, None);
    }

    // Deterministic re-execution
    let summary = bot::replay_script(mode, difficulty, seed, &inputs, terminal_tick + 600);

    let grade = compute_grade(&summary);
    let hash_got = format!("0x{:016x}", summary.state_hash);

    let mut mismatches: Vec<String> = Vec::new();
    cmp_u32(
        &mut mismatches,
        "terminal_tick",
        replay.expected.terminal_tick,
        summary.terminal_tick,
    );
    cmp_str(
        &mut mismatches,
        "status",
        &replay.expected.status,
        summary.status.as_str(),
    );
    cmp_str(
        &mut mismatches,
        "terminal_reason",
        &replay.expected.terminal_reason,
        summary.reason.as_str(),
    );
    cmp_str(
        &mut mismatches,
        "grade",
        &replay.expected.grade,
        grade.name(),
    );
    cmp_i32(
        &mut mismatches,
        "trust",
        replay.expected.trust,
        summary.trust,
    );
    cmp_i32(&mut mismatches, "risk", replay.expected.risk, summary.risk);
    cmp_i32(
        &mut mismatches,
        "integrity",
        replay.expected.integrity,
        summary.integrity,
    );
    cmp_str(
        &mut mismatches,
        "evidence_level",
        &replay.expected.evidence_level,
        EvidenceLevel::from_bits(summary.evidence_bits).as_str(),
    );
    cmp_u8(
        &mut mismatches,
        "evidence_bits",
        replay.expected.evidence_bits,
        summary.evidence_bits,
    );
    cmp_u8(
        &mut mismatches,
        "gate_mask",
        replay.expected.gate_mask,
        summary.gate_mask,
    );
    cmp_u8(
        &mut mismatches,
        "gates_passed",
        replay.expected.gates_passed,
        summary.gate_mask.count_ones() as u8,
    );
    cmp_u8(
        &mut mismatches,
        "raw_leaks",
        replay.expected.raw_leaks,
        summary.raw_leaks,
    );
    cmp_u8(
        &mut mismatches,
        "typed_intents",
        replay.expected.typed_intents,
        summary.typed_intents,
    );
    cmp_u32(
        &mut mismatches,
        "quarantined",
        replay.expected.quarantined,
        summary.quarantined,
    );
    cmp_u32(
        &mut mismatches,
        "wrong_actions",
        replay.expected.wrong_actions,
        summary.wrong_actions,
    );
    cmp_u64(
        &mut mismatches,
        "score",
        replay.expected.score,
        summary.score,
    );
    cmp_u32(
        &mut mismatches,
        "best_combo",
        replay.expected.best_combo,
        summary.best_combo,
    );
    cmp_str(
        &mut mismatches,
        "state_hash",
        &replay.expected.state_hash.to_lowercase(),
        &hash_got.to_lowercase(),
    );

    if mismatches.is_empty() {
        if print {
            println!(
                "Replay OK  {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            println!(
                "  Grade {}  Status {}  Tick {}  Score {}  Hash {}",
                grade.name(),
                summary.status.as_str(),
                summary.terminal_tick,
                summary.score,
                hash_got
            );
        }
        (OK, Some(summary))
    } else {
        eprintln!("Replay FAIL  {}:", path.display());
        for m in &mismatches {
            eprintln!("  {m}");
        }
        (STATE, None)
    }
}

fn cmp_str(out: &mut Vec<String>, f: &str, e: &str, g: &str) {
    if e.to_lowercase() != g.to_lowercase() {
        out.push(format!("{f}: expected {e:?} got {g:?}"));
    }
}
fn cmp_i32(out: &mut Vec<String>, f: &str, e: i32, g: i32) {
    if e != g {
        out.push(format!("{f}: expected {e} got {g}"));
    }
}
fn cmp_u8(out: &mut Vec<String>, f: &str, e: u8, g: u8) {
    if e != g {
        out.push(format!("{f}: expected {e} got {g}"));
    }
}
fn cmp_u32(out: &mut Vec<String>, f: &str, e: u32, g: u32) {
    if e != g {
        out.push(format!("{f}: expected {e} got {g}"));
    }
}
fn cmp_u64(out: &mut Vec<String>, f: &str, e: u64, g: u64) {
    if e != g {
        out.push(format!("{f}: expected {e} got {g}"));
    }
}

fn compute_grade(s: &RunSummary) -> Grade {
    use neural_boundary_core::EvidenceLevel;
    match s.status {
        Status::Unsafe => Grade::Unsafe,
        Status::Breached => Grade::Breached,
        Status::Sealed => {
            let gates = s.gate_mask == ALL_GATES_MASK;
            if gates
                && s.trust >= 900
                && s.risk <= 100
                && s.integrity >= 900
                && EvidenceLevel::from_bits(s.evidence_bits) == EvidenceLevel::L3
                && s.raw_leaks == 0
                && s.wrong_actions == 0
            {
                Grade::Sovereign
            } else if gates
                && s.trust >= 750
                && s.risk <= 250
                && s.integrity >= 750
                && s.raw_leaks == 0
            {
                Grade::Sealed
            } else {
                Grade::Reviewable
            }
        }
        _ => {
            let p = s.gate_mask.count_ones() as u8;
            if p >= 5 && s.integrity >= 650 && s.risk <= 450 {
                Grade::Reviewable
            } else if p >= 3 && s.integrity > 0 && s.risk < 1000 {
                Grade::Degraded
            } else {
                Grade::Breached
            }
        }
    }
}

fn parse_date(s: &str) -> Option<(u16, u8, u8)> {
    let mut parts = s.split('-');
    let y: u16 = parts.next()?.parse().ok()?;
    let m: u8 = parts.next()?.parse().ok()?;
    let d: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

// ── verify-all ──────────────────────────────────────────────────────────────

fn verify_all_cmd() -> i32 {
    // 1. Checksum verification
    let checksum_text = match fs::read_to_string(CHECKSUM_FILE) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {CHECKSUM_FILE}: {e}");
            return CHECKSUM;
        }
    };
    let mut listed: Vec<(String, String)> = Vec::new();
    for line in checksum_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((digest, name)) = line.split_once(char::is_whitespace) else {
            eprintln!("{CHECKSUM_FILE}: malformed line");
            return CHECKSUM;
        };
        listed.push((digest.to_lowercase(), name.trim().to_string()));
    }
    let mut vectors: Vec<PathBuf> = match fs::read_dir(VECTOR_DIR) {
        Ok(e) => e
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect(),
        Err(e) => {
            eprintln!("{VECTOR_DIR}: {e}");
            return USAGE;
        }
    };
    vectors.sort();

    let mut bad_checksums = 0u32;
    for path in &vectors {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let bytes = fs::read(path).unwrap_or_default();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        match listed.iter().find(|(_, n)| *n == name) {
            None => {
                eprintln!("CHECKSUM FAIL  {name}: not in ledger");
                bad_checksums += 1;
            }
            Some((exp, _)) if *exp != digest => {
                eprintln!("CHECKSUM FAIL  {name}: digest mismatch");
                bad_checksums += 1;
            }
            Some(_) => println!("checksum OK    {name}"),
        }
    }
    for (_, name) in &listed {
        if !vectors
            .iter()
            .any(|p| p.file_name().unwrap_or_default().to_string_lossy() == *name)
        {
            eprintln!("CHECKSUM FAIL  {name}: listed but missing");
            bad_checksums += 1;
        }
    }
    if bad_checksums > 0 {
        eprintln!("{bad_checksums} checksum failure(s)");
        return CHECKSUM;
    }

    // 2. Replay verification
    let mut worst = OK;
    for path in &vectors {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let (code, _) = verify_cmd(path, false);
        if code == OK {
            println!("replay   OK    {name}");
        } else {
            eprintln!("replay   FAIL  {name}  (exit {code})");
            worst = worst.max(code);
        }
    }
    if worst == OK {
        println!(
            "\nverify-all OK: {} vector(s) · schema {REPLAY_SCHEMA}",
            vectors.len()
        );
    }
    worst
}

// ── record ──────────────────────────────────────────────────────────────────

fn record_cmd(args: &[String]) -> i32 {
    let mut seed_hex: Option<String> = None;
    let mut mode = Mode::Standard;
    let mut difficulty = Difficulty::Standard;
    let mut policy = Policy::Clean;
    let mut date: Option<String> = None;
    let mut title: Option<String> = None;
    let mut summary_txt: Option<String> = None;
    let mut out_path = String::new();
    let mut max_ticks = MAX_TICKS;

    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--seed" => seed_hex = Some(req(iter.next(), "--seed")),
            "--mode" => {
                let raw = req(iter.next(), "--mode");
                mode = Mode::from_name(&raw).unwrap_or_else(|| {
                    eprintln!("unknown mode {raw:?}");
                    process::exit(USAGE);
                });
            }
            "--difficulty" => {
                let v: u8 = req(iter.next(), "--difficulty")
                    .parse()
                    .unwrap_or_else(|_| {
                        eprintln!("difficulty must be 0|1|2");
                        process::exit(USAGE);
                    });
                difficulty = Difficulty::from_u8(v).unwrap_or(Difficulty::Standard);
            }
            "--policy" => {
                let raw = req(iter.next(), "--policy");
                policy = Policy::from_name(&raw).unwrap_or_else(|| {
                    eprintln!("unknown policy {raw:?}");
                    process::exit(USAGE);
                });
            }
            "--date" => date = Some(req(iter.next(), "--date")),
            "--title" => title = Some(req(iter.next(), "--title")),
            "--summary" => summary_txt = Some(req(iter.next(), "--summary")),
            "--out" => out_path = req(iter.next(), "--out"),
            "--max-ticks" => {
                max_ticks = req(iter.next(), "--max-ticks").parse().unwrap_or(MAX_TICKS)
            }
            other => {
                eprintln!("unknown flag: {other}");
                return USAGE;
            }
        }
    }

    let seed = if mode == Mode::Daily {
        let date_str = date.as_deref().unwrap_or_else(|| {
            eprintln!("daily requires --date YYYY-MM-DD");
            process::exit(USAGE);
        });
        let (y, m, d) = parse_date(date_str).unwrap_or_else(|| {
            eprintln!("invalid date {date_str:?}");
            process::exit(USAGE);
        });
        daily_seed(y, m, d)
    } else if let Some(hex) = &seed_hex {
        u64::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or_else(|_| {
            eprintln!("--seed must be hex");
            process::exit(USAGE);
        })
    } else {
        bot::mix_seed(42, 1)
    };

    if out_path.is_empty() {
        eprintln!("--out required");
        return USAGE;
    }

    let result = bot::run_policy(mode, difficulty, seed, policy, max_ticks);
    let s = &result.summary;
    if s.status == Status::Running {
        eprintln!("run did not terminate in {max_ticks} ticks");
        return USAGE;
    }

    let grade = compute_grade(s);
    let file = ReplayFile {
        schema: REPLAY_SCHEMA.to_string(),
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        core_version: CORE_VERSION.to_string(),
        abi_version: ABI_VERSION,
        hash_algorithm: HASH_ALGORITHM.to_string(),
        rng_algorithm: RNG_ALGORITHM.to_string(),
        tick_rate_hz: TICKS_PER_SECOND,
        mode: mode.name().to_string(),
        difficulty: difficulty.code(),
        seed: format!("{seed:016x}"),
        date,
        title: title.unwrap_or_else(|| {
            format!(
                "{} {} {} seed {:016x}",
                mode.name(),
                policy.name(),
                s.status.as_str(),
                seed
            )
        }),
        generated_by: format!(
            "neural-boundary-cli record --mode {} --difficulty {} --policy {} --seed {:016x}",
            mode.name(),
            difficulty.code(),
            policy.name(),
            seed
        ),
        summary: summary_txt,
        inputs: result
            .inputs
            .iter()
            .map(|i| InputEntry {
                tick: i.tick,
                lane: i.lane,
                action: i.action.name().to_string(),
            })
            .collect(),
        expected: Expected {
            terminal_tick: s.terminal_tick,
            status: s.status.as_str().to_string(),
            terminal_reason: s.reason.as_str().to_string(),
            grade: grade.name().to_string(),
            trust: s.trust,
            risk: s.risk,
            integrity: s.integrity,
            evidence_level: EvidenceLevel::from_bits(s.evidence_bits)
                .as_str()
                .to_string(),
            evidence_bits: s.evidence_bits,
            gate_mask: s.gate_mask,
            gates_passed: s.gate_mask.count_ones() as u8,
            raw_leaks: s.raw_leaks,
            typed_intents: s.typed_intents,
            quarantined: s.quarantined,
            wrong_actions: s.wrong_actions,
            score: s.score,
            best_combo: s.best_combo,
            revocations: s.revocations,
            state_hash: format!("0x{:016x}", s.state_hash),
        },
    };

    let mut json = serde_json::to_string_pretty(&file).expect("serialize");
    json.push('\n');
    if let Err(e) = fs::write(&out_path, json) {
        eprintln!("write {out_path}: {e}");
        return USAGE;
    }
    println!("Recorded {} input(s) → {out_path}", file.inputs.len());
    println!(
        "Grade {}  Status {}  Tick {}  Score {}  Hash 0x{:016x}",
        grade.name(),
        s.status.as_str(),
        s.terminal_tick,
        s.score,
        s.state_hash
    );
    OK
}

// ── trace ────────────────────────────────────────────────────────────────────

fn trace_cmd(path: &Path) -> i32 {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return USAGE;
        }
    };
    let replay: ReplayFile = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("malformed: {e}");
            return MALFORMED;
        }
    };
    let (Some(mode), Some(difficulty)) = (
        Mode::from_name(&replay.mode),
        Difficulty::from_u8(replay.difficulty),
    ) else {
        eprintln!("unknown mode or difficulty");
        return MALFORMED;
    };
    let Ok(seed) = u64::from_str_radix(&replay.seed, 16) else {
        eprintln!("invalid seed");
        return MALFORMED;
    };
    let mut inputs: Vec<RecordedInput> = Vec::new();
    for entry in &replay.inputs {
        let Some(action) = Action::from_name(&entry.action) else {
            continue;
        };
        inputs.push(RecordedInput {
            tick: entry.tick,
            lane: entry.lane,
            action,
        });
    }
    let s = bot::replay_script(
        mode,
        difficulty,
        seed,
        &inputs,
        replay.expected.terminal_tick + 600,
    );
    println!(
        "Grade {}  Status {}  Tick {}  Hash 0x{:016x}",
        compute_grade(&s).name(),
        s.status.as_str(),
        s.terminal_tick,
        s.state_hash
    );
    OK
}

// ── search ───────────────────────────────────────────────────────────────────

fn search_cmd(args: &[String]) -> i32 {
    let mut from = 1u64;
    let mut to = 50_000u64;
    let mut mode = Mode::Standard;
    let mut difficulty = Difficulty::Standard;
    let mut policy = Policy::Clean;
    let mut want = TerminalReason::SuccessRelease;
    let mut min_revocations = 0u32;
    let mut max_ticks = MAX_TICKS;

    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--from" => from = req(iter.next(), "--from").parse().unwrap_or(1u64),
            "--to" => to = req(iter.next(), "--to").parse().unwrap_or(50_000u64),
            "--mode" => {
                let raw = req(iter.next(), "--mode");
                mode = Mode::from_name(&raw).unwrap_or(Mode::Standard);
            }
            "--difficulty" => {
                let v: u8 = req(iter.next(), "--difficulty").parse().unwrap_or(1);
                difficulty = Difficulty::from_u8(v).unwrap_or(Difficulty::Standard);
            }
            "--policy" => {
                let raw = req(iter.next(), "--policy");
                policy = Policy::from_name(&raw).unwrap_or(Policy::Clean);
            }
            "--want" => {
                let raw = req(iter.next(), "--want");
                want =
                    TerminalReason::from_schema_str(&raw).unwrap_or(TerminalReason::SuccessRelease);
            }
            "--min-revocations" => {
                min_revocations = req(iter.next(), "--min-revocations").parse().unwrap_or(0)
            }
            "--max-ticks" => {
                max_ticks = req(iter.next(), "--max-ticks").parse().unwrap_or(MAX_TICKS)
            }
            other => {
                eprintln!("unknown flag: {other}");
                return USAGE;
            }
        }
    }
    eprintln!(
        "searching {} seeds [{from}..{to}] mode={} difficulty={} policy={} want={}",
        to - from + 1,
        mode.name(),
        difficulty.code(),
        policy.name(),
        want.as_str()
    );
    match bot::search_seed(
        mode,
        difficulty,
        policy,
        bot::SearchGoal {
            from,
            to,
            want_reason: want,
            min_revocations,
            max_ticks,
        },
    ) {
        Some((seed, result)) => {
            let s = &result.summary;
            println!("Seed {:016x} ({seed})", seed);
            println!(
                "Grade {}  Status {}  Tick {}  Score {}  Hash 0x{:016x}",
                compute_grade(s).name(),
                s.status.as_str(),
                s.terminal_tick,
                s.score,
                s.state_hash
            );
            OK
        }
        None => {
            eprintln!("no seed found");
            1
        }
    }
}

fn req(val: Option<&String>, flag: &str) -> String {
    val.cloned().unwrap_or_else(|| {
        eprintln!("{flag} requires a value");
        process::exit(USAGE);
    })
}
