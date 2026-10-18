"""Acquire a complete Ubuntu workspace pass at the immutable implementation pin.

Each invocation builds from a new remote detached clone. The triage and final
passes have separate fresh roots and job budgets; final is scheduled only after
triage succeeds. The tested source and existing lane evidence are never edited.
"""
import argparse
import gzip
import importlib.util
import json
import os
from pathlib import Path
import re
import traceback


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--pass", dest="phase", choices=["triage", "final"], required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    instrument = Path(__file__).resolve().parent.parent / "r5-qualification-2026-10-14/harness/ubuntu_qualification.py"
    spec = importlib.util.spec_from_file_location("r5_ubuntu", instrument)
    lane = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(lane)
    rec = lane.Recorder(args.out.resolve())
    error = None
    totals = None
    try:
        provenance = rec.acquire()
        root = rec.checkout
        for name, command in [("rustc", ["rustc", "-vV"]), ("llvm", ["llvm-config-18", "--version"]),
                              ("llvm-host-target", ["llvm-config-18", "--host-target"]), ("cargo", ["cargo", "--version"])]:
            provenance[name] = rec.text(rec.run(name, command, cwd=root)).strip()
        lane.save(rec.evidence / "provenance.json", provenance)
        runtime = rec.out / "runtime"
        rec.run("runtime-config", ["cmake", "-S", root / "runtime", "-B", runtime, "-G", "Ninja",
                                  "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_C_COMPILER=gcc", "-DCMAKE_CXX_COMPILER=g++"])
        rec.run("runtime-build", ["cmake", "--build", runtime, "--parallel"])
        rec.run("runtime-abi", ["ctest", "--test-dir", runtime, "-R", "^RuntimeAbiTests$",
                               "--output-on-failure", "--no-tests=error", "-V"])
        rec.env.update(LUNA_RUNTIME_LIB=str(runtime / "libluna-runtime.a"), LUNA_SYSROOT=str(root / "luna-rs"))
        cargo = ["cargo", "build", "--locked", "--manifest-path", root / "luna-rs/Cargo.toml", "-p", "luna-cli"]
        rec.run("cli-debug", cargo, cwd=root)
        rec.run("cli-release", cargo + ["--release"], cwd=root)
        debug, release = root / "luna-rs/target/debug/luna", root / "luna-rs/target/release/luna"
        rec.env.update(LUNA_E1_RELEASE_CLI=str(release), LUNA_DOCS_CLI=str(debug))
        lane.save(rec.evidence / "compiler-runtime-hashes.json", dict(tested_commit=lane.PIN,
                  debug_cli=lane.digest(debug), release_cli=lane.digest(release), runtime=lane.digest(runtime / "libluna-runtime.a")))
        rec.run("sysroot", [debug, "build-sysroot", "--quiet"], cwd=root)
        lane.providers(rec, provenance["llvm-host-target"])
        command = ["cargo", "test", "--locked", "--manifest-path", root / "luna-rs/Cargo.toml", "--workspace"]
        if args.phase == "triage":
            command.append("--no-fail-fast")
        result = rec.run("workspace-" + args.phase, command, cwd=root, required=False)
        text = rec.text(result)
        groups = re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text)
        totals = dict(groups=len(groups), passed=sum(int(x[0]) for x in groups),
                      failed=sum(int(x[1]) for x in groups), ignored=sum(int(x[2]) for x in groups),
                      missing_return_suite_observed="Running tests/missing_return_cli.rs" in text,
                      native_math_suite_observed="Running tests/native_math_link_cli.rs" in text,
                      move_origin_suite_observed="Running tests/move_origin_join_cli.rs" in text)
        if result["status"] != "PASS" or not groups or totals["passed"] == 0 or totals["failed"] or not all(totals[name] for name in ("missing_return_suite_observed", "native_math_suite_observed", "move_origin_suite_observed")):
            raise RuntimeError("Workspace did not complete with zero failures and all three compiler regression suites: " + json.dumps(totals))
        rec.run("protected-tree-diff", ["git", "diff", "--exit-code", "HEAD", "--"], cwd=root)
    except BaseException:
        error = traceback.format_exc()
        (rec.evidence / "failure.txt").write_text(error, encoding="utf-8")
        print(error, flush=True)
    finally:
        try:
            status = rec.run("final-status", ["git", "status", "--porcelain"], cwd=rec.checkout, required=False)
            if status["exit"] != 0 or rec.text(status).strip():
                error = error or "Final checkout identity is not clean"
            rec.run("disk-final", ["df", "-h"], required=False)
        except BaseException:
            error = error or traceback.format_exc()
        failed = [r["gate"] for r in rec.rows if r["status"] != "PASS"]
        lane.save(rec.evidence / "gates.json", rec.rows)
        lane.save(rec.evidence / "summary.json", dict(tested_commit=lane.PIN,
                  harness_commit=os.environ.get("GITHUB_SHA"), lane="workspace-" + args.phase,
                  workflow_run=os.environ.get("GITHUB_RUN_ID"), totals=totals,
                  status="FAIL" if error or failed else "PASS_IN_EXECUTED_LANE_SCOPE",
                  failure=error, failed_gates=failed, gate_count=len(rec.rows),
                  scope="Complete cargo test --workspace; final uses default parallelism",
                  release="BLOCKED PENDING COMPLETE R5 RECONCILIATION"))
        lane.save(rec.evidence / "evidence-hashes.json", {str(p.relative_to(rec.evidence)): lane.digest(p)
                  for p in sorted(rec.evidence.rglob("*")) if p.is_file() and p.name != "evidence-hashes.json"})
    return 1 if error or failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
