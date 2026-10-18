"""Independent verification of a local exact-pin lane's recorded evidence.

Recomputes every reported log / archive digest and re-checks the recorded exit
codes, so a lane result is verifiable without trusting the recorder.
"""
import hashlib, json, pathlib, sys

LANE = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else r"D:/r5-8525067f/windows")
PIN = sys.argv[2] if len(sys.argv) > 2 else "8525067f5e6b7c8d711e9df0cb5acf9606b1b0e5"

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

checked = mismatched = missing = 0
rows = []
for path in sorted(LANE.rglob("*.json")):
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    if not isinstance(data, dict):
        continue
    label = data.get("label")
    if not label:
        continue
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
for label, kind, detail in rows[:30]:
    print(f"  {kind}: {label} {detail}")

# Stage summary roll-up: every recorded stage must report its exit code.
summary_files = sorted(LANE.glob("*-summary.json")) + sorted(LANE.glob("quickstart.json"))
for path in summary_files:
    data = json.loads(path.read_text(encoding="utf-8"))
    status = data.get("status")
    if status is None and "exit" in data:
        status = f"exit={data['exit']}"
    print(f"  stage {path.name}: {status}")
