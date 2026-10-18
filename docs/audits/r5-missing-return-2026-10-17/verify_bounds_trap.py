"""Replay the A5 bounds fixture with a strict Luna trap signature.

Run after the active Cargo qualification lane finishes. This reads its already
rebuilt exact-pin sysroot, clones isolated provider roots, and performs only
public CLI build/native execution. It never rebuilds a shared canonical root.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def copy_providers(source, destination, extensions):
    for path in source.rglob("*"):
        if path.is_file() and path.suffix in extensions:
            target = destination / path.relative_to(source)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, target)


def main():
    parser = argparse.ArgumentParser()
    for name in ("checkout", "debug", "release", "runtime", "out"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--pin", required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=args.checkout, text=True).strip()
    if head != args.pin:
        raise RuntimeError("Wrong implementation pin")
    subprocess.run(["git", "diff", "--exit-code", "HEAD", "--"], cwd=args.checkout, check=True)
    fixture = args.checkout / "tests/luna/language/raw_slice_contract/a5_slice_index_oob_trap.ln"
    providers = args.checkout / "luna-rs/libs/external"
    rows = []
    for profile, cli in (("debug", args.debug), ("release", args.release)):
        cli_hash = digest(cli)
        for mode in ("source", "artifact"):
            work = args.out / (profile + "-" + mode)
            root = work / "sysroot"
            external = root / "libs/external"
            copy_providers(providers, external, {".ln", ".toml"} if mode == "source" else {".llib", ".obj", ".toml"})
            assert not list(external.rglob("*.llib")) if mode == "source" else not list(external.rglob("*.ln"))
            program = work / "program.ln"
            shutil.copyfile(fixture, program)
            executable = work / ("program.exe" if os.name == "nt" else "program")
            env = os.environ.copy()
            env.update(LUNA_SYSROOT=str(root.resolve()), LUNA_RUNTIME_LIB=str(args.runtime.resolve()))
            command = [str(cli.resolve()), "build", str(program.resolve()), "--quiet", "-I", str(root.resolve()), "-o", str(executable.resolve())]
            build = subprocess.run(command, env=env, capture_output=True, timeout=120)
            (work / "build.stdout").write_bytes(build.stdout)
            (work / "build.stderr").write_bytes(build.stderr)
            run = None
            if build.returncode == 0 and executable.is_file():
                run = subprocess.run([str(executable.resolve())], env=env, capture_output=True, timeout=30)
                (work / "run.stdout").write_bytes(run.stdout)
                (work / "run.stderr").write_bytes(run.stderr)
            stderr = run.stderr.decode("utf-8", errors="replace") if run else ""
            valid = (build.returncode == 0 and run is not None and run.returncode != 0
                     and "luna: PANIC [L002]" in stderr and "index 3, length 2" in stderr)
            row = dict(profile=profile, mode=mode, command=command, build_exit=build.returncode,
                       native_exit=run.returncode if run else None, expected="L002: index 3, length 2",
                       status="PASS" if valid else "FAIL")
            rows.append(row)
            print(json.dumps(row), flush=True)
            assert digest(cli) == cli_hash, "CLI changed during replay"
    result = dict(tested_commit=head, fixture=str(fixture), fixture_sha256=digest(fixture),
                  runtime_sha256=digest(args.runtime), debug_sha256=digest(args.debug), release_sha256=digest(args.release),
                  rows=rows, status="PASS" if all(r["status"] == "PASS" for r in rows) else "FAIL")
    (args.out / "summary.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
