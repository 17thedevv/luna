"""Independent verification of a local exact-pin lane's recorded evidence.

Recomputes recorded log/archive digests and lists recorded exits. This is an
integrity check, not a lane PASS verdict. Interrupted attempts are preserved
but excluded: their renamed records no longer resolve against the active root.
"""
import hashlib, json, pathlib, sys

LANE = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else r"D:/r5-8525067f/windows")
PIN = sys.argv[2] if len(sys.argv) > 2 else "8525067f5e6b7c8d711e9df0cb5acf9606b1b0e5"

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

checked = mismatched = missing = 0
rows = []
excluded = []
nonzero_exits = []
for path in sorted(LANE.rglob("*.json")):
    if "interrupted" in path.relative_to(LANE).parts:
        excluded.append(path.relative_to(LANE).as_posix())
        continue
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    if not isinstance(data, dict):
        continue
    label = data.get("label")
    if not label:
        continue
    if data.get("exit", 0) != 0 and data.get("log_sha256"):
        nonzero_exits.append((path.relative_to(LANE).as_posix(), data["exit"]))
    if data.get("tested_commit") and data["tested_commit"] != PIN:
        rows.append((label, "WRONG PIN", data["tested_commit"][:12]))
        mismatched += 1
    for key, digest_key in (("log", "log_sha256"), ("log_archive", "archive_sha256")):
        rel = data.get(key)
        recorded = data.get(digest_key)
        if not rel or not recorded:
            continue
        target = LANE / rel
        if not target.exists():
            rows.append((label, f"MISSING {rel}", ""))
            missing += 1
            continue
        actual = sha(target)
        checked += 1
        if actual != recorded:
            rows.append((label, f"HASH MISMATCH {rel}", f"{recorded[:12]} != {actual[:12]}"))
            mismatched += 1

print(f"lane={LANE}")
print(f"digests_checked={checked} mismatched={mismatched} missing={missing}")
print(f"interrupted_records_excluded={len(excluded)} (never counted as qualification evidence)")
for path, code in nonzero_exits:
    print(f"  recorded nonzero exit={code}: {path} (integrity does not establish PASS)")
for label, kind, detail in rows[:30]:
    print(f"  {kind}: {label} {detail}")

# Stage summary roll-up: every recorded stage must report its exit code.
summary_files = sorted(LANE.glob("*-summary.json")) + sorted(LANE.glob("quickstart.json"))
for path in summary_files:
    data = json.loads(path.read_text(encoding="utf-8"))
    status = data.get("status")
    if status is None and "exit" in data:
        status = f"exit={data['exit']}"
    if status is None and isinstance(data.get("gates"), list):
        from collections import Counter
        status = dict(Counter(g.get("result", "UNKNOWN") for g in data["gates"]))
    print(f"  stage {path.name}: {status}")

sys.exit(1 if mismatched or missing or checked == 0 else 0)
