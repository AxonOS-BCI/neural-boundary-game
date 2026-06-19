// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.0.1).
// See LICENSE and IP_NOTICE.md for details.

//! Eight gameplay metrics (§5.1), each `0..=100`, clamped in Rust.

use crate::hash::Fnv64;

/// §5.1 clamp: deltas are computed in `i16`, then clamped into `0..=100`.
#[inline]
pub fn clamp_metric(value: i16) -> u8 {
    value.clamp(0, 100) as u8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metrics {
    pub boundary_integrity: u8,
    pub consent_coherence: u8,
    pub vault_integrity: u8,
    pub cognitive_flow: u8,
    pub raw_leak_risk: u8,
    pub stimulation_risk: u8,
    pub latency_pressure: u8,
    pub audit_confidence: u8,
}

impl Metrics {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        boundary_integrity: u8,
        consent_coherence: u8,
        vault_integrity: u8,
        cognitive_flow: u8,
        raw_leak_risk: u8,
        stimulation_risk: u8,
        latency_pressure: u8,
        audit_confidence: u8,
    ) -> Self {
        Self {
            boundary_integrity,
            consent_coherence,
            vault_integrity,
            cognitive_flow,
            raw_leak_risk,
            stimulation_risk,
            latency_pressure,
            audit_confidence,
        }
    }

    /// Apply a signed delta to a field accessor via i16 then clamp.
    pub fn adjust(field: &mut u8, delta: i16) {
        *field = clamp_metric(*field as i16 + delta);
    }

    /// Invariant guard (§19): every metric must remain in `0..=100`.
    pub fn in_bounds(&self) -> bool {
        let ok = |v: u8| v <= 100;
        ok(self.boundary_integrity)
            && ok(self.consent_coherence)
            && ok(self.vault_integrity)
            && ok(self.cognitive_flow)
            && ok(self.raw_leak_risk)
            && ok(self.stimulation_risk)
            && ok(self.latency_pressure)
            && ok(self.audit_confidence)
    }

    pub fn feed_hash(&self, h: &mut Fnv64) {
        h.feed_u8(self.boundary_integrity);
        h.feed_u8(self.consent_coherence);
        h.feed_u8(self.vault_integrity);
        h.feed_u8(self.cognitive_flow);
        h.feed_u8(self.raw_leak_risk);
        h.feed_u8(self.stimulation_risk);
        h.feed_u8(self.latency_pressure);
        h.feed_u8(self.audit_confidence);
    }
}
