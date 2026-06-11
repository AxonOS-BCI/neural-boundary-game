//! Integration tests: the replay vectors shipped in `vectors/` must verify
//! byte-for-byte against the deterministic core through the CLI binary.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn verify_vector(relative: &str) -> String {
    let root = repo_root();
    let output = Command::new(env!("CARGO_BIN_EXE_neural-boundary-cli"))
        .arg("verify")
        .arg(root.join(relative))
        .current_dir(&root)
        .output()
        .expect("run neural-boundary-cli");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "verify {relative} failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("Replay OK"), "missing Replay OK:\n{stdout}");
    stdout
}

#[test]
fn canonical_clean_vector_verifies_and_seals() {
    let stdout = verify_vector("vectors/replay-v2.1.2.json");
    assert!(stdout.contains("Boundary status: SEALED"), "{stdout}");
}

#[test]
fn breach_demo_vector_verifies_and_breaches() {
    let stdout = verify_vector("vectors/replay-breach-demo-v2.1.2.json");
    assert!(stdout.contains("Boundary status: BREACHED"), "{stdout}");
}
