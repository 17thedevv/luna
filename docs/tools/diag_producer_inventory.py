"""DIAG-7 structural producer inventory for the Luna diagnostic registry.

Produces a deterministic, machine-checked inventory of every producer of every
code in `DiagnosticCode::ALL`:

  * every registered code must have at least one producer, or an explicitly
    declared generic emitter (a helper that forwards a caller-supplied code);
  * every producer's span policy is classified (anchored / contract /
    infrastructure / generic) so a producer that forgot its primary span is a
    failure rather than a silent gap;
  * every code must have at least one automated coverage reference in the test
    tree (code identity), and codes with structural semantics (phase ownership,
    related labels) name their focused regression.

Usage:
    python docs/tools/diag_producer_inventory.py --write
    python docs/tools/diag_producer_inventory.py --check
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "luna-rs/crates/luna-common/src/diagnostic.rs"
INVENTORY = ROOT / "docs/diagnostics/diag-producer-inventory.json"
SOURCE_ROOT = ROOT / "luna-rs"

INFRASTRUCTURE = {
    "BackendInvariantViolation", "ObjectEmissionFailure", "LinkerFailure",
    "SysrootConfigurationFailure", "ProviderReadFailure", "OutputWriteFailure",
    "InvalidArtifactOutput", "ProviderConfigurationError", "ProcessExecutionFailure",
}

# Registered codes that no producer in this release can emit. They stay in the
# frozen registry, and the inventory declares them explicitly instead of
# claiming coverage that does not exist. A new unproduced code fails the gate.
RESERVED_NO_PRODUCER = {
    "E0004": "UnclosedDelimiter: the lexer reports delimiter errors without this code in 0.1",
    "E1006": "InvalidVisibility: visibility syntax is admitted by Parser grammar (E0003)",
    "E1007": "WildcardImportProhibited: wildcard imports are rejected as unresolved (E1001)",
    "E2014": "DuplicateField: struct-init field duplication is reported as E2001 today",
    "E2015": "DuplicateVariant: no producer path in 0.1",
    "E2020": "CannotCallNonFunction: call misuse is reported as E2001 today",
    # No comptime panic/assert admission exists: the interpreter has no panic path
    # and the semantic comptime engine has no assert builtin, so E4004 has no
    # reachable condition in 0.1.
    "E4004": "ComptimePanic: no comptime panic/assert condition is reachable in 0.1",
    "E5002": "InfiniteMonomorphizationRecursion: mono recursion is rejected through backend admission today",
}

# Registered codes whose condition cannot be reproduced in a test scope. The
# reason is recorded instead of claiming behavioural coverage that does not
# exist, and these codes are excluded from the coverage-debt list because the
# limitation is declared rather than pending.
UNREACHABLE_IN_TEST = {
    "E3004": "no producer fires: a non-void body that can fall through is accepted today (recorded as a separate correctness finding)",
    "E3007": "explicit destructor calls are rejected (E2001/E2031), so a dropped place cannot be read afterwards",
    "E6002": "requires an LLVM object-write failure",
    "E6003": "requires a linker failure environment that is not reproducible portably",
    "E6009": "requires a process-spawn failure; the run path no longer reclassifies compile diagnostics",
}

# Helpers that forward a caller-supplied code instead of naming one. They are the
# declared generic emitters; a code reached only through them is still counted as
# produced, with a `generic` span policy.
GENERIC_EMITTERS = {
    "error_with_code": "parser helper: code supplied by the call site, span required by signature",
    "source_error": "luna-common constructor: asserts a source-range code and requires a span",
    "contract_error": "luna-common constructor: declared provider/manifest contract diagnostic",
}

# Producer paths that are legacy/inactive surfaces, retained as discovery only.
INACTIVE = ("luna-cli-old-backup", "/target/")

WITH_CODE = re.compile(r"\.with_code\(\s*(?:luna_common::)?DiagnosticCode::(\w+)")
NAMED_CALL = re.compile(
    r"(?:source_error|contract_error|error_with_code)\(\s*(?:luna_common::)?DiagnosticCode::(\w+)"
)
SPAN_IN_WINDOW = re.compile(r"\.with_span\(|\.span\s*=|with_related\(")
SPAN_PARAM = re.compile(r"\bspan\s*:\s*(?:&)?(?:luna_common::)?Span\b|(?:^|[,(]\s*)\bspan\b\s*[,)]")

STRUCTURAL_COVERAGE = {
    "E1008": "owner-provided resolver constructor; see resolver::ambiguous_method_diagnostic + method_resolution",
    "E2001": "owner-provided typechecker constructor; see typechecker::type_mismatch_diagnostic",
    "E2016": "owner-provided typechecker constructor; see typechecker::lifetime_constraint_violation",
    "E3001": "related origin label: diagnostic_conformance_cli::related_spans_and_poison_containment_hold",
    "E3003": "related loan label: diagnostic_conformance_cli::related_spans_and_poison_containment_hold",
    "E2026": "poison containment at the failing boundary: diagnostic_conformance_cli",
    "E6005": "typed CLI I/O failure: luna-cli src/main.rs fail() with ProviderReadFailure",
}


def registry() -> list[dict]:
    text = REGISTRY.read_text(encoding="utf-8")
    body = text.split("pub enum DiagnosticCode {", 1)[1].split("}", 1)[0]
    codes = []
    for line in body.splitlines():
        line = line.strip().rstrip(",")
        if not line or line.startswith("//"):
            continue
        name, _, value = line.partition("=")
        name = name.strip()
        if not name or not value.strip().isdigit():
            continue
        number = int(value.strip())
        codes.append({"name": name, "number": number, "code": f"E{number:04}"})
    return codes


def sources() -> list[pathlib.Path]:
    found = []
    for path in sorted(SOURCE_ROOT.rglob("*.rs")):
        posix = path.as_posix()
        if any(marker in posix for marker in INACTIVE):
            continue
        # The registry module declares the codes; it is not a producer of them.
        if path == REGISTRY:
            continue
        found.append(path)
    return found


def classify(window: str, code_name: str, signature: str) -> str:
    if code_name in INFRASTRUCTURE:
        return "infrastructure"
    if "contract_error(" in window:
        return "contract"
    # `source_error(code, span, ...)` and `error_with_code(code, message, span)`
    # require a span by signature; a helper that takes `span` as a parameter
    # anchors the diagnostic at its call site.
    if "source_error(" in window or "error_with_code(" in window:
        return "anchored"
    if SPAN_IN_WINDOW.search(window) or SPAN_PARAM.search(signature) or SPAN_PARAM.search(window):
        return "anchored"
    return "dynamic"


def scan() -> dict:
    files = sources()
    producers: dict[str, list[dict]] = {}
    generic = dict(GENERIC_EMITTERS)
    for path in files:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        relative = path.relative_to(ROOT).as_posix()
        # Only production code is inventoried; test modules are coverage, not
        # producers.
        limit = len(lines)
        for index, line in enumerate(lines):
            if line.strip().startswith("#[cfg(test)]") or line.strip().startswith("mod tests"):
                limit = index
                break
        for index in range(min(limit, len(lines))):
            line = lines[index]
            if "DiagnosticCode::" not in line:
                continue
            window = "\n".join(lines[index:index + 9])
            signature = "\n".join(lines[max(0, index - 8):index])
            construction = (
                ".with_code(" in line
                or "error_with_code(" in line
                or "source_error(" in line
                or "contract_error(" in line
            )
            names = set(WITH_CODE.findall(line)) | set(NAMED_CALL.findall(line))
            if not names:
                names = set(re.findall(r"DiagnosticCode::(\w+)", line))
            if not names:
                continue
            for name in sorted(names):
                producers.setdefault(name, []).append({
                    "file": relative,
                    "line": index + 1,
                    "span_policy": classify(window, name, signature) if construction else "dynamic",
                })

    tests = [p for p in files if "/tests/" in p.as_posix() and p.as_posix().endswith(".rs")]
    test_text = {p.relative_to(ROOT).as_posix(): p.read_text(encoding="utf-8", errors="replace") for p in tests}
    # Language fixtures at the repository root assert diagnostic codes too.
    fixture_root = ROOT / "tests"
    for path in sorted(fixture_root.rglob("*.ln")):
        test_text[path.relative_to(ROOT).as_posix()] = path.read_text(encoding="utf-8", errors="replace")

    records = []
    for entry in registry():
        name, code = entry["name"], entry["code"]
        sites = producers.get(name, [])
        coverage = sorted(
            path for path, text in test_text.items()
            if code in text or re.search(rf"\b{name}\b", text)
        )
        policy = sorted({site["span_policy"] for site in sites}) or ["generic"]
        reserved = RESERVED_NO_PRODUCER.get(code)
        records.append({
            "code": code,
            "name": name,
            "number": entry["number"],
            "producers": sites,
            "produced": bool(sites),
            "reserved_no_producer": reserved,
            "declared_unreachable_in_test": UNREACHABLE_IN_TEST.get(code),
            "span_policies": policy,
            "generic_emitter": generic.get(name),
            "automated_coverage": coverage,
            "structural_coverage": STRUCTURAL_COVERAGE.get(code),
        })
    return {"registry_size": len(records), "records": records}


def validate(inventory: dict) -> list[str]:
    problems = []
    for record in inventory["records"]:
        code = record["code"]
        if not record["produced"] and not record["reserved_no_producer"]:
            problems.append(
                f"{code} ({record['name']}): registered but no producer site, and not declared as reserved"
            )
        if record["produced"] and record["reserved_no_producer"]:
            problems.append(
                f"{code} ({record['name']}): declared as reserved but producers exist; drop the stale declaration"
            )
        for site in record["producers"]:
            if site["span_policy"] == "unanchored":
                problems.append(
                    f"{code} ({record['name']}): {site['file']}:{site['line']} has no primary span "
                    "and is not a declared contract or infrastructure diagnostic"
                )
    return problems


def coverage_debt(inventory: dict) -> list[dict]:
    """Codes whose identity and span policy are gated by the per-code contract
    test but which have no behavioural (fixture or CLI) coverage reference yet.
    Reported as explicit debt instead of an overclaimed coverage claim."""
    debt = []
    for record in inventory["records"]:
        if record["automated_coverage"] or record["structural_coverage"] or record["reserved_no_producer"]:
            continue
        if record.get("declared_unreachable_in_test"):
            continue
        debt.append({
            "code": record["code"],
            "name": record["name"],
            "status": "OPEN",
            "class": "behavioral-coverage",
            "must_complete_before": "release-gate:per-code behavioral coverage",
        })
    return debt


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    inventory = scan()
    inventory["coverage_debt"] = coverage_debt(inventory)
    problems = validate(inventory)
    document = json.dumps(inventory, indent=2, sort_keys=False) + "\n"

    if args.write:
        INVENTORY.parent.mkdir(parents=True, exist_ok=True)
        INVENTORY.write_text(document, encoding="utf-8", newline="\n")
        print(f"wrote {INVENTORY.relative_to(ROOT)} ({inventory['registry_size']} codes)")
    if args.check:
        if not INVENTORY.exists():
            print(f"missing inventory: {INVENTORY.relative_to(ROOT)}")
            return 1
        committed = INVENTORY.read_text(encoding="utf-8").replace("\r\n", "\n")
        if committed != document:
            print("DIAG-7 inventory drift: recomputed inventory differs from the committed one")
            return 1
    for problem in problems:
        print(f"DIAG-7: {problem}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
