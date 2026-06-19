// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
// See LICENSE and IP_NOTICE.md for details.

//! Deterministic primitives (§19): xorshift64star-v1 RNG, fnv1a64-v1 little-endian
//! hash, and the daily seed. No wall-clock, no float, stable across machines.

pub const HASH_ALGORITHM: &str = "fnv1a64-v1";
pub const RNG_ALGORITHM: &str = "xorshift64star-v1";

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// xorshift64star-v1. Seed 0 remaps to the golden-ratio constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform-ish in `0..n` (n>0). Returns 0 when n==0.
    pub fn range(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as u32
        }
    }

    pub const fn state(&self) -> u64 {
        self.state
    }
}

/// FNV-1a 64-bit, little-endian field encoding.
#[derive(Clone, Copy, Debug)]
pub struct Fnv64 {
    h: u64,
}

impl Fnv64 {
    pub const fn new() -> Self {
        Self { h: FNV_OFFSET }
    }
    pub fn feed_u8(&mut self, v: u8) {
        self.h ^= v as u64;
        self.h = self.h.wrapping_mul(FNV_PRIME);
    }
    pub fn feed_u16(&mut self, v: u16) {
        for b in v.to_le_bytes() {
            self.feed_u8(b);
        }
    }
    pub fn feed_u32(&mut self, v: u32) {
        for b in v.to_le_bytes() {
            self.feed_u8(b);
        }
    }
    pub fn feed_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.feed_u8(b);
        }
    }
    pub fn feed_i32(&mut self, v: i32) {
        self.feed_u32(v as u32);
    }
    pub fn feed_bool(&mut self, v: bool) {
        self.feed_u8(v as u8);
    }
    pub fn feed_str(&mut self, s: &str) {
        for b in s.as_bytes() {
            self.feed_u8(*b);
        }
    }
    pub fn finish(&self) -> u64 {
        self.h
    }
}

impl Default for Fnv64 {
    fn default() -> Self {
        Self::new()
    }
}

/// Daily seed from `"NBG|8.2.1|YYYY-MM-DD|DAILY"` via FNV-1a then one xorshift round.
pub fn daily_seed(year: u16, month: u8, day: u8) -> u64 {
    let mut h: u64 = FNV_OFFSET;
    let mut feed = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    };
    for b in b"NBG|8.2.1|" {
        feed(*b);
    }
    feed(b'0' + (year / 1000) as u8);
    feed(b'0' + (year / 100 % 10) as u8);
    feed(b'0' + (year / 10 % 10) as u8);
    feed(b'0' + (year % 10) as u8);
    feed(b'-');
    feed(b'0' + month / 10);
    feed(b'0' + month % 10);
    feed(b'-');
    feed(b'0' + day / 10);
    feed(b'0' + day % 10);
    for b in b"|DAILY" {
        feed(*b);
    }
    let seed = if h == 0 { 0x7300 } else { h };
    Rng::new(seed).next_u64()
}
