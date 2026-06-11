//! Deterministic replay verifier and vector recorder.

use neural_boundary_core::{
    BoundaryAction, BoundaryStatus, Difficulty, EntityKind, EvidenceLevel, FeedbackCode,
    InputEvent, RunMode, Simulation, SimulationConfig, TerminalReason, CORE_VERSION,
    HASH_ALGORITHM, REPLAY_SCHEMA, TICK_RATE,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const EXIT_USAGE: u8 = 2;
const EXIT_SCHEMA: u8 = 3;
const EXIT_COMPATIBILITY: u8 = 4;
const EXIT_MISMATCH: u8 = 5;
const EXIT_INVARIANT: u8 = 7;
const MAX_REPLAY_BYTES: u64 = 1_048_576;
const MAX_REPLAY_EVENTS: usize = 10_000;
const MAX_REPLAY_TICKS: u32 = 100_000;
const MAX_SUMMARY_BYTES: usize = 4_096;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReplayFile {
    schema: String,
    product_version: String,
    core_version: String,
    hash_algorithm: String,
    seed: u64,
    mode: String,
    difficulty: String,
    tick_rate: u16,
    initial_config: ReplayConfig,
    events: Vec<ReplayEvent>,
    expected: ReplayExpected,
    summary: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReplayConfig {
    max_ticks: u32,
    raw_leak_limit: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReplayEvent {
    tick: u32,
    lane: u8,
    action: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReplayExpected {
    terminal_tick: u32,
    status: String,
    reason: String,
    state_hash: String,
    trust: u8,
    risk: u8,
    integrity: u8,
    evidence: u8,
    review_gates: u8,
    raw_leaks: u8,
    score: u32,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.code)
        }
    }
}

#[derive(Debug)]
struct CliError {
    code: u8,
    message: String,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

fn error(code: u8, message: impl Into<String>) -> CliError {
    CliError {
        code,
        message: message.into(),
    }
}

fn run() -> Result<(), CliError> {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "version" => {
            println!("Neural Boundary Game v{CORE_VERSION}");
            Ok(())
        }
        "schema" => {
            println!("{REPLAY_SCHEMA}");
            println!("hash: {HASH_ALGORITHM}");
            println!("tick-rate: {TICK_RATE}");
            Ok(())
        }
        "verify" => {
            let path = args.get(1).map_or("vectors/01-clean-sealed.json", String::as_str);
            verify_path(Path::new(path), false)
        }
        "trace" => {
            let path = args
                .get(1)
                .ok_or_else(|| error(EXIT_USAGE, "usage: neural-boundary trace <path>"))?;
            verify_path(Path::new(path), true)
        }
        "verify-all" => verify_all(Path::new("vectors")),
        "record" => record_command(&args[1..]),
        "search" => search_command(&args[1..]),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => Err(error(
            EXIT_USAGE,
            format!("unknown command: {command}\nRun `neural-boundary help`."),
        )),
    }
}

fn print_help() {
    println!(
        "Neural Boundary Game deterministic tooling\n\n\
         Commands:\n\
           version                  print product/core version\n\
           schema                   print replay compatibility identifiers\n\
           verify [path]            verify one replay vector\n\
           verify-all               verify every vectors/*.json file\n\
           trace <path>             verify with event-by-event diagnostics\n\
           record [mode] [seed]     generate an autopilot replay on stdout\n\
           search [mode] [start]    find the first autopilot-sealed seed\n\n\
         Exit codes: 0 success, 2 usage, 3 malformed schema, 4 incompatible,\n\
         5 deterministic mismatch, 7 invariant failure."
    );
}

fn load_replay(path: &Path) -> Result<ReplayFile, CliError> {
    let metadata = fs::metadata(path).map_err(|source| {
        error(EXIT_SCHEMA, format!("FAIL: cannot inspect {}: {source}", path.display()))
    })?;
    if metadata.len() > MAX_REPLAY_BYTES {
        return Err(error(
            EXIT_SCHEMA,
            format!("FAIL: replay exceeds {MAX_REPLAY_BYTES} bytes: {}", path.display()),
        ));
    }
    let bytes = fs::read(path).map_err(|source| {
        error(
            EXIT_SCHEMA,
            format!("FAIL: cannot read {}: {source}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|source| {
        error(
            EXIT_SCHEMA,
            format!("FAIL: malformed replay {}: {source}", path.display()),
        )
    })
}

fn verify_path(path: &Path, trace: bool) -> Result<(), CliError> {
    let replay = load_replay(path)?;
    validate_header(&replay)?;
    validate_expected(&replay.expected, &replay.initial_config)?;
    validate_event_order(
        &replay.events,
        replay.initial_config.max_ticks,
        replay.expected.terminal_tick,
    )?;

    let mode = parse_mode(&replay.mode)?;
    let difficulty = parse_difficulty(&replay.difficulty)?;
    let mut config = SimulationConfig::canonical(replay.seed, mode, difficulty);
    config.max_ticks = replay.initial_config.max_ticks;
    config.raw_leak_limit = replay.initial_config.raw_leak_limit;
    let mut simulation = Simulation::new(config);

    for event in &replay.events {
        let input = InputEvent {
            tick: event.tick,
            lane: event.lane,
            action: parse_action(&event.action)?,
        };
        simulation.apply_event(input);
        if trace {
            let snapshot = simulation.snapshot();
            println!(
                concat!(
                    "tick={:04} lane={} action={:<10} feedback={:?} ",
                    "trust={} risk={} integrity={} evidence=L{} gates={:05b} hash={:016x}"
                ),
                snapshot.tick,
                snapshot.selected_lane,
                event.action,
                snapshot.feedback,
                snapshot.trust,
                snapshot.risk,
                snapshot.integrity,
                snapshot.evidence as u8,
                snapshot.review_gates,
                snapshot.state_hash,
            );
        }
    }

    if simulation.snapshot().tick < replay.expected.terminal_tick {
        simulation.tick_many(replay.expected.terminal_tick - simulation.snapshot().tick);
    }
    let snapshot = simulation.snapshot();
    let expected_hash = parse_hash(&replay.expected.state_hash)?;
    let expected_status = parse_status(&replay.expected.status)?;
    let expected_reason = parse_reason(&replay.expected.reason)?;

    let mut mismatches = Vec::new();
    compare(&mut mismatches, "terminal_tick", replay.expected.terminal_tick, snapshot.tick);
    compare(&mut mismatches, "status", expected_status as u8, snapshot.status as u8);
    compare(
        &mut mismatches,
        "reason",
        expected_reason as u8,
        snapshot.terminal_reason as u8,
    );
    compare(&mut mismatches, "state_hash", expected_hash, snapshot.state_hash);
    compare(&mut mismatches, "trust", replay.expected.trust, snapshot.trust);
    compare(&mut mismatches, "risk", replay.expected.risk, snapshot.risk);
    compare(
        &mut mismatches,
        "integrity",
        replay.expected.integrity,
        snapshot.integrity,
    );
    compare(
        &mut mismatches,
        "evidence",
        replay.expected.evidence,
        snapshot.evidence as u8,
    );
    compare(
        &mut mismatches,
        "review_gates",
        replay.expected.review_gates,
        snapshot.review_gates,
    );
    compare(
        &mut mismatches,
        "raw_leaks",
        replay.expected.raw_leaks,
        snapshot.raw_leaks,
    );
    compare(&mut mismatches, "score", replay.expected.score, snapshot.score);

    if !mismatches.is_empty() {
        return Err(error(
            EXIT_MISMATCH,
            format!(
                "FAIL: deterministic replay mismatch in {}\n{}",
                path.display(),
                mismatches.join("\n")
            ),
        ));
    }

    println!("PASS: {}", path.display());
    println!(
        "  terminal: tick {} · {:?} · {:?}",
        snapshot.tick, snapshot.status, snapshot.terminal_reason
    );
    println!(
        "  metrics: trust {} · risk {} · integrity {} · evidence L{}",
        snapshot.trust, snapshot.risk, snapshot.integrity, snapshot.evidence as u8
    );
    println!(
        "  gates: {:05b} · raw leaks {} · score {}",
        snapshot.review_gates, snapshot.raw_leaks, snapshot.score
    );
    println!("  state hash: {:016x}", snapshot.state_hash);
    Ok(())
}

fn compare<T>(mismatches: &mut Vec<String>, field: &str, expected: T, actual: T)
where
    T: std::fmt::Debug + PartialEq,
{
    if expected != actual {
        mismatches.push(format!("  {field}: expected {expected:?}, got {actual:?}"));
    }
}

fn verify_all(directory: &Path) -> Result<(), CliError> {
    let mut paths: Vec<PathBuf> = fs::read_dir(directory)
        .map_err(|source| {
            error(
                EXIT_SCHEMA,
                format!("cannot read {}: {source}", directory.display()),
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .map(|extension| extension.to_string_lossy() == "json")
                .unwrap_or(false)
        })
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(error(EXIT_SCHEMA, "no replay vectors found"));
    }
    for path in &paths {
        verify_path(path, false)?;
    }
    println!("PASS: verified {} canonical replay vectors", paths.len());
    Ok(())
}

fn record_command(args: &[String]) -> Result<(), CliError> {
    let mode = args
        .first()
        .map_or(Ok(RunMode::Guided), |value| parse_mode(value))?;
    let seed = args
        .get(1)
        .map_or(Ok(58_u64), |value| {
            value
                .parse::<u64>()
                .map_err(|_| error(EXIT_USAGE, "seed must be an unsigned integer"))
        })?;
    let (simulation, events) = run_autopilot(seed, mode, Difficulty::Standard);
    if simulation.snapshot().terminal_reason != TerminalReason::Released {
        return Err(error(
            EXIT_INVARIANT,
            format!("autopilot did not seal seed {seed}: {:?}", simulation.snapshot()),
        ));
    }
    let replay = replay_from_run(seed, mode, Difficulty::Standard, events, &simulation);
    println!(
        "{}",
        serde_json::to_string_pretty(&replay)
            .map_err(|source| error(EXIT_INVARIANT, format!("serialization failed: {source}")))?
    );
    Ok(())
}

fn search_command(args: &[String]) -> Result<(), CliError> {
    let mode = args
        .first()
        .map_or(Ok(RunMode::Standard), |value| parse_mode(value))?;
    let start = args
        .get(1)
        .map_or(Ok(1_u64), |value| {
            value
                .parse::<u64>()
                .map_err(|_| error(EXIT_USAGE, "start seed must be an unsigned integer"))
        })?;
    for seed in start..start.saturating_add(10_000) {
        let (simulation, events) = run_autopilot(seed, mode, Difficulty::Standard);
        if simulation.snapshot().terminal_reason == TerminalReason::Released {
            println!(
                "PASS: seed {seed} sealed in {} events at tick {} with hash {:016x}",
                events.len(),
                simulation.snapshot().tick,
                simulation.snapshot().state_hash
            );
            return Ok(());
        }
    }
    Err(error(EXIT_INVARIANT, "no sealed seed found in search window"))
}

fn run_autopilot(
    seed: u64,
    mode: RunMode,
    difficulty: Difficulty,
) -> (Simulation, Vec<ReplayEvent>) {
    let mut simulation = Simulation::new(SimulationConfig::canonical(seed, mode, difficulty));
    let mut events = Vec::new();
    let mut handled: BTreeSet<(u32, u8)> = BTreeSet::new();

    while !simulation.is_terminal() {
        simulation.tick();
        let mut visible = Vec::new();
        for index in 0..neural_boundary_core::ENTITY_CAPACITY {
            if let Some(entity) = simulation.entity(index) {
                visible.push(entity);
            }
        }
        visible.sort_by(|left, right| right.position.cmp(&left.position));

        for entity in visible {
            let action = match entity.kind {
                EntityKind::TypedIntent => continue,
                EntityKind::ValidatedIntent => {
                    let snapshot = simulation.snapshot();
                    if snapshot.consent.active && snapshot.evidence >= EvidenceLevel::L1 {
                        BoundaryAction::Convert
                    } else {
                        continue;
                    }
                }
                kind => kind.required_action(),
            };
            let key = (entity.id, action as u8);
            if handled.contains(&key) {
                continue;
            }
            simulation.select_lane(entity.lane);
            simulation.apply_action(action);
            events.push(ReplayEvent {
                tick: simulation.snapshot().tick,
                lane: entity.lane,
                action: action_name(action).to_owned(),
            });
            handled.insert(key);

            if action == BoundaryAction::Validate {
                if let Some(updated) = (0..neural_boundary_core::ENTITY_CAPACITY)
                    .filter_map(|index| simulation.entity(index))
                    .find(|candidate| candidate.id == entity.id)
                {
                    if updated.kind == EntityKind::ValidatedIntent {
                        let snapshot = simulation.snapshot();
                        if snapshot.consent.active && snapshot.evidence >= EvidenceLevel::L1 {
                            simulation.select_lane(updated.lane);
                            simulation.apply_action(BoundaryAction::Convert);
                            events.push(ReplayEvent {
                                tick: simulation.snapshot().tick,
                                lane: updated.lane,
                                action: "convert".to_owned(),
                            });
                            handled.insert((updated.id, BoundaryAction::Convert as u8));
                        }
                    }
                }
            }
        }

        if simulation.snapshot().tick > 900 && simulation.release_ready() {
            simulation.apply_action(BoundaryAction::Release);
            events.push(ReplayEvent {
                tick: simulation.snapshot().tick,
                lane: simulation.snapshot().selected_lane,
                action: "release".to_owned(),
            });
        }
    }
    (simulation, events)
}

fn replay_from_run(
    seed: u64,
    mode: RunMode,
    difficulty: Difficulty,
    events: Vec<ReplayEvent>,
    simulation: &Simulation,
) -> ReplayFile {
    let snapshot = simulation.snapshot();
    ReplayFile {
        schema: REPLAY_SCHEMA.to_owned(),
        product_version: CORE_VERSION.to_owned(),
        core_version: CORE_VERSION.to_owned(),
        hash_algorithm: HASH_ALGORITHM.to_owned(),
        seed,
        mode: mode_name(mode).to_owned(),
        difficulty: difficulty_name(difficulty).to_owned(),
        tick_rate: TICK_RATE,
        initial_config: ReplayConfig {
            max_ticks: simulation.config().max_ticks,
            raw_leak_limit: simulation.config().raw_leak_limit,
        },
        events,
        expected: ReplayExpected {
            terminal_tick: snapshot.tick,
            status: status_name(snapshot.status).to_owned(),
            reason: reason_name(snapshot.terminal_reason).to_owned(),
            state_hash: format!("{:016x}", snapshot.state_hash),
            trust: snapshot.trust,
            risk: snapshot.risk,
            integrity: snapshot.integrity,
            evidence: snapshot.evidence as u8,
            review_gates: snapshot.review_gates,
            raw_leaks: snapshot.raw_leaks,
            score: snapshot.score,
        },
        summary: "Generated by the deterministic v3.0.0 autopilot recorder.".to_owned(),
    }
}

fn validate_header(replay: &ReplayFile) -> Result<(), CliError> {
    if replay.schema != REPLAY_SCHEMA {
        return Err(error(
            EXIT_COMPATIBILITY,
            format!("incompatible replay schema: {}", replay.schema),
        ));
    }
    if replay.product_version != CORE_VERSION || replay.core_version != CORE_VERSION {
        return Err(error(
            EXIT_COMPATIBILITY,
            format!(
                "version mismatch: product={} core={} expected={CORE_VERSION}",
                replay.product_version, replay.core_version
            ),
        ));
    }
    if replay.hash_algorithm != HASH_ALGORITHM {
        return Err(error(
            EXIT_COMPATIBILITY,
            format!("unsupported hash algorithm: {}", replay.hash_algorithm),
        ));
    }
    if replay.tick_rate != TICK_RATE {
        return Err(error(
            EXIT_COMPATIBILITY,
            format!("tick-rate mismatch: {}", replay.tick_rate),
        ));
    }
    if replay.initial_config.max_ticks == 0 || replay.initial_config.max_ticks > MAX_REPLAY_TICKS {
        return Err(error(
            EXIT_SCHEMA,
            format!(
                "max_ticks must be within 1..={MAX_REPLAY_TICKS}, got {}",
                replay.initial_config.max_ticks
            ),
        ));
    }
    if replay.initial_config.raw_leak_limit == 0 {
        return Err(error(EXIT_SCHEMA, "raw_leak_limit must be at least 1"));
    }
    if replay.summary.trim().is_empty() || replay.summary.len() > MAX_SUMMARY_BYTES {
        return Err(error(
            EXIT_SCHEMA,
            format!("summary must contain 1..={MAX_SUMMARY_BYTES} UTF-8 bytes"),
        ));
    }
    Ok(())
}

fn validate_event_order(
    events: &[ReplayEvent],
    max_ticks: u32,
    terminal_tick: u32,
) -> Result<(), CliError> {
    if events.len() > MAX_REPLAY_EVENTS {
        return Err(error(
            EXIT_SCHEMA,
            format!("replay has {} events; maximum is {MAX_REPLAY_EVENTS}", events.len()),
        ));
    }
    let mut previous = 0_u32;
    for (index, event) in events.iter().enumerate() {
        if event.tick > max_ticks {
            return Err(error(
                EXIT_SCHEMA,
                format!("event {index} tick {} exceeds max_ticks {max_ticks}", event.tick),
            ));
        }
        if event.tick > terminal_tick {
            return Err(error(
                EXIT_SCHEMA,
                format!(
                    "event {index} tick {} exceeds expected terminal_tick {terminal_tick}",
                    event.tick
                ),
            ));
        }
        if event.lane >= neural_boundary_core::LANE_COUNT as u8 {
            return Err(error(
                EXIT_SCHEMA,
                format!("event {index} has invalid lane {}", event.lane),
            ));
        }
        if index > 0 && event.tick < previous {
            return Err(error(
                EXIT_SCHEMA,
                format!("event {index} is out of order"),
            ));
        }
        previous = event.tick;
        parse_action(&event.action)?;
    }
    Ok(())
}

fn parse_mode(value: &str) -> Result<RunMode, CliError> {
    match value.to_ascii_lowercase().as_str() {
        "guided" => Ok(RunMode::Guided),
        "standard" => Ok(RunMode::Standard),
        "audit" => Ok(RunMode::Audit),
        "grand" => Ok(RunMode::Grand),
        "daily" => Ok(RunMode::Daily),
        _ => Err(error(EXIT_SCHEMA, format!("unknown run mode: {value}"))),
    }
}

fn parse_difficulty(value: &str) -> Result<Difficulty, CliError> {
    match value.to_ascii_lowercase().as_str() {
        "assisted" => Ok(Difficulty::Assisted),
        "standard" => Ok(Difficulty::Standard),
        "expert" => Ok(Difficulty::Expert),
        _ => Err(error(EXIT_SCHEMA, format!("unknown difficulty: {value}"))),
    }
}

fn parse_action(value: &str) -> Result<BoundaryAction, CliError> {
    match value.to_ascii_lowercase().as_str() {
        "validate" => Ok(BoundaryAction::Validate),
        "convert" => Ok(BoundaryAction::Convert),
        "quarantine" => Ok(BoundaryAction::Quarantine),
        "consent" => Ok(BoundaryAction::Consent),
        "evidence" => Ok(BoundaryAction::Evidence),
        "release" => Ok(BoundaryAction::Release),
        _ => Err(error(EXIT_SCHEMA, format!("unknown action: {value}"))),
    }
}

fn parse_status(value: &str) -> Result<BoundaryStatus, CliError> {
    match value.to_ascii_lowercase().as_str() {
        "open" => Ok(BoundaryStatus::Open),
        "sealed" => Ok(BoundaryStatus::Sealed),
        "degraded" => Ok(BoundaryStatus::Degraded),
        "breached" => Ok(BoundaryStatus::Breached),
        "unsafe" => Ok(BoundaryStatus::Unsafe),
        _ => Err(error(EXIT_SCHEMA, format!("unknown status: {value}"))),
    }
}

fn parse_reason(value: &str) -> Result<TerminalReason, CliError> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(TerminalReason::None),
        "released" => Ok(TerminalReason::Released),
        "raw-leak-limit" => Ok(TerminalReason::RawLeakLimit),
        "stimulation-crossed" => Ok(TerminalReason::StimulationCrossed),
        "integrity-collapse" => Ok(TerminalReason::IntegrityCollapse),
        "risk-overflow" => Ok(TerminalReason::RiskOverflow),
        "time-expired" => Ok(TerminalReason::TimeExpired),
        "invariant-violation" => Ok(TerminalReason::InvariantViolation),
        _ => Err(error(EXIT_SCHEMA, format!("unknown terminal reason: {value}"))),
    }
}

fn validate_expected(expected: &ReplayExpected, config: &ReplayConfig) -> Result<(), CliError> {
    if expected.terminal_tick > config.max_ticks {
        return Err(error(
            EXIT_SCHEMA,
            format!(
                "expected terminal_tick {} exceeds max_ticks {}",
                expected.terminal_tick, config.max_ticks
            ),
        ));
    }
    if expected.trust > 100 || expected.risk > 100 || expected.integrity > 100 {
        return Err(error(EXIT_SCHEMA, "expected trust/risk/integrity must be within 0..=100"));
    }
    if expected.evidence > 3 {
        return Err(error(EXIT_SCHEMA, "expected evidence must be within 0..=3"));
    }
    if expected.review_gates & !neural_boundary_core::ALL_REVIEW_GATES != 0 {
        return Err(error(EXIT_SCHEMA, "expected review_gates contains unknown bits"));
    }
    parse_hash(&expected.state_hash)?;
    let status = parse_status(&expected.status)?;
    let reason = parse_reason(&expected.reason)?;
    let expected_status = match reason {
        TerminalReason::None => BoundaryStatus::Open,
        TerminalReason::Released => BoundaryStatus::Sealed,
        TerminalReason::RawLeakLimit => BoundaryStatus::Breached,
        TerminalReason::TimeExpired => BoundaryStatus::Degraded,
        TerminalReason::StimulationCrossed
        | TerminalReason::IntegrityCollapse
        | TerminalReason::RiskOverflow
        | TerminalReason::InvariantViolation => BoundaryStatus::Unsafe,
    };
    if status != expected_status {
        return Err(error(
            EXIT_SCHEMA,
            format!(
                "status/reason mismatch: {:?} requires {:?}, got {:?}",
                reason, expected_status, status
            ),
        ));
    }
    Ok(())
}

fn parse_hash(value: &str) -> Result<u64, CliError> {
    if value.len() != 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(error(
            EXIT_SCHEMA,
            format!("state hash must be exactly 16 lowercase hexadecimal characters: {value}"),
        ));
    }
    u64::from_str_radix(value, 16)
        .map_err(|_| error(EXIT_SCHEMA, format!("invalid state hash: {value}")))
}

fn action_name(action: BoundaryAction) -> &'static str {
    match action {
        BoundaryAction::Validate => "validate",
        BoundaryAction::Convert => "convert",
        BoundaryAction::Quarantine => "quarantine",
        BoundaryAction::Consent => "consent",
        BoundaryAction::Evidence => "evidence",
        BoundaryAction::Release => "release",
        BoundaryAction::None => "none",
    }
}

fn mode_name(mode: RunMode) -> &'static str {
    match mode {
        RunMode::Guided => "guided",
        RunMode::Standard => "standard",
        RunMode::Audit => "audit",
        RunMode::Grand => "grand",
        RunMode::Daily => "daily",
    }
}

fn difficulty_name(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Assisted => "assisted",
        Difficulty::Standard => "standard",
        Difficulty::Expert => "expert",
    }
}

fn status_name(status: BoundaryStatus) -> &'static str {
    match status {
        BoundaryStatus::Open => "open",
        BoundaryStatus::Sealed => "sealed",
        BoundaryStatus::Degraded => "degraded",
        BoundaryStatus::Breached => "breached",
        BoundaryStatus::Unsafe => "unsafe",
    }
}

fn reason_name(reason: TerminalReason) -> &'static str {
    match reason {
        TerminalReason::None => "none",
        TerminalReason::Released => "released",
        TerminalReason::RawLeakLimit => "raw-leak-limit",
        TerminalReason::StimulationCrossed => "stimulation-crossed",
        TerminalReason::IntegrityCollapse => "integrity-collapse",
        TerminalReason::RiskOverflow => "risk-overflow",
        TerminalReason::TimeExpired => "time-expired",
        TerminalReason::InvariantViolation => "invariant-violation",
    }
}

#[allow(dead_code)]
fn feedback_name(feedback: FeedbackCode) -> &'static str {
    match feedback {
        FeedbackCode::None => "none",
        FeedbackCode::IntentValidated => "intent-validated",
        FeedbackCode::FalseIntentDetected => "false-intent-detected",
        FeedbackCode::IntentConverted => "intent-converted",
        FeedbackCode::ConversionBlocked => "conversion-blocked",
        FeedbackCode::Quarantined => "quarantined",
        FeedbackCode::ConsentGranted => "consent-granted",
        FeedbackCode::ConsentRevoked => "consent-revoked",
        FeedbackCode::EvidenceRegistered => "evidence-registered",
        FeedbackCode::ReleaseSealed => "release-sealed",
        FeedbackCode::ReleaseBlocked => "release-blocked",
        FeedbackCode::IncorrectAction => "incorrect-action",
        FeedbackCode::NoTarget => "no-target",
        FeedbackCode::RawLeak => "raw-leak",
        FeedbackCode::StimulationBreach => "stimulation-breach",
        FeedbackCode::TypedIntentReleased => "typed-intent-released",
        FeedbackCode::ConsentExpired => "consent-expired",
        FeedbackCode::TimeExpired => "time-expired",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guided_autopilot_seals() {
        let (simulation, events) = run_autopilot(58, RunMode::Guided, Difficulty::Standard);
        assert_eq!(simulation.snapshot().terminal_reason, TerminalReason::Released);
        assert!(!events.is_empty());
    }

    #[test]
    fn action_parser_is_strict() {
        assert!(parse_action("validate").is_ok());
        assert!(parse_action("explode").is_err());
    }
}
