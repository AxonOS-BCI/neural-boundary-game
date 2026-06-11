//! Integration tests for the conformance toolkit: the shipped vectors must
//! verify through the release binary, and the documented exit-code contract
//! must hold for malformed, incompatible, tampered and checksum-broken
//! inputs.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn run(args: &[&str], cwd: &PathBuf) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_neural-boundary-cli"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run neural-boundary-cli");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nbg-cli-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn verify_all_passes_on_shipped_vectors() {
    let root = repo_root();
    let (code, stdout, stderr) = run(&["verify-all"], &root);
    assert_eq!(code, 0, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.contains("8 vector(s) verified"), "{stdout}");
}

#[test]
fn clean_vector_prints_canonical_block() {
    let root = repo_root();
    let (code, stdout, _) = run(&["verify", "vectors/01-clean-sealed.json"], &root);
    assert_eq!(code, 0);
    assert!(stdout.contains("Replay OK"), "{stdout}");
    assert!(stdout.contains("Boundary status: SEALED"), "{stdout}");
}

#[test]
fn breach_vectors_report_breached() {
    let root = repo_root();
    for vector in [
        "vectors/02-idle-breach.json",
        "vectors/04-raw-leak.json",
        "vectors/05-stimulation-fail-closed.json",
    ] {
        let (code, stdout, _) = run(&["verify", vector], &root);
        assert_eq!(code, 0, "{vector}");
        assert!(
            stdout.contains("Boundary status: BREACHED"),
            "{vector}: {stdout}"
        );
    }
}

#[test]
fn revoked_consent_vector_records_revocation() {
    let root = repo_root();
    let raw = fs::read_to_string(root.join("vectors/03-revoked-consent.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(value["expected"]["revocations"].as_u64().unwrap() >= 1);
}

#[test]
fn tampered_state_hash_exits_5() {
    let root = repo_root();
    let dir = temp_dir("hash");
    let raw = fs::read_to_string(root.join("vectors/01-clean-sealed.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["expected"]["state_hash"] = "0x0000000000000000".into();
    let path = dir.join("tampered.json");
    fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let (code, _, stderr) = run(&["verify", path.to_str().unwrap()], &root);
    assert_eq!(code, 5, "{stderr}");
    assert!(stderr.contains("state_hash"), "{stderr}");
}

#[test]
fn wrong_schema_exits_4() {
    let root = repo_root();
    let dir = temp_dir("schema");
    let raw = fs::read_to_string(root.join("vectors/01-clean-sealed.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["schema"] = "neural-boundary-replay-v2.1.2".into();
    let path = dir.join("old-schema.json");
    fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let (code, _, stderr) = run(&["verify", path.to_str().unwrap()], &root);
    assert_eq!(code, 4, "{stderr}");
}

#[test]
fn daily_seed_mismatch_exits_4() {
    let root = repo_root();
    let dir = temp_dir("daily");
    let raw = fs::read_to_string(root.join("vectors/08-daily-seed-sealed.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["seed"] = 12345u64.into();
    let path = dir.join("daily-bad-seed.json");
    fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let (code, _, stderr) = run(&["verify", path.to_str().unwrap()], &root);
    assert_eq!(code, 4, "{stderr}");
    assert!(stderr.contains("daily_seed"), "{stderr}");
}

#[test]
fn malformed_json_exits_3() {
    let root = repo_root();
    let dir = temp_dir("malformed");
    let path = dir.join("broken.json");
    fs::write(&path, "{ this is not json").unwrap();
    let (code, _, _) = run(&["verify", path.to_str().unwrap()], &root);
    assert_eq!(code, 3);
}

#[test]
fn nonincreasing_ticks_exit_3() {
    let root = repo_root();
    let dir = temp_dir("ticks");
    let raw = fs::read_to_string(root.join("vectors/01-clean-sealed.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let first = value["inputs"][0].clone();
    value["inputs"][1] = first;
    let path = dir.join("bad-ticks.json");
    fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let (code, _, stderr) = run(&["verify", path.to_str().unwrap()], &root);
    assert_eq!(code, 3, "{stderr}");
}

#[test]
fn checksum_mismatch_exits_6() {
    let root = repo_root();
    let dir = temp_dir("checksum");
    let vectors = dir.join("vectors");
    fs::create_dir_all(&vectors).unwrap();
    fs::copy(
        root.join("vectors/01-clean-sealed.json"),
        vectors.join("01-clean-sealed.json"),
    )
    .unwrap();
    fs::write(
        vectors.join("checksums.sha256"),
        "0000000000000000000000000000000000000000000000000000000000000000  01-clean-sealed.json\n",
    )
    .unwrap();
    let (code, _, stderr) = run(&["verify-all"], &dir);
    assert_eq!(code, 6, "{stderr}");
}

#[test]
fn version_and_schema_commands() {
    let root = repo_root();
    let (code, stdout, _) = run(&["version"], &root);
    assert_eq!(code, 0);
    assert!(stdout.contains("3.0.1"), "{stdout}");
    let (code, stdout, _) = run(&["schema"], &root);
    assert_eq!(code, 0);
    assert!(stdout.contains("neural-boundary-replay-v3.0.1"), "{stdout}");
    assert!(stdout.contains("fnv1a64-v1"), "{stdout}");
}

#[test]
fn unknown_subcommand_exits_2() {
    let root = repo_root();
    let (code, _, _) = run(&["frobnicate"], &root);
    assert_eq!(code, 2);
}
