//! Local-only persistence under the `axonos_nbg_v301_` namespace.
//!
//! Documented keys (docs/GAME_SPEC.md §storage):
//!   axonos_nbg_v301_prefs        {"mode":"standard","difficulty":"standard"}
//!   axonos_nbg_v301_best_<mode>  {"score":1234,"grade":"Sealed"}
//!   axonos_nbg_v301_guided_done  "1"
//!
//! Every read is validated; malformed values are ignored and overwritten on
//! the next write. Reset removes every key in the namespace.

use neural_boundary_core::{Difficulty, RunMode};
use web_sys::Storage;

pub const NAMESPACE: &str = "axonos_nbg_v301_";

fn local() -> Option<Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn get(key: &str) -> Option<String> {
    local()?
        .get_item(&format!("{NAMESPACE}{key}"))
        .ok()
        .flatten()
}

fn set(key: &str, value: &str) {
    if let Some(storage) = local() {
        let _ = storage.set_item(&format!("{NAMESPACE}{key}"), value);
    }
}

pub fn has_prefs() -> bool {
    get("prefs").is_some()
}

pub fn load_prefs() -> (RunMode, Difficulty) {
    let raw = get("prefs").unwrap_or_default();
    let mut mode = RunMode::Standard;
    let mut difficulty = Difficulty::Standard;
    for part in raw.split(';') {
        if let Some(value) = part.strip_prefix("mode=") {
            if let Some(parsed) = RunMode::from_name(value) {
                mode = parsed;
            }
        }
        if let Some(value) = part.strip_prefix("difficulty=") {
            if let Some(parsed) = Difficulty::from_name(value) {
                difficulty = parsed;
            }
        }
    }
    (mode, difficulty)
}

pub fn save_prefs(mode: RunMode, difficulty: Difficulty) {
    set(
        "prefs",
        &format!("mode={};difficulty={}", mode.name(), difficulty.name()),
    );
}

/// Best score per mode. Returns the stored value when valid.
pub fn load_best(mode: RunMode) -> Option<u32> {
    get(&format!("best_{}", mode.name()))?.parse().ok()
}

pub fn save_best(mode: RunMode, score: u32) {
    let improved = load_best(mode).map(|best| score > best).unwrap_or(true);
    if improved {
        set(&format!("best_{}", mode.name()), &score.to_string());
    }
}

pub fn guided_done() -> bool {
    get("guided_done").as_deref() == Some("1")
}

pub fn mark_guided_done() {
    set("guided_done", "1");
}

/// Remove every key in the namespace. Visible reset action in the UI.
pub fn reset_all() {
    let Some(storage) = local() else {
        return;
    };
    let mut doomed = Vec::new();
    let length = storage.length().unwrap_or(0);
    for index in 0..length {
        if let Ok(Some(key)) = storage.key(index) {
            if key.starts_with(NAMESPACE) {
                doomed.push(key);
            }
        }
    }
    for key in doomed {
        let _ = storage.remove_item(&key);
    }
}
