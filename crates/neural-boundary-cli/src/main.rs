// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
// See LICENSE and IP_NOTICE.md for details.

//! Conformance toolkit (§16.2): verify-all, run-vector, record, dump-scenario,
//! hash-state, explain-grade, generate-daily-vector, release-evidence.
//! Replay schema: `neural-boundary-replay-v4` (§19.2).

mod bot;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use neural_boundary_core::{
    daily_seed, replay_run, scenario_by_id, verify_vector, EndReason, ExpectedOutcome, GameState,
    PlayerAction, ReplayAction, RunStatus, SovereigntyGrade, CORE_VERSION, REPLAY_SCHEMA,
    SCENARIOS, SCENARIO_COUNT, TICK_RATE_HZ,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PRODUCT: &str = "Neural Boundary Game";

// ── Replay vector schema (§19.2) ─────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct Vector {
    schema: String,
    product: String,
    version: String,
    scenario_id: u32,
    seed: String,
    tick_rate: u32,
    #[serde(default)]
    title: String,
    #[serde(default)]
    generated_by: String,
    actions: Vec<Step>,
    expected: Expected,
}

#[derive(Serialize, Deserialize)]
struct Step {
    tick: u32,
    action: String,
}

#[derive(Serialize, Deserialize)]
struct Expected {
    grade: String,
    reason: String,
    score_min: i32,
    state_hash: String,
}

fn grade_from_label(s: &str) -> Option<SovereigntyGrade> {
    Some(match s {
        "PENDING" => SovereigntyGrade::Pending,
        "SEALED" => SovereigntyGrade::Sealed,
        "REVIEWABLE" => SovereigntyGrade::Reviewable,
        "BREACHED" => SovereigntyGrade::Breached,
        "UNSAFE" => SovereigntyGrade::Unsafe,
        _ => return None,
    })
}

fn parse_seed(s: &str) -> Option<u64> {
    let s = s.trim().trim_start_matches("0x");
    if s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        u64::from_str_radix(s, 16).ok()
    } else {
        s.parse::<u64>().ok()
    }
}

fn hash_hex(h: u64) -> String {
    format!("0x{h:016x}")
}

fn steps_to_actions(steps: &[Step]) -> Result<Vec<ReplayAction>, String> {
    let mut out = Vec::with_capacity(steps.len());
    for s in steps {
        let action = PlayerAction::from_name(&s.action)
            .ok_or_else(|| format!("unknown action {:?}", s.action))?;
        out.push(ReplayAction {
            tick: s.tick,
            action,
        });
    }
    Ok(out)
}

// ── Vector build / verify ────────────────────────────────────────────────────

fn build_vector(
    scenario_id: u32,
    seed: u64,
    title: &str,
    actions: &[ReplayAction],
    g: &GameState,
) -> Vector {
    Vector {
        schema: REPLAY_SCHEMA.to_string(),
        product: PRODUCT.to_string(),
        version: CORE_VERSION.to_string(),
        scenario_id,
        seed: format!("{seed:016x}"),
        tick_rate: TICK_RATE_HZ,
        title: title.to_string(),
        generated_by: format!("neural-boundary-cli {CORE_VERSION}"),
        actions: actions
            .iter()
            .map(|a| Step {
                tick: a.tick,
                action: a.action.name().to_string(),
            })
            .collect(),
        expected: Expected {
            grade: g.grade().label().to_string(),
            reason: g.end_reason().label().to_string(),
            score_min: g.score(),
            state_hash: hash_hex(g.state_hash()),
        },
    }
}

fn replay_vector(v: &Vector) -> Result<GameState, String> {
    if v.schema != REPLAY_SCHEMA {
        return Err(format!("schema {:?} != {REPLAY_SCHEMA}", v.schema));
    }
    if v.tick_rate != TICK_RATE_HZ {
        return Err(format!("tick_rate {} != {TICK_RATE_HZ}", v.tick_rate));
    }
    let seed = parse_seed(&v.seed).ok_or_else(|| format!("bad seed {:?}", v.seed))?;
    let actions = steps_to_actions(&v.actions)?;
    replay_run(v.scenario_id, seed, &actions)
        .ok_or_else(|| format!("unknown scenario {}", v.scenario_id))
}

fn expected_from(v: &Vector) -> Result<ExpectedOutcome, String> {
    let grade = grade_from_label(&v.expected.grade)
        .ok_or_else(|| format!("bad grade {:?}", v.expected.grade))?;
    let reason = EndReason::from_label(&v.expected.reason)
        .ok_or_else(|| format!("bad reason {:?}", v.expected.reason))?;
    let state_hash = u64::from_str_radix(v.expected.state_hash.trim_start_matches("0x"), 16)
        .map_err(|_| format!("bad state_hash {:?}", v.expected.state_hash))?;
    Ok(ExpectedOutcome {
        grade,
        reason,
        score_min: v.expected.score_min,
        state_hash,
    })
}

// ── Paths ────────────────────────────────────────────────────────────────────

fn vectors_dir() -> PathBuf {
    PathBuf::from("vectors")
}

fn list_vectors() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(vectors_dir())
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn sha256_file(p: &Path) -> std::io::Result<String> {
    let bytes = fs::read(p)?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(format!("{:x}", h.finalize()))
}

fn rewrite_checksums() -> std::io::Result<()> {
    let mut lines = String::new();
    for p in list_vectors() {
        let name = p.file_name().unwrap().to_string_lossy();
        lines.push_str(&format!("{}  {}\n", sha256_file(&p)?, name));
    }
    fs::write(vectors_dir().join("checksums.sha256"), lines)
}

// ── Commands ─────────────────────────────────────────────────────────────────

fn verify_all_cmd() -> ExitCode {
    let dir = vectors_dir();
    let checks = dir.join("checksums.sha256");
    let vectors = list_vectors();
    if vectors.is_empty() {
        eprintln!("no vectors found in {}", dir.display());
        return ExitCode::from(2);
    }
    // 1. checksum integrity
    let mut listed = std::collections::HashMap::new();
    if let Ok(txt) = fs::read_to_string(&checks) {
        for line in txt.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                listed.insert(parts[1].to_string(), parts[0].to_lowercase());
            }
        }
    } else {
        eprintln!("missing {}", checks.display());
        return ExitCode::from(2);
    }
    let mut failures = 0;
    for p in &vectors {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        match (sha256_file(p), listed.get(&name)) {
            (Ok(d), Some(rec)) if &d == rec => {}
            (Ok(_), Some(_)) => {
                eprintln!("checksum  FAIL  {name} (digest mismatch)");
                failures += 1;
            }
            (Ok(_), None) => {
                eprintln!("checksum  FAIL  {name} (not listed)");
                failures += 1;
            }
            (Err(e), _) => {
                eprintln!("checksum  FAIL  {name} ({e})");
                failures += 1;
            }
        }
    }
    // 2. deterministic replay
    for p in &vectors {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        match verify_one(p) {
            Ok(()) => println!("replay   OK    {name}"),
            Err(e) => {
                eprintln!("replay   FAIL  {name}: {e}");
                failures += 1;
            }
        }
    }
    if failures == 0 {
        println!(
            "\nverify-all OK: {} vector(s) · schema {REPLAY_SCHEMA}",
            vectors.len()
        );
        ExitCode::SUCCESS
    } else {
        eprintln!("\nverify-all FAILED: {failures} problem(s)");
        ExitCode::from(3)
    }
}

fn verify_one(path: &Path) -> Result<(), String> {
    let txt = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: Vector = serde_json::from_str(&txt).map_err(|e| e.to_string())?;
    let g = replay_vector(&v)?;
    let expected = expected_from(&v)?;
    if verify_vector(&g, &expected) {
        Ok(())
    } else {
        Err(format!(
            "grade {}/{} reason {}/{} score {}>={} hash {}/{}",
            g.grade().label(),
            v.expected.grade,
            g.end_reason().label(),
            v.expected.reason,
            g.score(),
            v.expected.score_min,
            hash_hex(g.state_hash()),
            v.expected.state_hash
        ))
    }
}

fn run_vector_cmd(args: &[String]) -> ExitCode {
    let Some(file) = args.first() else {
        eprintln!("usage: run-vector <file.json>");
        return ExitCode::from(2);
    };
    let txt = match fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::from(2);
        }
    };
    let v: Vector = match serde_json::from_str(&txt) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse: {e}");
            return ExitCode::from(2);
        }
    };
    match replay_vector(&v) {
        Ok(g) => {
            print_run(&g);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(3)
        }
    }
}

fn record_cmd(args: &[String]) -> ExitCode {
    let mut scenario = 1u32;
    let mut seed = 1u64;
    let mut policy = bot::Policy::Clean;
    let mut out: Option<String> = None;
    let mut title = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => {
                i += 1;
                scenario = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(1);
            }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| parse_seed(s)).unwrap_or(1);
            }
            "--policy" => {
                i += 1;
                policy = args
                    .get(i)
                    .and_then(|s| bot::Policy::parse(s))
                    .unwrap_or(bot::Policy::Clean);
            }
            "--out" => {
                i += 1;
                out = args.get(i).cloned();
            }
            "--title" => {
                i += 1;
                title = args.get(i).cloned().unwrap_or_default();
            }
            other => {
                eprintln!("unknown flag {other}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let Some((actions, g)) = bot::play(scenario, seed, policy) else {
        eprintln!("unknown scenario {scenario}");
        return ExitCode::from(2);
    };
    let vector = build_vector(scenario, seed, &title, &actions, &g);
    let json = serde_json::to_string_pretty(&vector).unwrap();
    match out {
        Some(path) => {
            let p = PathBuf::from(&path);
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(e) = fs::write(&p, json + "\n") {
                eprintln!("write {path}: {e}");
                return ExitCode::from(2);
            }
            // Refresh checksums if the file landed in vectors/.
            if p.parent() == Some(&vectors_dir()) {
                let _ = rewrite_checksums();
            }
            println!(
                "recorded {} → {}  [{} / {} / score {}]",
                bot_label(policy),
                path,
                g.grade().label(),
                g.end_reason().label(),
                g.score()
            );
        }
        None => println!("{json}"),
    }
    ExitCode::SUCCESS
}

fn bot_label(p: bot::Policy) -> &'static str {
    match p {
        bot::Policy::Clean => "clean",
        bot::Policy::Idle => "idle",
        bot::Policy::Lapse => "lapse",
    }
}

fn dump_scenario_cmd(args: &[String]) -> ExitCode {
    let id: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let Some(s) = scenario_by_id(id) else {
        eprintln!("usage: dump-scenario <1..{SCENARIO_COUNT}>");
        return ExitCode::from(2);
    };
    println!("Scenario {} — {}", s.id, s.name);
    println!("  difficulty:  {}", s.difficulty.label());
    println!("  objective:   {}", s.objective);
    println!(
        "  max_ticks:   {} ({} s @ {} Hz)",
        s.max_ticks,
        s.max_ticks / TICK_RATE_HZ,
        TICK_RATE_HZ
    );
    println!(
        "  thresholds:  boundary>={} consent>={} vault>={} raw<={} stim<={} latency<={} min_release={}t",
        s.success_thresholds.boundary_min,
        s.success_thresholds.consent_min,
        s.success_thresholds.vault_min,
        s.success_thresholds.raw_max,
        s.success_thresholds.stim_max,
        s.success_thresholds.latency_max,
        s.success_thresholds.min_release_tick
    );
    println!("  schedule:    {} events", s.event_schedule.len());
    for e in s.event_schedule {
        println!(
            "    t={:>5}  {:<22} sev={:<3} hidden={:<3} visible={:<3} ttl={}",
            e.at_tick,
            e.kind.label(),
            e.severity,
            e.hidden_risk,
            e.visible_risk,
            e.ttl_ticks
        );
    }
    ExitCode::SUCCESS
}

fn hash_state_cmd(args: &[String]) -> ExitCode {
    let mut scenario = 1u32;
    let mut seed = 1u64;
    let mut ticks = 0u32;
    let mut policy = bot::Policy::Idle;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => {
                i += 1;
                scenario = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(1);
            }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| parse_seed(s)).unwrap_or(1);
            }
            "--ticks" => {
                i += 1;
                ticks = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
            "--policy" => {
                i += 1;
                policy = args
                    .get(i)
                    .and_then(|s| bot::Policy::parse(s))
                    .unwrap_or(bot::Policy::Idle);
            }
            _ => {}
        }
        i += 1;
    }
    let Some(mut g) = GameState::new(scenario, seed) else {
        eprintln!("unknown scenario {scenario}");
        return ExitCode::from(2);
    };
    if ticks == 0 {
        // Run to terminal under the chosen policy.
        if let Some((_, finished)) = bot::play(scenario, seed, policy) {
            g = finished;
        }
    } else {
        for _ in 0..ticks {
            if g.status() != RunStatus::Running {
                break;
            }
            g.advance(PlayerAction::None);
        }
    }
    print_run(&g);
    ExitCode::SUCCESS
}

fn explain_grade_cmd(args: &[String]) -> ExitCode {
    let Some(file) = args.first() else {
        eprintln!("usage: explain-grade <file.json>");
        return ExitCode::from(2);
    };
    let txt = match fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::from(2);
        }
    };
    let v: Vector = match serde_json::from_str(&txt) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse: {e}");
            return ExitCode::from(2);
        }
    };
    let g = match replay_vector(&v) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(3);
        }
    };
    let m = g.metrics();
    println!("{} → {}", file, g.grade().label());
    println!("  reason:  {}", g.end_reason().label());
    println!("  score:   {}", g.score());
    println!(
        "  metrics: boundary={} consent={} vault={} flow={} raw={} stim={} latency={} audit={}",
        m.boundary_integrity,
        m.consent_coherence,
        m.vault_integrity,
        m.cognitive_flow,
        m.raw_leak_risk,
        m.stimulation_risk,
        m.latency_pressure,
        m.audit_confidence
    );
    println!(
        "  counters: unresolved_critical={} unsafe_actions={} audits={} revocations={} rejected={}",
        g.counters.unresolved_critical_events,
        g.counters.unsafe_actions,
        g.counters.successful_audits,
        g.counters.correct_revocations,
        g.gate.rejected_actions_total
    );
    match g.grade() {
        SovereigntyGrade::Sealed => {
            println!("  → Excellent sovereignty; boundary preserved (§5.5).")
        }
        SovereigntyGrade::Reviewable => {
            println!("  → Released or timed out within tolerance; review advised.")
        }
        SovereigntyGrade::Breached => {
            println!("  → Boundary / raw / consent / vault failure (§5.2).")
        }
        SovereigntyGrade::Unsafe => {
            println!("  → Unsafe stimulation escape — never SEALED (§5.5 invariant).")
        }
        SovereigntyGrade::Pending => println!("  → Run did not terminate."),
    }
    ExitCode::SUCCESS
}

fn generate_daily_vector_cmd(args: &[String]) -> ExitCode {
    let mut date: Option<String> = None;
    let mut out: Option<String> = None;
    let mut scenario = 1u32;
    let mut policy = bot::Policy::Clean;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--date" => {
                i += 1;
                date = args.get(i).cloned();
            }
            "--out" => {
                i += 1;
                out = args.get(i).cloned();
            }
            "--scenario" => {
                i += 1;
                scenario = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(1);
            }
            "--policy" => {
                i += 1;
                policy = args
                    .get(i)
                    .and_then(|s| bot::Policy::parse(s))
                    .unwrap_or(bot::Policy::Clean);
            }
            _ => {}
        }
        i += 1;
    }
    let Some(date) = date else {
        eprintln!("usage: generate-daily-vector --date YYYY-MM-DD [--scenario N] [--out f]");
        return ExitCode::from(2);
    };
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        eprintln!("bad date {date}");
        return ExitCode::from(2);
    }
    let (y, mo, d) = (
        parts[0].parse::<u16>().unwrap_or(0),
        parts[1].parse::<u8>().unwrap_or(0),
        parts[2].parse::<u8>().unwrap_or(0),
    );
    let seed = daily_seed(y, mo, d);
    let Some((actions, g)) = bot::play(scenario, seed, policy) else {
        eprintln!("unknown scenario {scenario}");
        return ExitCode::from(2);
    };
    let title = format!("daily {date}");
    let vector = build_vector(scenario, seed, &title, &actions, &g);
    let json = serde_json::to_string_pretty(&vector).unwrap();
    match out {
        Some(path) => {
            if let Err(e) = fs::write(&path, json + "\n") {
                eprintln!("write {path}: {e}");
                return ExitCode::from(2);
            }
            if PathBuf::from(&path).parent() == Some(&vectors_dir()) {
                let _ = rewrite_checksums();
            }
            println!(
                "daily {date}: seed {seed:016x} → {} ({})",
                g.grade().label(),
                path
            );
        }
        None => println!("{json}"),
    }
    ExitCode::SUCCESS
}

fn release_evidence_cmd() -> ExitCode {
    println!("Neural Boundary Game {CORE_VERSION} — release evidence");
    println!("  schema:    {REPLAY_SCHEMA}");
    println!("  scenarios: {SCENARIO_COUNT}");
    println!("  tick rate: {TICK_RATE_HZ} Hz");
    println!();
    verify_all_cmd()
}

fn print_run(g: &GameState) {
    println!(
        "scenario {} seed {:016x} → {} / {} | score {} | tick {} | hash {}",
        g.scenario_id,
        g.seed,
        g.grade().label(),
        g.end_reason().label(),
        g.score(),
        g.tick(),
        hash_hex(g.state_hash())
    );
}

fn usage() -> ExitCode {
    eprintln!(
        "neural-boundary-cli {CORE_VERSION}\n\
         commands:\n\
         \x20 verify-all\n\
         \x20 run-vector <file.json>\n\
         \x20 record --scenario N --seed HEX --policy clean|idle|lapse [--out f] [--title t]\n\
         \x20 dump-scenario <1..{SCENARIO_COUNT}>\n\
         \x20 hash-state --scenario N --seed HEX [--ticks T] [--policy P]\n\
         \x20 explain-grade <file.json>\n\
         \x20 generate-daily-vector --date YYYY-MM-DD [--scenario N] [--out f]\n\
         \x20 release-evidence"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("verify-all") => verify_all_cmd(),
        Some("run-vector") => run_vector_cmd(&args[1..]),
        Some("record") => record_cmd(&args[1..]),
        Some("dump-scenario") => dump_scenario_cmd(&args[1..]),
        Some("hash-state") => hash_state_cmd(&args[1..]),
        Some("explain-grade") => explain_grade_cmd(&args[1..]),
        Some("generate-daily-vector") => generate_daily_vector_cmd(&args[1..]),
        Some("release-evidence") => release_evidence_cmd(),
        _ => usage(),
    }
}

#[allow(dead_code)]
fn scenarios_count_sanity() -> usize {
    SCENARIOS.len()
}
