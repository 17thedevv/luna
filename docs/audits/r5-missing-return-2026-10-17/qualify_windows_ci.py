"""Run the existing exact-pin Windows instrument with hosted-runner tool paths."""
import importlib.util
import argparse
import json
import os
from pathlib import Path
import shutil
import traceback


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--pass", dest="lane_pass", choices=["normal", "triage", "final"], default="normal")
    args = parser.parse_args()
    instrument = Path(__file__).resolve().parent.parent / "r5-qualification-2026-10-14/harness/windows_qualification.py"
    spec = importlib.util.spec_from_file_location("r5_windows", instrument)
    lane = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(lane)
    lane.CARGO = Path(shutil.which("cargo.exe"))
    lane.RUSTC = Path(shutil.which("rustc.exe"))
    lane.LLVM = Path(os.environ["R5_LLVM_HOME"])
    lane.ENV = os.environ.copy()
    lane.ENV["PATH"] = str(lane.LLVM / "bin") + ";" + lane.ENV["PATH"]
    lane.ENV.update(LLVM_SYS_180_PREFIX=str(lane.LLVM), LLVM_CONFIG_PATH=str(lane.LLVM / "bin/llvm-config.exe"),
                    LLVM_HOME=str(lane.LLVM), CARGO_TARGET_DIR=str(lane.BUILD / "cargo-target"),
                    LUNA_SYSROOT=str(lane.SRC / "luna-rs"), LUNA_RUNTIME_LIB=str(lane.BUILD / "runtime/libluna-runtime.a"))
    lane.OUT.mkdir(parents=True, exist_ok=False)
    lane.save(lane.OUT / "toolchain-acquisition.json", json.loads((lane.LLVM / "acquisition.json").read_text()))
    error = None
    try:
        stages = ("acquire", "build", "focused", "diagnostics", "examples", "quickstart", "comptime") if args.lane_pass == "normal" else ("acquire", "build")
        for stage in stages:
            getattr(lane, stage)()
            lane.event("STAGE_COMPLETED", stage=stage)
        if args.lane_pass != "normal":
            command = [lane.CARGO, "test", "--locked", "--workspace"]
            if args.lane_pass == "triage":
                command.append("--no-fail-fast")
            result = lane.run("workspace-" + args.lane_pass, command, cwd=lane.SRC / "luna-rs",
                              extra_env=dict(LUNA_E1_RELEASE_CLI=str(lane.RELEASE)))
            raw = (lane.OUT / result["log"]).read_text(encoding="utf-8", errors="replace")
            if not result["result_groups"] or result["failed"] or "missing_return_cli" not in raw or "native_math_link_cli" not in raw:
                raise RuntimeError("Complete workspace summaries and both compiler regression suites are required")
        else:
            lane.run("strict-bounds", ["python", Path(__file__).with_name("verify_bounds_trap.py"),
                     "--checkout", lane.SRC, "--debug", lane.CLI, "--release", lane.RELEASE,
                     "--runtime", lane.BUILD / "runtime/libluna-runtime.a", "--out", lane.OUT / "strict-bounds",
                     "--pin", lane.PIN])
        lane.identity()
        status = lane.git("status", "--porcelain")
        if status:
            raise RuntimeError("Final exact-pin checkout is not clean: " + status)
    except BaseException:
        error = traceback.format_exc()
        (lane.OUT / "failure.txt").write_text(error, encoding="utf-8")
        print(error, flush=True)
    finally:
        lane.save(lane.OUT / "ci-summary.json", dict(tested_commit=lane.PIN, harness_commit=os.environ.get("GITHUB_SHA"),
                  workflow_run=os.environ.get("GITHUB_RUN_ID"), status="FAIL" if error else "PASS_IN_EXECUTED_SCOPE",
                  failure=error, lane=args.lane_pass,
                  scope="Windows GNU native normal plus strict bounds replay" if args.lane_pass == "normal" else "Complete Windows workspace " + args.lane_pass + "; fresh clone/build/sysroot; default parallelism",
                  release="BLOCKED PENDING COMPLETE R5 RECONCILIATION"))
    return 1 if error else 0


if __name__ == "__main__":
    raise SystemExit(main())
