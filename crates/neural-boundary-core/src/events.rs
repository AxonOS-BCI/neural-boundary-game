// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
//
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
// See LICENSE and IP_NOTICE.md for details.

//! Neural event model (§6). Ten event kinds; hidden vs visible risk (§6.3).

use crate::hash::Fnv64;

/// Permission scope bitmask. RAW and ADMIN are sensitive scopes (§5.2 consent collapse).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PermissionScope(pub u16);

impl PermissionScope {
    pub const NONE: Self = Self(0);
    pub const FLOW: Self = Self(0x0001);
    pub const STIM: Self = Self(0x0002);
    pub const RAW: Self = Self(0x0004);
    pub const ADMIN: Self = Self(0x0008);
    pub const SENSITIVE: u16 = Self::RAW.0 | Self::ADMIN.0;

    pub const fn bits(self) -> u16 {
        self.0
    }
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
    pub fn is_sensitive(self) -> bool {
        self.0 & Self::SENSITIVE != 0
    }
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Ten required event types (§6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EventKind {
    BenignFlow = 0,
    ConsentRequest = 1,
    PermissionEscalation = 2,
    RawSignalExposure = 3,
    StimulationRequest = 4,
    UnsafeStimulation = 5,
    LatencySpike = 6,
    AdversarialProbe = 7,
    VaultPressure = 8,
    AuditCheckpoint = 9,
}

impl EventKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::BenignFlow,
            1 => Self::ConsentRequest,
            2 => Self::PermissionEscalation,
            3 => Self::RawSignalExposure,
            4 => Self::StimulationRequest,
            5 => Self::UnsafeStimulation,
            6 => Self::LatencySpike,
            7 => Self::AdversarialProbe,
            8 => Self::VaultPressure,
            9 => Self::AuditCheckpoint,
            _ => return None,
        })
    }
    pub const fn code(self) -> u8 {
        self as u8
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::BenignFlow => "BENIGN_FLOW",
            Self::ConsentRequest => "CONSENT_REQUEST",
            Self::PermissionEscalation => "PERMISSION_ESCALATION",
            Self::RawSignalExposure => "RAW_SIGNAL_EXPOSURE",
            Self::StimulationRequest => "STIMULATION_REQUEST",
            Self::UnsafeStimulation => "UNSAFE_STIMULATION",
            Self::LatencySpike => "LATENCY_SPIKE",
            Self::AdversarialProbe => "ADVERSARIAL_PROBE",
            Self::VaultPressure => "VAULT_PRESSURE",
            Self::AuditCheckpoint => "AUDIT_CHECKPOINT",
        }
    }
    /// Critical events end the run badly if left unresolved past expiry.
    pub const fn is_critical(self) -> bool {
        matches!(
            self,
            Self::RawSignalExposure
                | Self::UnsafeStimulation
                | Self::PermissionEscalation
                | Self::AdversarialProbe
        )
    }
}

/// Active neural event (§6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NeuralEvent {
    pub id: u32,
    pub kind: EventKind,
    pub severity: u8,
    pub hidden_risk: u8,
    pub visible_risk: u8,
    pub permission_scope: PermissionScope,
    pub requires_audit: bool,
    pub audited: bool,
    pub created_at_tick: u32,
    pub expires_at_tick: u32,
}

impl NeuralEvent {
    /// Risk the player currently perceives (§6.3): visible until audited.
    pub fn perceived_risk(&self) -> u8 {
        if self.audited {
            self.hidden_risk
        } else {
            self.visible_risk
        }
    }
    /// Reveal hidden risk and scope (§6.3) once audited.
    pub fn reveal(&mut self) {
        self.audited = true;
        self.visible_risk = self.hidden_risk;
    }
    pub fn feed_hash(&self, h: &mut Fnv64) {
        h.feed_u32(self.id);
        h.feed_u8(self.kind.code());
        h.feed_u8(self.severity);
        h.feed_u8(self.hidden_risk);
        h.feed_u8(self.visible_risk);
        h.feed_u16(self.permission_scope.bits());
        h.feed_bool(self.requires_audit);
        h.feed_bool(self.audited);
        h.feed_u32(self.created_at_tick);
        h.feed_u32(self.expires_at_tick);
    }
}
