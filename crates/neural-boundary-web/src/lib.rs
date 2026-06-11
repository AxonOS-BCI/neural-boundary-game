//! neural-boundary-web — Rust/WASM browser adapter for the Neural Boundary
//! Game v3.0.1 (Sovereign Boundary Edition).
//!
//! The browser is never authoritative: this crate maps keyboard, pointer and
//! touch input to deterministic core actions, renders snapshots, mirrors
//! state into accessible DOM, and manages local-only preferences under the
//! `axonos_nbg_v301_` storage namespace. If WASM fails to initialize, the
//! shell shows an explicit error — never a fake game.

#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
pub fn native_placeholder() -> &'static str {
    "neural-boundary-web is intended for wasm32 builds"
}

#[cfg(target_arch = "wasm32")]
mod accessibility;
#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod bridge;
#[cfg(target_arch = "wasm32")]
mod hud;
#[cfg(target_arch = "wasm32")]
mod input;
#[cfg(target_arch = "wasm32")]
mod render;
#[cfg(target_arch = "wasm32")]
mod storage;

#[cfg(target_arch = "wasm32")]
pub use bridge::start;
