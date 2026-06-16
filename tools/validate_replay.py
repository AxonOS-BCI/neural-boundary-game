#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Denis Yermakou
# SPDX-FileContributor: AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
from __future__ import annotations
import hashlib, json, re, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VECTORS = ROOT / "vectors"
MODES = {"GUIDED","STANDARD","AUDIT","GRAND","DAILY","PRIVACY_VAULT","KERNEL_TRIAL"}
ACTIONS = {"VALIDATE","CONVERT","QUARANTINE","CONSENT","EVIDENCE","RELEASE"}
STATUSES = {"SEALED","BREACHED","UNSAFE","ABORTED","FATAL_RUNTIME","RUNNING"}
REASONS = {"SUCCESS_RELEASE","TIMEOUT_UNSEALED","RISK_OVERFLOW","INTEGRITY_COLLAPSE","RAW_LEAK_LIMIT","UNSAFE_STIMULATION_ESCAPE","DEADLINE_BREACH","DETERMINISM_MISMATCH","REPLAY_SCHEMA_ERROR","WASM_INIT_FAILURE","USER_ABORT"}
GRADES = {"SOVEREIGN","SEALED","REVIEWABLE","DEGRADED","BREACHED","UNSAFE"}
EV_LEVELS = {"L0","L1","L2","L3"}
REQUIRED = ["01-standard-clean-sealed.json","02-standard-idle-risk-overflow.json","03-standard-lapse-revocations-sealed.json","04-standard-idle-raw-leak-limit.json","05-standard-idle-stimulation.json","06-audit-clean-sealed.json","07-grand-clean-sealed.json","08-daily-2026-06-14-sealed.json"]
EXPECTED_TYPES: dict = {"terminal_tick":int,"status":str,"terminal_reason":str,"grade":str,"trust":int,"risk":int,"integrity":int,"evidence_level":str,"evidence_bits":int,"gate_mask":int,"gates_passed":int,"raw_leaks":int,"typed_intents":int,"quarantined":int,"wrong_actions":int,"score":int,"best_combo":int,"revocations":int,"state_hash":str}

def load_manifest():
    with open(ROOT/"release.toml","rb") as f: return tomllib.load(f)

def _fnv(b):
    h=0xCBF29CE484222325
    for x in b: h=((h^x)*0x100000001B3)&0xFFFFFFFFFFFFFFFF
    return h

def _xss(x):
    x&=0xFFFFFFFFFFFFFFFF; x^=x>>12; x^=(x<<25)&0xFFFFFFFFFFFFFFFF; x^=x>>27
    return (x*0x2545F4914F6CDD1D)&0xFFFFFFFFFFFFFFFF

def daily_seed_py(year,month,day):
    s=f"NBG|5.5.12|{year:04d}-{month:02d}-{day:02d}|DAILY"
    h=_fnv(s.encode()); seed=h or 0x3001; r=_xss(seed); return r or 0x3001

def validate_vector(path,identity,errors):
    name=path.name
    try: data=json.loads(path.read_text())
    except json.JSONDecodeError as e: errors.append(f"{name}: JSON error: {e}"); return
    for field,expected in [("schema",identity["replay_schema"]),("product_version",identity["version"]),("core_version",identity["version"]),("hash_algorithm",identity["state_hash_algorithm"]),("rng_algorithm",identity["rng_algorithm"])]:
        if data.get(field)!=expected: errors.append(f"{name}: {field} must be {expected!r}")
    if data.get("abi_version")!=1: errors.append(f"{name}: abi_version must be 1")
    if data.get("tick_rate_hz")!=60: errors.append(f"{name}: tick_rate_hz must be 60")
    if data.get("mode") not in MODES: errors.append(f"{name}: mode invalid")
    diff=data.get("difficulty")
    if not isinstance(diff,int) or diff not in (0,1,2): errors.append(f"{name}: difficulty must be int 0|1|2")
    sr=data.get("seed",""); vsr=isinstance(sr,str) and bool(re.fullmatch(r"[0-9a-f]{16}",sr))
    if not vsr: errors.append(f"{name}: seed must be 16 lowercase hex digits")
    elif int(sr,16)==0: errors.append(f"{name}: seed must be non-zero")
    if data.get("mode")=="DAILY":
        dt=data.get("date","")
        if not (isinstance(dt,str) and re.fullmatch(r"\d{4}-\d{2}-\d{2}",dt)): errors.append(f"{name}: DAILY requires date YYYY-MM-DD")
        elif vsr:
            y,m,d=(int(x) for x in dt.split("-")); es=daily_seed_py(y,m,d); as2=int(sr,16)
            if es!=as2: errors.append(f"{name}: daily_seed mismatch py={es:016x} file={as2:016x}")
    exp=data.get("expected")
    if not isinstance(exp,dict): errors.append(f"{name}: expected block missing"); return
    for f,k in EXPECTED_TYPES.items():
        if not isinstance(exp.get(f),k): errors.append(f"{name}: expected.{f} must be {k.__name__}")
    if exp.get("status") not in STATUSES: errors.append(f"{name}: expected.status invalid")
    if exp.get("terminal_reason") not in REASONS: errors.append(f"{name}: expected.terminal_reason invalid")
    if exp.get("grade") not in GRADES: errors.append(f"{name}: expected.grade invalid")
    if exp.get("evidence_level") not in EV_LEVELS: errors.append(f"{name}: expected.evidence_level invalid")
    h=exp.get("state_hash","")
    if not (isinstance(h,str) and re.fullmatch(r"0x[0-9a-f]{16}",h)): errors.append(f"{name}: expected.state_hash must be 0x+16 hex")
    inputs=data.get("inputs")
    if not isinstance(inputs,list): errors.append(f"{name}: inputs must be list"); return
    last=0; tt=exp.get("terminal_tick",0) if isinstance(exp,dict) else 0
    for i,inp in enumerate(inputs):
        if not isinstance(inp,dict): errors.append(f"{name}: inputs[{i}] not object"); continue
        t=inp.get("tick")
        if not isinstance(t,int) or t<1 or t<=last: errors.append(f"{name}: inputs[{i}].tick not strictly increasing")
        elif isinstance(tt,int) and t>tt+600: errors.append(f"{name}: inputs[{i}].tick past terminal_tick+600")
        if not isinstance(inp.get("lane"),int) or not (0<=inp.get("lane",99)<=4): errors.append(f"{name}: inputs[{i}].lane must be 0..4")
        if inp.get("action") not in ACTIONS: errors.append(f"{name}: inputs[{i}].action {inp.get('action')!r} invalid")
        if isinstance(t,int): last=t

def validate_checksums(paths,errors):
    checks=VECTORS/"checksums.sha256"
    if not checks.exists(): errors.append("vectors/checksums.sha256 missing"); return
    listed={}
    for line in checks.read_text().splitlines():
        line=line.strip()
        if not line: continue
        parts=line.split(None,1)
        if len(parts)!=2: errors.append(f"checksums.sha256: malformed {line!r}"); continue
        listed[parts[1].strip()]=parts[0].lower()
    for path in paths:
        digest=hashlib.sha256(path.read_bytes()).hexdigest(); rec=listed.pop(path.name,None)
        if rec is None: errors.append(f"checksums.sha256: {path.name} missing")
        elif rec!=digest: errors.append(f"checksums.sha256: digest mismatch {path.name}")
    for orphan in listed: errors.append(f"checksums.sha256: {orphan} listed but absent")

def main():
    identity=load_manifest(); vectors=sorted(VECTORS.glob("*.json")); errors=[]
    names={p.name for p in vectors}
    for req in REQUIRED:
        if req not in names: errors.append(f"required vector missing: {req}")
    for path in vectors: validate_vector(path,identity,errors)
    validate_checksums(vectors,errors)
    if errors:
        print("Replay vector validation FAILED:")
        for e in errors: print(f"  - {e}")
        return 1
    print(f"Replay vectors OK ({len(vectors)} files, schema {identity['replay_schema']}, daily seed Python cross-check passed).")
    return 0

if __name__ == "__main__": sys.exit(main())
