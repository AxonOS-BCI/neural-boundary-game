#!/usr/bin/env python3
"""Independent development mirror for generating v3.0.0 replay fixtures.

The Rust core remains authoritative. CI verifies generated vectors with the Rust
CLI. This mirror exists so maintainers can regenerate fixture expectations and
detect accidental schema drift before invoking Cargo.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum
from typing import Iterable

CORE_VERSION = "3.0.0"
REPLAY_SCHEMA = "neural-boundary-replay-v3.0.0"
HASH_ALGORITHM = "fnv1a64-v1"
TICK_RATE = 60
LANE_COUNT = 5
ENTITY_CAPACITY = 32
BOUNDARY_X = 840
SPAWN_X = 16

GATE_TYPE_SAFETY = 1 << 0
GATE_CONSENT = 1 << 1
GATE_EVIDENCE = 1 << 2
GATE_PRIVACY = 1 << 3
GATE_DETERMINISM = 1 << 4
ALL_REVIEW_GATES = 31

CONSENT_SCOPE_CONVERT = 1 << 0
CONSENT_SCOPE_RELEASE = 1 << 1
CONSENT_SCOPE_REQUIRED = CONSENT_SCOPE_CONVERT | CONSENT_SCOPE_RELEASE

BLOCK_TRUST = 1 << 0
BLOCK_RISK = 1 << 1
BLOCK_INTEGRITY = 1 << 2
BLOCK_EVIDENCE = 1 << 3
BLOCK_REVIEW_GATES = 1 << 4
BLOCK_CONSENT = 1 << 5
BLOCK_RAW_LEAK = 1 << 6
BLOCK_TERMINAL = 1 << 7
BLOCK_REVIEW_WINDOW = 1 << 8
BLOCK_ACTIVE_ENTITIES = 1 << 9

MASK64 = (1 << 64) - 1


class RunMode(IntEnum):
    GUIDED = 1
    STANDARD = 2
    AUDIT = 3
    GRAND = 4
    DAILY = 5


class Difficulty(IntEnum):
    ASSISTED = 1
    STANDARD = 2
    EXPERT = 3


class EntityKind(IntEnum):
    RAW_FRAME = 1
    ARTIFACT = 2
    VALIDATED_INTENT = 3
    UNKNOWN_PACKET = 4
    TYPED_INTENT = 5
    CONSENT_TOKEN = 6
    REVOKED_CONSENT = 7
    EVIDENCE = 8
    CHECKSUM = 9
    CI_TEST = 10
    UNSUPPORTED_CLAIM = 11
    UNTRACEABLE_CLAIM = 12
    ROADMAP_AS_FACT = 13
    STIMULATION_COMMAND = 14

    def required_action(self) -> "BoundaryAction":
        if self == EntityKind.UNKNOWN_PACKET:
            return BoundaryAction.VALIDATE
        if self == EntityKind.VALIDATED_INTENT:
            return BoundaryAction.CONVERT
        if self in (EntityKind.CONSENT_TOKEN, EntityKind.REVOKED_CONSENT):
            return BoundaryAction.CONSENT
        if self in (EntityKind.EVIDENCE, EntityKind.CHECKSUM, EntityKind.CI_TEST):
            return BoundaryAction.EVIDENCE
        if self == EntityKind.TYPED_INTENT:
            return BoundaryAction.NONE
        return BoundaryAction.QUARANTINE

    def is_quarantine_target(self) -> bool:
        return self in {
            EntityKind.RAW_FRAME,
            EntityKind.ARTIFACT,
            EntityKind.UNSUPPORTED_CLAIM,
            EntityKind.UNTRACEABLE_CLAIM,
            EntityKind.ROADMAP_AS_FACT,
            EntityKind.STIMULATION_COMMAND,
        }


class BoundaryAction(IntEnum):
    NONE = 0
    VALIDATE = 1
    CONVERT = 2
    QUARANTINE = 3
    CONSENT = 4
    EVIDENCE = 5
    RELEASE = 6


class EvidenceLevel(IntEnum):
    L0 = 0
    L1 = 1
    L2 = 2
    L3 = 3


class BoundaryStatus(IntEnum):
    OPEN = 0
    SEALED = 1
    DEGRADED = 2
    BREACHED = 3
    UNSAFE = 4


class TerminalReason(IntEnum):
    NONE = 0
    RELEASED = 1
    RAW_LEAK_LIMIT = 2
    STIMULATION_CROSSED = 3
    INTEGRITY_COLLAPSE = 4
    RISK_OVERFLOW = 5
    TIME_EXPIRED = 6
    INVARIANT_VIOLATION = 7


class FeedbackCode(IntEnum):
    NONE = 0
    INTENT_VALIDATED = 1
    FALSE_INTENT_DETECTED = 2
    INTENT_CONVERTED = 3
    CONVERSION_BLOCKED = 4
    QUARANTINED = 5
    CONSENT_GRANTED = 6
    CONSENT_REVOKED = 7
    EVIDENCE_REGISTERED = 8
    RELEASE_SEALED = 9
    RELEASE_BLOCKED = 10
    INCORRECT_ACTION = 11
    NO_TARGET = 12
    RAW_LEAK = 13
    STIMULATION_BREACH = 14
    TYPED_INTENT_RELEASED = 15
    CONSENT_EXPIRED = 16
    TIME_EXPIRED = 17


@dataclass
class ConsentState:
    active: bool = False
    scope: int = 0
    activation_tick: int = 0
    expiry_tick: int = 0

    def allows(self, requested_scope: int) -> bool:
        return self.active and self.scope & requested_scope == requested_scope


@dataclass
class Entity:
    active: bool = False
    id: int = 0
    kind: EntityKind = EntityKind.UNKNOWN_PACKET
    lane: int = 0
    position: int = 0
    speed: int = 0
    flags: int = 0
    spawn_tick: int = 0

    def clone(self) -> "Entity":
        return Entity(**self.__dict__)


@dataclass
class SimulationConfig:
    seed: int
    mode: RunMode
    difficulty: Difficulty = Difficulty.STANDARD
    max_ticks: int = 0
    raw_leak_limit: int = 0


GUIDED = [
    EntityKind.RAW_FRAME,
    EntityKind.CONSENT_TOKEN,
    EntityKind.EVIDENCE,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.CHECKSUM,
    EntityKind.CI_TEST,
    EntityKind.UNSUPPORTED_CLAIM,
    EntityKind.REVOKED_CONSENT,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.CONSENT_TOKEN,
    EntityKind.EVIDENCE,
    EntityKind.ROADMAP_AS_FACT,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.STIMULATION_COMMAND,
    EntityKind.CHECKSUM,
    EntityKind.UNTRACEABLE_CLAIM,
]
STANDARD = [
    EntityKind.RAW_FRAME,
    EntityKind.CONSENT_TOKEN,
    EntityKind.EVIDENCE,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.ARTIFACT,
    EntityKind.CHECKSUM,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.UNSUPPORTED_CLAIM,
    EntityKind.CI_TEST,
    EntityKind.CONSENT_TOKEN,
    EntityKind.RAW_FRAME,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.EVIDENCE,
    EntityKind.UNTRACEABLE_CLAIM,
    EntityKind.CHECKSUM,
    EntityKind.UNKNOWN_PACKET,
]
AUDIT = [
    EntityKind.RAW_FRAME,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.CONSENT_TOKEN,
    EntityKind.EVIDENCE,
    EntityKind.REVOKED_CONSENT,
    EntityKind.UNSUPPORTED_CLAIM,
    EntityKind.CHECKSUM,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.STIMULATION_COMMAND,
    EntityKind.CONSENT_TOKEN,
    EntityKind.CI_TEST,
    EntityKind.ROADMAP_AS_FACT,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.RAW_FRAME,
    EntityKind.UNTRACEABLE_CLAIM,
    EntityKind.EVIDENCE,
    EntityKind.ARTIFACT,
    EntityKind.UNKNOWN_PACKET,
]
GRAND = [
    EntityKind.RAW_FRAME,
    EntityKind.ARTIFACT,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.CONSENT_TOKEN,
    EntityKind.EVIDENCE,
    EntityKind.CHECKSUM,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.REVOKED_CONSENT,
    EntityKind.UNSUPPORTED_CLAIM,
    EntityKind.CONSENT_TOKEN,
    EntityKind.CI_TEST,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.STIMULATION_COMMAND,
    EntityKind.ROADMAP_AS_FACT,
    EntityKind.RAW_FRAME,
    EntityKind.EVIDENCE,
    EntityKind.UNKNOWN_PACKET,
    EntityKind.UNTRACEABLE_CLAIM,
    EntityKind.CHECKSUM,
    EntityKind.UNKNOWN_PACKET,
]
GUIDED_LANES = [2, 1, 3, 2, 4, 0, 1, 3, 2, 1, 4, 0, 2, 4, 1, 3]


def default_max_ticks(mode: RunMode) -> int:
    return {
        RunMode.GUIDED: 3600,
        RunMode.STANDARD: 5400,
        RunMode.AUDIT: 6000,
        RunMode.GRAND: 7200,
        RunMode.DAILY: 5400,
    }[mode]


def minimum_release_tick(mode: RunMode) -> int:
    return {
        RunMode.GUIDED: 3250,
        RunMode.STANDARD: 1500,
        RunMode.AUDIT: 1800,
        RunMode.GRAND: 2400,
        RunMode.DAILY: 1500,
    }[mode]


def spawn_interval(mode: RunMode, difficulty: Difficulty) -> int:
    if mode == RunMode.GUIDED:
        return 210
    base = {RunMode.STANDARD: 135, RunMode.AUDIT: 105, RunMode.GRAND: 120, RunMode.DAILY: 135}[mode]
    if difficulty == Difficulty.ASSISTED:
        return base + 35
    if difficulty == Difficulty.EXPERT:
        return base - 25
    return base


def entity_speed(kind: EntityKind, mode: RunMode, difficulty: Difficulty) -> int:
    if kind == EntityKind.STIMULATION_COMMAND:
        speed = 7
    elif kind in (EntityKind.UNSUPPORTED_CLAIM, EntityKind.UNTRACEABLE_CLAIM, EntityKind.ROADMAP_AS_FACT):
        speed = 6
    elif kind in (EntityKind.RAW_FRAME, EntityKind.ARTIFACT):
        speed = 5
    else:
        speed = 4
    if mode in (RunMode.AUDIT, RunMode.GRAND):
        speed += 1
    if difficulty == Difficulty.ASSISTED:
        speed = max(2, speed - 1)
    elif difficulty == Difficulty.EXPERT:
        speed += 1
    return speed


def kind_for_spawn(mode: RunMode, index: int, seed: int) -> EntityKind:
    if mode == RunMode.GUIDED:
        return GUIDED[index % len(GUIDED)]
    if mode == RunMode.STANDARD:
        return STANDARD[index % len(STANDARD)]
    if mode == RunMode.AUDIT:
        return AUDIT[index % len(AUDIT)]
    if mode == RunMode.GRAND:
        return GRAND[index % len(GRAND)]
    offset = seed % len(STANDARD)
    return STANDARD[(index + offset) % len(STANDARD)]


def daily_seed(yyyymmdd: int) -> int:
    value = 0xCBF29CE484222325
    for byte in REPLAY_SCHEMA.encode("ascii"):
        value = fnv_byte(value, byte)
    for byte in yyyymmdd.to_bytes(4, "little"):
        value = fnv_byte(value, byte)
    return value or 1


def fnv_byte(value: int, byte: int) -> int:
    return ((value ^ byte) * 0x00000100000001B3) & MASK64


def hash_int(value: int, size: int) -> bytes:
    return value.to_bytes(size, "little", signed=False)


@dataclass
class Simulation:
    config: SimulationConfig
    seed: int = field(init=False)
    rng_state: int = field(init=False)
    tick_value: int = 0
    next_spawn_tick: int = field(init=False)
    spawn_index: int = 0
    next_entity_id: int = 1
    selected_lane: int = 2
    trust: int = 60
    risk: int = 18
    integrity: int = 100
    evidence: EvidenceLevel = EvidenceLevel.L0
    review_gates: int = 0
    consent: ConsentState = field(default_factory=ConsentState)
    raw_leaks: int = 0
    privacy_containments: int = 0
    score: int = 0
    streak: int = 0
    best_streak: int = 0
    status: BoundaryStatus = BoundaryStatus.OPEN
    terminal_reason: TerminalReason = TerminalReason.NONE
    feedback: FeedbackCode = FeedbackCode.NONE
    entities: list[Entity] = field(default_factory=lambda: [Entity() for _ in range(ENTITY_CAPACITY)])

    def __post_init__(self) -> None:
        if self.config.seed == 0:
            self.config.seed = 0xA30D5EED5AFE0001
        if self.config.max_ticks == 0:
            self.config.max_ticks = default_max_ticks(self.config.mode)
        if self.config.raw_leak_limit == 0:
            self.config.raw_leak_limit = 3
        self.seed = self.config.seed & MASK64
        self.rng_state = self.seed
        self.next_spawn_tick = 60 if self.config.mode == RunMode.GUIDED else 45

    def is_terminal(self) -> bool:
        return self.terminal_reason != TerminalReason.NONE

    def tick(self) -> None:
        if self.is_terminal():
            return
        self.tick_value = min(0xFFFFFFFF, self.tick_value + 1)
        self.feedback = FeedbackCode.NONE
        if self.consent.active and self.tick_value >= self.consent.expiry_tick:
            self.consent = ConsentState()
            self.review_gates &= ~GATE_CONSENT
            self.feedback = FeedbackCode.CONSENT_EXPIRED
        if self.tick_value >= self.next_spawn_tick:
            self.spawn_next()
            self.next_spawn_tick = min(0xFFFFFFFF, self.next_spawn_tick + spawn_interval(self.config.mode, self.config.difficulty))
        self.advance_entities()
        self.refresh_dynamic_gates()
        self.enforce_terminal_invariants()
        if not self.is_terminal() and self.tick_value >= self.config.max_ticks:
            self.status = BoundaryStatus.DEGRADED
            self.terminal_reason = TerminalReason.TIME_EXPIRED
            self.feedback = FeedbackCode.TIME_EXPIRED

    def tick_many(self, count: int) -> None:
        for _ in range(count):
            if self.is_terminal():
                break
            self.tick()

    def select_lane(self, lane: int) -> None:
        self.selected_lane = min(max(lane, 0), LANE_COUNT - 1)

    def move_lane(self, delta: int) -> None:
        self.selected_lane = (self.selected_lane + delta) % LANE_COUNT

    def apply_event(self, event: dict) -> None:
        if event["tick"] < self.tick_value or self.is_terminal():
            return
        self.tick_many(event["tick"] - self.tick_value)
        self.select_lane(event["lane"])
        self.apply_action(ACTION_BY_NAME[event["action"]])

    def apply_action(self, action: BoundaryAction) -> None:
        if self.is_terminal() or action == BoundaryAction.NONE:
            return
        if action == BoundaryAction.RELEASE:
            self.attempt_release()
            return
        index = self.target_index(self.selected_lane)
        if index is None:
            self.feedback = FeedbackCode.NO_TARGET
            self.penalize_incorrect(1, 0)
            return
        kind = self.entities[index].kind
        if action == BoundaryAction.VALIDATE:
            self.action_validate(index, kind)
        elif action == BoundaryAction.CONVERT:
            self.action_convert(index, kind)
        elif action == BoundaryAction.QUARANTINE:
            self.action_quarantine(index, kind)
        elif action == BoundaryAction.CONSENT:
            self.action_consent(index, kind)
        elif action == BoundaryAction.EVIDENCE:
            self.action_evidence(index, kind)
        self.refresh_dynamic_gates()
        self.enforce_terminal_invariants()

    def action_validate(self, index: int, kind: EntityKind) -> None:
        if kind != EntityKind.UNKNOWN_PACKET:
            self.feedback = FeedbackCode.INCORRECT_ACTION
            self.penalize_incorrect(2, 1)
            return
        if self.entities[index].flags & 1:
            self.entities[index].kind = EntityKind.VALIDATED_INTENT
            self.feedback = FeedbackCode.INTENT_VALIDATED
            self.reward(4, 0, 70)
        else:
            self.entities[index].kind = EntityKind.ARTIFACT
            self.feedback = FeedbackCode.FALSE_INTENT_DETECTED
            self.reward(3, 0, 60)

    def action_convert(self, index: int, kind: EntityKind) -> None:
        if kind != EntityKind.VALIDATED_INTENT:
            self.feedback = FeedbackCode.INCORRECT_ACTION
            self.penalize_incorrect(2, 1)
            return
        if not self.consent.allows(CONSENT_SCOPE_CONVERT) or self.evidence < EvidenceLevel.L1:
            self.feedback = FeedbackCode.CONVERSION_BLOCKED
            self.penalize_incorrect(4, 1)
            return
        self.entities[index].kind = EntityKind.TYPED_INTENT
        self.review_gates |= GATE_TYPE_SAFETY
        self.feedback = FeedbackCode.INTENT_CONVERTED
        self.reward(6, 2, 130)

    def action_quarantine(self, index: int, kind: EntityKind) -> None:
        if not kind.is_quarantine_target():
            self.feedback = FeedbackCode.INCORRECT_ACTION
            self.penalize_incorrect(2, 1)
            return
        self.entities[index].active = False
        if kind in (EntityKind.RAW_FRAME, EntityKind.ARTIFACT):
            self.privacy_containments = min(0xFFFF, self.privacy_containments + 1)
        self.feedback = FeedbackCode.QUARANTINED
        self.reward(5, 3, 180 if kind == EntityKind.STIMULATION_COMMAND else 100)

    def action_consent(self, index: int, kind: EntityKind) -> None:
        if kind == EntityKind.CONSENT_TOKEN:
            self.entities[index].active = False
            self.consent = ConsentState(True, CONSENT_SCOPE_REQUIRED, self.tick_value, min(0xFFFFFFFF, self.tick_value + 1500))
            self.feedback = FeedbackCode.CONSENT_GRANTED
            self.reward(4, 2, 90)
        elif kind == EntityKind.REVOKED_CONSENT:
            self.entities[index].active = False
            self.consent = ConsentState()
            self.review_gates &= ~GATE_CONSENT
            self.feedback = FeedbackCode.CONSENT_REVOKED
            self.reward(3, 2, 100)
        else:
            self.feedback = FeedbackCode.INCORRECT_ACTION
            self.penalize_incorrect(2, 1)

    def action_evidence(self, index: int, kind: EntityKind) -> None:
        if kind == EntityKind.EVIDENCE:
            next_level, points = max(self.evidence, EvidenceLevel.L1), 80
        elif kind == EntityKind.CHECKSUM:
            next_level, points = max(self.evidence, EvidenceLevel.L2), 110
        elif kind == EntityKind.CI_TEST:
            next_level, points = EvidenceLevel.L3, 140
        else:
            self.feedback = FeedbackCode.INCORRECT_ACTION
            self.penalize_incorrect(2, 1)
            return
        self.entities[index].active = False
        self.evidence = EvidenceLevel(next_level)
        self.feedback = FeedbackCode.EVIDENCE_REGISTERED
        self.reward(4, 2, points)

    def attempt_release(self) -> None:
        self.refresh_dynamic_gates()
        if self.release_blockers() == 0:
            self.status = BoundaryStatus.SEALED
            self.terminal_reason = TerminalReason.RELEASED
            self.feedback = FeedbackCode.RELEASE_SEALED
            self.score = min(0xFFFFFFFF, self.score + 1000)
        else:
            self.feedback = FeedbackCode.RELEASE_BLOCKED
            self.risk = min(100, self.risk + 1)
            self.streak = 0

    def reward(self, trust_gain: int, risk_reduction: int, points: int) -> None:
        self.trust = min(100, self.trust + trust_gain)
        self.risk = max(0, self.risk - risk_reduction)
        self.score = min(0xFFFFFFFF, self.score + points)
        self.streak = min(0xFFFF, self.streak + 1)
        self.best_streak = max(self.best_streak, self.streak)

    def penalize_incorrect(self, risk_gain: int, integrity_loss: int) -> None:
        self.risk = min(100, self.risk + risk_gain)
        self.integrity = max(0, self.integrity - integrity_loss)
        self.streak = 0

    def refresh_dynamic_gates(self) -> None:
        if self.consent.allows(CONSENT_SCOPE_REQUIRED):
            self.review_gates |= GATE_CONSENT
        else:
            self.review_gates &= ~GATE_CONSENT
        if self.evidence >= EvidenceLevel.L2:
            self.review_gates |= GATE_EVIDENCE
        if self.evidence >= EvidenceLevel.L3:
            self.review_gates |= GATE_DETERMINISM
        if self.privacy_containments > 0 and self.raw_leaks == 0:
            self.review_gates |= GATE_PRIVACY
        elif self.raw_leaks > 0:
            self.review_gates &= ~GATE_PRIVACY

    def enforce_terminal_invariants(self) -> None:
        if self.is_terminal():
            return
        if self.raw_leaks >= self.config.raw_leak_limit:
            self.status = BoundaryStatus.BREACHED
            self.terminal_reason = TerminalReason.RAW_LEAK_LIMIT
        elif self.integrity == 0:
            self.status = BoundaryStatus.UNSAFE
            self.terminal_reason = TerminalReason.INTEGRITY_COLLAPSE
        elif self.risk >= 100:
            self.status = BoundaryStatus.UNSAFE
            self.terminal_reason = TerminalReason.RISK_OVERFLOW

    def release_blockers(self) -> int:
        mask = 0
        if self.trust < 90:
            mask |= BLOCK_TRUST
        if self.risk > 20:
            mask |= BLOCK_RISK
        if self.integrity < 80:
            mask |= BLOCK_INTEGRITY
        if self.evidence < EvidenceLevel.L2:
            mask |= BLOCK_EVIDENCE
        if self.review_gates & ALL_REVIEW_GATES != ALL_REVIEW_GATES:
            mask |= BLOCK_REVIEW_GATES
        if not self.consent.allows(CONSENT_SCOPE_RELEASE):
            mask |= BLOCK_CONSENT
        if self.raw_leaks > 0:
            mask |= BLOCK_RAW_LEAK
        if self.is_terminal():
            mask |= BLOCK_TERMINAL
        if self.tick_value < minimum_release_tick(self.config.mode):
            mask |= BLOCK_REVIEW_WINDOW
        if any(entity.active for entity in self.entities):
            mask |= BLOCK_ACTIVE_ENTITIES
        return mask

    def release_ready(self) -> bool:
        return self.release_blockers() == 0

    def target_index(self, lane: int) -> int | None:
        best_index = None
        best_position = -1
        for index, entity in enumerate(self.entities):
            if not entity.active or entity.lane != lane or entity.kind == EntityKind.TYPED_INTENT:
                continue
            if entity.position > best_position:
                best_index, best_position = index, entity.position
        return best_index

    def spawn_next(self) -> None:
        kind = kind_for_spawn(self.config.mode, self.spawn_index, self.seed)
        lane = GUIDED_LANES[self.spawn_index % len(GUIDED_LANES)] if self.config.mode == RunMode.GUIDED else self.next_random() % LANE_COUNT
        flags = 0
        if kind == EntityKind.UNKNOWN_PACKET:
            candidate = self.spawn_index != 8 if self.config.mode == RunMode.GUIDED else self.spawn_index % 3 != 1
            if candidate:
                flags = 1
        slot = next((entity for entity in self.entities if not entity.active), None)
        if slot is None:
            self.risk = min(100, self.risk + 5)
            self.integrity = max(0, self.integrity - 2)
            return
        slot.active = True
        slot.id = self.next_entity_id
        slot.kind = kind
        slot.lane = int(lane)
        slot.position = SPAWN_X
        slot.speed = entity_speed(kind, self.config.mode, self.config.difficulty)
        slot.flags = flags
        slot.spawn_tick = self.tick_value
        self.next_entity_id = min(0xFFFFFFFF, self.next_entity_id + 1)
        self.spawn_index = min(0xFFFFFFFF, self.spawn_index + 1)

    def advance_entities(self) -> None:
        crossed: list[Entity] = []
        for entity in self.entities:
            if not entity.active:
                continue
            entity.position = min(0xFFFF, entity.position + entity.speed)
            if entity.position >= BOUNDARY_X:
                crossed.append(entity.clone())
                entity.active = False
        for entity in crossed:
            self.handle_crossing(entity)
            if self.is_terminal():
                break

    def handle_crossing(self, entity: Entity) -> None:
        kind = entity.kind
        if kind == EntityKind.TYPED_INTENT:
            self.feedback = FeedbackCode.TYPED_INTENT_RELEASED
            self.reward(3, 1, 120)
        elif kind == EntityKind.RAW_FRAME:
            self.raw_leaks = min(0xFF, self.raw_leaks + 1)
            self.risk = min(100, self.risk + 24)
            self.integrity = max(0, self.integrity - 14)
            self.streak = 0
            self.feedback = FeedbackCode.RAW_LEAK
        elif kind == EntityKind.STIMULATION_COMMAND:
            self.status = BoundaryStatus.UNSAFE
            self.terminal_reason = TerminalReason.STIMULATION_CROSSED
            self.feedback = FeedbackCode.STIMULATION_BREACH
        elif kind in (EntityKind.ARTIFACT, EntityKind.UNKNOWN_PACKET, EntityKind.VALIDATED_INTENT):
            self.risk = min(100, self.risk + 12)
            self.integrity = max(0, self.integrity - 7)
            self.streak = 0
        elif kind in (EntityKind.UNSUPPORTED_CLAIM, EntityKind.UNTRACEABLE_CLAIM, EntityKind.ROADMAP_AS_FACT):
            self.risk = min(100, self.risk + 15)
            self.integrity = max(0, self.integrity - 6)
            self.streak = 0
        elif kind == EntityKind.REVOKED_CONSENT:
            self.consent = ConsentState()
            self.review_gates &= ~GATE_CONSENT
            self.risk = min(100, self.risk + 10)
            self.integrity = max(0, self.integrity - 3)
            self.streak = 0
            self.feedback = FeedbackCode.CONSENT_REVOKED
        elif kind == EntityKind.CONSENT_TOKEN:
            self.risk = min(100, self.risk + 5)
            self.streak = 0
        elif kind in (EntityKind.EVIDENCE, EntityKind.CHECKSUM, EntityKind.CI_TEST):
            self.risk = min(100, self.risk + 4)
            self.streak = 0

    def next_random(self) -> int:
        x = self.rng_state
        x ^= x >> 12
        x &= MASK64
        x ^= (x << 25) & MASK64
        x &= MASK64
        x ^= x >> 27
        x &= MASK64
        self.rng_state = x
        return (x * 0x2545F4914F6CDD1D) & MASK64

    def active_entities(self) -> list[Entity]:
        return [entity.clone() for entity in self.entities if entity.active]

    def state_hash(self) -> int:
        value = 0xCBF29CE484222325
        chunks: Iterable[bytes] = [
            bytes([1]),
            hash_int(self.seed, 8),
            bytes([self.config.mode]),
            bytes([self.config.difficulty]),
            hash_int(self.config.max_ticks, 4),
            bytes([self.config.raw_leak_limit]),
            hash_int(self.tick_value, 4),
            hash_int(self.next_spawn_tick, 4),
            hash_int(self.spawn_index, 4),
            hash_int(self.next_entity_id, 4),
            hash_int(self.rng_state, 8),
            bytes([self.selected_lane, self.trust, self.risk, self.integrity, self.evidence, self.review_gates]),
            bytes([1 if self.consent.active else 0, self.consent.scope]),
            hash_int(self.consent.activation_tick, 4),
            hash_int(self.consent.expiry_tick, 4),
            bytes([self.raw_leaks]),
            hash_int(self.privacy_containments, 2),
            hash_int(self.score, 4),
            hash_int(self.streak, 2),
            hash_int(self.best_streak, 2),
            bytes([self.status, self.terminal_reason, self.feedback]),
        ]
        for chunk in chunks:
            for byte in chunk:
                value = fnv_byte(value, byte)
        for entity in self.entities:
            chunks = [
                bytes([1 if entity.active else 0]),
                hash_int(entity.id, 4),
                bytes([entity.kind, entity.lane]),
                hash_int(entity.position, 2),
                hash_int(entity.speed, 2),
                bytes([entity.flags]),
                hash_int(entity.spawn_tick, 4),
            ]
            for chunk in chunks:
                for byte in chunk:
                    value = fnv_byte(value, byte)
        return value

    def expected(self) -> dict:
        return {
            "terminal_tick": self.tick_value,
            "status": STATUS_NAME[self.status],
            "reason": REASON_NAME[self.terminal_reason],
            "state_hash": f"{self.state_hash():016x}",
            "trust": self.trust,
            "risk": self.risk,
            "integrity": self.integrity,
            "evidence": int(self.evidence),
            "review_gates": self.review_gates,
            "raw_leaks": self.raw_leaks,
            "score": self.score,
        }


ACTION_BY_NAME = {
    "validate": BoundaryAction.VALIDATE,
    "convert": BoundaryAction.CONVERT,
    "quarantine": BoundaryAction.QUARANTINE,
    "consent": BoundaryAction.CONSENT,
    "evidence": BoundaryAction.EVIDENCE,
    "release": BoundaryAction.RELEASE,
}
ACTION_NAME = {value: key for key, value in ACTION_BY_NAME.items()}
MODE_NAME = {
    RunMode.GUIDED: "guided",
    RunMode.STANDARD: "standard",
    RunMode.AUDIT: "audit",
    RunMode.GRAND: "grand",
    RunMode.DAILY: "daily",
}
STATUS_NAME = {
    BoundaryStatus.OPEN: "open",
    BoundaryStatus.SEALED: "sealed",
    BoundaryStatus.DEGRADED: "degraded",
    BoundaryStatus.BREACHED: "breached",
    BoundaryStatus.UNSAFE: "unsafe",
}
REASON_NAME = {
    TerminalReason.NONE: "none",
    TerminalReason.RELEASED: "released",
    TerminalReason.RAW_LEAK_LIMIT: "raw-leak-limit",
    TerminalReason.STIMULATION_CROSSED: "stimulation-crossed",
    TerminalReason.INTEGRITY_COLLAPSE: "integrity-collapse",
    TerminalReason.RISK_OVERFLOW: "risk-overflow",
    TerminalReason.TIME_EXPIRED: "time-expired",
    TerminalReason.INVARIANT_VIOLATION: "invariant-violation",
}


def replay_event(simulation: Simulation, action: BoundaryAction, lane: int) -> dict:
    simulation.select_lane(lane)
    simulation.apply_action(action)
    return {"tick": simulation.tick_value, "lane": lane, "action": ACTION_NAME[action]}


def run_autopilot(seed: int, mode: RunMode, difficulty: Difficulty = Difficulty.STANDARD) -> tuple[Simulation, list[dict]]:
    simulation = Simulation(SimulationConfig(seed, mode, difficulty))
    events: list[dict] = []
    handled: set[tuple[int, int]] = set()
    while not simulation.is_terminal():
        simulation.tick()
        visible = sorted(simulation.active_entities(), key=lambda entity: entity.position, reverse=True)
        for entity in visible:
            if entity.kind == EntityKind.TYPED_INTENT:
                continue
            if entity.kind == EntityKind.VALIDATED_INTENT:
                if simulation.consent.allows(CONSENT_SCOPE_CONVERT) and simulation.evidence >= EvidenceLevel.L1:
                    action = BoundaryAction.CONVERT
                else:
                    continue
            else:
                action = entity.kind.required_action()
            key = (entity.id, int(action))
            if key in handled:
                continue
            events.append(replay_event(simulation, action, entity.lane))
            handled.add(key)
            if action == BoundaryAction.VALIDATE:
                updated = next((item for item in simulation.active_entities() if item.id == entity.id), None)
                if updated and updated.kind == EntityKind.VALIDATED_INTENT and simulation.consent.allows(CONSENT_SCOPE_CONVERT) and simulation.evidence >= EvidenceLevel.L1:
                    events.append(replay_event(simulation, BoundaryAction.CONVERT, updated.lane))
                    handled.add((updated.id, int(BoundaryAction.CONVERT)))
        if simulation.tick_value > 900 and simulation.release_ready():
            events.append(replay_event(simulation, BoundaryAction.RELEASE, simulation.selected_lane))
    return simulation, events


def replay_document(simulation: Simulation, events: list[dict], summary: str) -> dict:
    return {
        "schema": REPLAY_SCHEMA,
        "product_version": CORE_VERSION,
        "core_version": CORE_VERSION,
        "hash_algorithm": HASH_ALGORITHM,
        "seed": simulation.seed,
        "mode": MODE_NAME[simulation.config.mode],
        "difficulty": "standard",
        "tick_rate": TICK_RATE,
        "initial_config": {
            "max_ticks": simulation.config.max_ticks,
            "raw_leak_limit": simulation.config.raw_leak_limit,
        },
        "events": events,
        "expected": simulation.expected(),
        "summary": summary,
    }
