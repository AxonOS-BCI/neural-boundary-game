#!/usr/bin/env python3
"""Regenerate canonical replay vectors and SHA-256 checksums."""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

from reference_model import (
    ACTION_NAME,
    BoundaryAction,
    Difficulty,
    EntityKind,
    EvidenceLevel,
    RunMode,
    Simulation,
    SimulationConfig,
    daily_seed,
    replay_document,
    replay_event,
    run_autopilot,
)

ROOT = Path(__file__).resolve().parents[1]
VECTOR_DIR = ROOT / "vectors"


def write_vector(name: str, document: dict) -> None:
    path = VECTOR_DIR / name
    path.write_text(json.dumps(document, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def idle_run(config: SimulationConfig) -> Simulation:
    simulation = Simulation(config)
    simulation.tick_many(simulation.config.max_ticks + 1000)
    return simulation


def run_script(config: SimulationConfig, events: list[dict]) -> Simulation:
    simulation = Simulation(config)
    for event in events:
        simulation.apply_event(event)
    simulation.tick_many(simulation.config.max_ticks + 1000)
    return simulation


def guided_event(index: int, action: str, lane: int, offset: int = 0) -> dict:
    return {"tick": 60 + 210 * index + offset, "lane": lane, "action": action}


def stimulation_failure() -> tuple[Simulation, list[dict]]:
    simulation = Simulation(SimulationConfig(58, RunMode.GUIDED, Difficulty.STANDARD))
    events: list[dict] = []
    handled: set[tuple[int, int]] = set()
    while not simulation.is_terminal():
        simulation.tick()
        visible = sorted(simulation.active_entities(), key=lambda entity: entity.position, reverse=True)
        for entity in visible:
            if entity.kind in (EntityKind.TYPED_INTENT, EntityKind.STIMULATION_COMMAND):
                continue
            if entity.kind == EntityKind.VALIDATED_INTENT:
                if simulation.consent.active and simulation.evidence >= EvidenceLevel.L1:
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
                if updated and updated.kind == EntityKind.VALIDATED_INTENT and simulation.consent.active and simulation.evidence >= EvidenceLevel.L1:
                    events.append(replay_event(simulation, BoundaryAction.CONVERT, updated.lane))
                    handled.add((updated.id, int(BoundaryAction.CONVERT)))
    return simulation, events


def main() -> None:
    VECTOR_DIR.mkdir(parents=True, exist_ok=True)
    for old in VECTOR_DIR.glob("*.json"):
        old.unlink()

    guided, guided_events = run_autopilot(58, RunMode.GUIDED)
    write_vector(
        "01-clean-sealed.json",
        replay_document(guided, guided_events, "Guided run seals after the complete consent, evidence, privacy, determinism, and release-review sequence."),
    )

    idle = idle_run(SimulationConfig(58, RunMode.STANDARD, Difficulty.STANDARD))
    write_vector(
        "02-idle-breach.json",
        replay_document(idle, [], "No operator input: unsafe packets cross until a deterministic terminal boundary failure."),
    )

    revoked_events = [
        guided_event(0, "quarantine", 2),
        guided_event(1, "consent", 1),
        guided_event(2, "evidence", 3),
        guided_event(3, "validate", 2),
        guided_event(3, "convert", 2, 1),
        guided_event(4, "evidence", 4),
        guided_event(5, "evidence", 0),
        guided_event(6, "quarantine", 1),
        guided_event(7, "consent", 3),
        guided_event(8, "validate", 2),
        guided_event(8, "quarantine", 2, 1),
        guided_event(10, "evidence", 4),
        guided_event(11, "quarantine", 0),
        guided_event(12, "validate", 2),
        guided_event(13, "quarantine", 4),
        guided_event(14, "evidence", 1),
        guided_event(15, "quarantine", 3),
        {"tick": 3250, "lane": 3, "action": "release"},
    ]
    revoked_config = SimulationConfig(77, RunMode.GUIDED, Difficulty.STANDARD, max_ticks=3350, raw_leak_limit=3)
    revoked = run_script(revoked_config, revoked_events)
    write_vector(
        "03-revoked-consent.json",
        replay_document(revoked, revoked_events, "Consent is revoked and never re-established; release remains fail-closed and the review window expires."),
    )

    raw_config = SimulationConfig(91, RunMode.GUIDED, Difficulty.STANDARD, max_ticks=3600, raw_leak_limit=1)
    raw = idle_run(raw_config)
    write_vector(
        "04-raw-leak.json",
        replay_document(raw, [], "A single raw frame crossing reaches the configured fail-closed leak limit."),
    )

    stimulation, stimulation_events = stimulation_failure()
    write_vector(
        "05-stimulation-fail-closed.json",
        replay_document(stimulation, stimulation_events, "All routine packets are handled, but one stimulation command is intentionally allowed to cross and terminates the run immediately."),
    )

    audit, audit_events = run_autopilot(404, RunMode.AUDIT)
    write_vector(
        "06-audit-sealed.json",
        replay_document(audit, audit_events, "Adversarial Audit Run with revocation, claims, and stimulation pressure seals deterministically."),
    )

    grand, grand_events = run_autopilot(9001, RunMode.GRAND)
    write_vector(
        "07-grand-run-sealed.json",
        replay_document(grand, grand_events, "Extended Grand Run completes the sovereign boundary review and seals."),
    )

    daily_value = daily_seed(20260611)
    daily, daily_events = run_autopilot(daily_value, RunMode.DAILY)
    write_vector(
        "08-daily-seed-sealed.json",
        replay_document(daily, daily_events, "Daily Seed for 2026-06-11 UTC. The date, schema, and input stream reproduce this terminal state without a backend."),
    )

    checksum_lines = []
    for path in sorted(VECTOR_DIR.glob("*.json")):
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        checksum_lines.append(f"{digest}  {path.name}")
    (VECTOR_DIR / "checksums.sha256").write_text("\n".join(checksum_lines) + "\n", encoding="utf-8")

    for path in sorted(VECTOR_DIR.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        expected = data["expected"]
        print(f"{path.name}: {expected['status']} / {expected['reason']} / tick {expected['terminal_tick']} / {expected['state_hash']}")


if __name__ == "__main__":
    main()
