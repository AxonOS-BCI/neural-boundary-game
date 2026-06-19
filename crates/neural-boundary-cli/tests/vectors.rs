// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.9.812).
// See LICENSE and IP_NOTICE.md for details.

//! CLI integration tests (§19): the shipped 16-vector suite must pass
//! verify-all, and the conformance commands must behave deterministically.

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn run(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_neural-boundary-cli"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("run cli");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), s)
}

#[test]
fn verify_all_passes_on_shipped_vectors() {
    let (code, out) = run(&["verify-all"]);
    assert_eq!(code, 0, "verify-all must pass:\n{out}");
    assert!(out.contains("neural-boundary-replay-v3"));
    assert!(out.contains("16 vector"));
}

#[test]
fn release_evidence_runs_green() {
    let (code, out) = run(&["release-evidence"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("7.9.812"));
}

#[test]
fn hash_state_is_deterministic() {
    let a = run(&[
        "hash-state",
        "--scenario",
        "9",
        "--seed",
        "abc123",
        "--policy",
        "clean",
    ]);
    let b = run(&[
        "hash-state",
        "--scenario",
        "9",
        "--seed",
        "abc123",
        "--policy",
        "clean",
    ]);
    assert_eq!(a.0, 0);
    assert_eq!(a.1, b.1, "same inputs must give identical output");
}

#[test]
fn run_vector_reports_each_shipped_vector() {
    for name in [
        "01-clean-boundary-sealed",
        "07-unsafe-stimulation",
        "15-grand-trial-sealed",
        "16-daily-seed-crosscheck",
    ] {
        let path = format!("vectors/{name}.json");
        let (code, out) = run(&["run-vector", &path]);
        assert_eq!(code, 0, "{name}:\n{out}");
    }
}

#[test]
fn dump_scenario_lists_all_nine() {
    for id in 1..=9 {
        let (code, out) = run(&["dump-scenario", &id.to_string()]);
        assert_eq!(code, 0, "scenario {id}");
        assert!(out.contains("difficulty"));
        assert!(out.contains("schedule"));
    }
    let (code, _) = run(&["dump-scenario", "0"]);
    assert_eq!(code, 2);
}

#[test]
fn explain_grade_describes_outcome() {
    let (code, out) = run(&["explain-grade", "vectors/07-unsafe-stimulation.json"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("UNSAFE"));
}

#[test]
fn unknown_command_is_usage_error() {
    let (code, _) = run(&["frobnicate"]);
    assert_eq!(code, 2);
}
