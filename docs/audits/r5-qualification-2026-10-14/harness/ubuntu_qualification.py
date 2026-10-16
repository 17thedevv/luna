"""R5 acquisition instrument; never changes the immutable tested candidate.

The workflow/harness commit and tested commit are recorded separately. All build,
test and Luna executions consume the fresh remote detached clone. Output lives
outside that checkout. This instrument produces evidence, not a release verdict.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import sys
import time
import tomllib
import traceback

PIN = "c085449b207b5860b1d7a79bee221c2ff3d357c3"
REPOSITORY = "https://github.com/17thedevv/luna.git"
TARGET = "x86_64-unknown-linux-gnu"


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


class Recorder:
    def __init__(self, out):
        self.out = out
        self.evidence = out / "evidence"
        self.evidence.mkdir(parents=True, exist_ok=False)
        self.checkout = out / "checkout"
        self.rows = []
        self.env = os.environ.copy()
        self.native_target = None

    def run(self, name, args, *, cwd=None, required=True, timeout=21000):
        args = list(map(str, args))
        log = self.evidence / (name + ".log")
        row = dict(gate=name, tested_commit=PIN, command=args,
                   cwd=str(cwd or self.out), started_utc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
        print("START " + name, flush=True)
        start = time.monotonic()
        with log.open("wb") as stream:
            try:
                result = subprocess.run(args, cwd=cwd or self.out, env=self.env,
                                        stdout=stream, stderr=subprocess.STDOUT,
                                        timeout=timeout, check=False)
                row["exit"] = result.returncode
            except subprocess.TimeoutExpired:
                row.update(exit=None, timed_out=True)
            except OSError as error:
                stream.write(str(error).encode())
                row.update(exit=None, launch_error=str(error))
        row.update(seconds=time.monotonic() - start, log_sha256=digest(log),
                   status="PASS" if row["exit"] == 0 else "FAIL")
        # Compress completed logs without a logging pipe hiding the process exit.
        packed = log.with_suffix(".log.gz")
        with log.open("rb") as source, packed.open("wb") as dest:
            with gzip.GzipFile(filename="", fileobj=dest, mode="wb", mtime=0) as z:
                shutil.copyfileobj(source, z)
        row.update(log=packed.name, compressed_sha256=digest(packed))
        log.unlink()
        self.rows.append(row)
        save(self.evidence / (name + ".json"), row)
        save(self.evidence / "gates.json", self.rows)
        print(row["status"] + " " + name + " exit=" + str(row["exit"]), flush=True)
        if required and row["status"] != "PASS":
            raise RuntimeError("Required gate failed: " + name)
        return row

    def text(self, row):
        with gzip.open(self.evidence / row["log"], "rt", errors="replace") as f:
            return f.read()

    def acquire(self):
        if self.env.get("R5_TESTED_COMMIT", PIN) != PIN:
            raise RuntimeError("Tested commit override is forbidden")
        if self.env.get("R5_REPOSITORY_URL", REPOSITORY) != REPOSITORY:
            raise RuntimeError("Repository override is forbidden")
        self.run("clone", ["git", "clone", REPOSITORY, self.checkout])
        self.run("checkout", ["git", "checkout", "--detach", PIN], cwd=self.checkout)
        head = self.text(self.run("head", ["git", "rev-parse", "HEAD"], cwd=self.checkout)).strip()
        status = self.text(self.run("initial-status", ["git", "status", "--porcelain"], cwd=self.checkout))
        symbolic = self.run("detached", ["git", "symbolic-ref", "-q", "HEAD"], cwd=self.checkout, required=False)
        # A detached HEAD is the expected nonzero result of symbolic-ref.
        symbolic["status"] = "PASS" if symbolic["exit"] == 1 else "FAIL"
        save(self.evidence / "detached.json", symbolic)
        if head != PIN or status.strip() or symbolic["exit"] != 1:
            raise RuntimeError("Exact clean detached pin gate failed")
        provenance = dict(tested_commit=PIN, harness_commit=self.env.get("GITHUB_SHA"),
                          repository=REPOSITORY, initial_status=status, detached=True,
                          os=platform.platform(), machine=platform.machine(),
                          workflow_run=self.env.get("GITHUB_RUN_ID"),
                          workflow_attempt=self.env.get("GITHUB_RUN_ATTEMPT"),
                          workflow_url=("https://github.com/17thedevv/luna/actions/runs/" + self.env.get("GITHUB_RUN_ID", "")),
                          asan_scope="C runtime + RuntimeAbiTests only")
        for name, command in [
            ("remote-url", ["git", "remote", "get-url", "origin"]),
            ("os-release", ["cat", "/etc/os-release"]),
            ("uname", ["uname", "-a"]),
            ("gcc-target", ["gcc", "-dumpmachine"]),
            ("gcc-version", ["gcc", "--version"]),
            ("cmake-version", ["cmake", "--version"]),
            ("disk-initial", ["df", "-h"]),
            ("code-tree-identity", ["git", "ls-tree", "HEAD", "luna-rs", "runtime", "tests", "CMakeLists.txt"]),
        ]:
            provenance[name] = self.text(self.run(name, command, cwd=self.checkout)).strip()
        save(self.evidence / "provenance.json", provenance)
        if platform.system() != "Linux" or platform.machine() != "x86_64":
            raise RuntimeError("This lane requires native Ubuntu GNU64")
        if "Ubuntu" not in provenance["os-release"] or "linux-gnu" not in provenance["gcc-target"]:
            raise RuntimeError("Actual advertised target mismatch")
        return provenance


def providers(rec, llvm_host_target):
    root = rec.checkout / "luna-rs/libs/external"
    manifest = tomllib.loads((root / "sysroot.toml").read_text())
    rows = []
    for entry in manifest["provider"]:
        stem = root / entry["path"]
        library, obj = stem.with_suffix(".llib"), stem.with_suffix(".obj")
        b = library.read_bytes()
        if b[:4] != b"LLIB":
            raise RuntimeError("Missing LLIB magic: " + str(library))
        fmt, compiler, mvir = struct.unpack_from("<HHH", b, 4)
        target = b[10:74].split(b"\0", 1)[0].decode()
        count, table = struct.unpack_from("<IQ", b, 110)
        sections = {}
        for i in range(count):
            _, kind, offset, size, version, compression, _, checksum = struct.unpack_from("<IIQQHB5sQ", b, table + i * 40)
            payload = b[offset:offset + size]
            if len(payload) != size or compression != 0:
                raise RuntimeError("Unsupported or truncated LLIB section")
            actual_checksum = int.from_bytes(hashlib.sha256(payload).digest()[:8], "little")
            if checksum != actual_checksum:
                raise RuntimeError("LLIB section checksum mismatch")
            sections[kind] = payload
        metadata = struct.unpack_from("<H", sections[16])[0]
        native = obj.read_bytes()
        embedded = sections[8]
        # Actual ELF class/data/type/machine, separately from advertised target.
        elf = native[:4] == b"\x7fELF" and native[4:6] == bytes([2, 1])
        e_type, e_machine = struct.unpack_from("<HH", native, 16)
        # Read the manifest target contract (bincode fixed-width string lengths).
        envelope, cursor = sections[10], 0
        strings = []
        for _ in range(9):
            length = struct.unpack_from("<Q", envelope, cursor)[0]
            cursor += 8
            strings.append(envelope[cursor:cursor + length].decode())
            cursor += length
        pointer_width = envelope[cursor]
        row = dict(name=entry["name"], path=entry["path"], format=fmt,
                   compiler=compiler, mvir=mvir, metadata=metadata, target=target,
                   manifest_target=strings[4], object_format=strings[7], abi=strings[8],
                   pointer_width=pointer_width, native_elf64_little_endian=elf,
                   elf_type=e_type, elf_machine=e_machine,
                   library_sha256=digest(library), object_sha256=digest(obj),
                   embedded_object_sha256=hashlib.sha256(embedded).hexdigest(),
                   embedded_equals_sidecar=embedded == native)
        # R5-FU-TARGET-01: the artifact records the canonical Luna release
        # target, not the host LLVM spelling. The host triple stays provenance.
        row["native_integrity_valid"] = (fmt, compiler, mvir, metadata) == (2, 25, 5, 9) and target == TARGET and strings[4] == target and pointer_width == 64 and elf and e_type == 1 and e_machine == 62 and native == embedded
        row["advertised_target_match"] = target == TARGET
        row["valid"] = row["native_integrity_valid"] and row["advertised_target_match"]
        rows.append(row)
    native_integrity = len(rows) == 49 and all(x["native_integrity_valid"] for x in rows)
    rec.native_target = llvm_host_target
    result = dict(tested_commit=PIN, providers=rows, count=len(rows),
                  llvm_host_target=llvm_host_target, advertised_target=TARGET,
                  native_integrity_valid=native_integrity,
                  scope="Artifact target identity is the canonical Luna release target; the LLVM host spelling is provenance only",
                  producer_cli_sha256=digest(rec.checkout / "luna-rs/target/debug/luna"),
                  status="PASS" if len(rows) == 49 and all(x["valid"] for x in rows) else "FAIL")
    save(rec.evidence / "sysroot-provenance.json", result)
    # Mandatory contract gate: every provider artifact must record the canonical
    # Luna target. The host LLVM spelling is recorded, never used as the
    # expected value.
    artifact_target = rows[0]["target"] if rows else None
    rec.run("TARGET_CONTRACT", [sys.executable, "-c",
            "import json,sys; artifact,advertised,host=sys.argv[1],sys.argv[2],sys.argv[3]; ok=artifact==advertised; print(json.dumps(dict(artifact_target=artifact,advertised_target=advertised,llvm_host_target=host,gate='MANDATORY TARGET_CONTRACT',status='PASS' if ok else 'FAIL',note='artifact records the canonical Luna target; host LLVM spelling is provenance only'))); sys.exit(0 if ok else 1)",
            artifact_target, TARGET, llvm_host_target], required=False)
    if not native_integrity:
        raise RuntimeError("49 provider/header/metadata/native integrity gate failed; dependent acquisitions stopped")


def normal(rec, provenance, *, probe_only=False, skip_workspace=False):
    root = rec.checkout
    for name, args in [("rustc", ["rustc", "-vV"]), ("llvm", ["llvm-config-18", "--version"]), ("llvm-host-target", ["llvm-config-18", "--host-target"]), ("cargo", ["cargo", "--version"])]:
        provenance[name] = rec.text(rec.run(name, args, cwd=root)).strip()
    save(rec.evidence / "provenance.json", provenance)
    build = rec.out / "runtime"
    rec.run("runtime-config", ["cmake", "-S", root / "runtime", "-B", build, "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_C_COMPILER=gcc", "-DCMAKE_CXX_COMPILER=g++"])
    rec.run("runtime-build", ["cmake", "--build", build, "--parallel"])
    rec.run("runtime-abi", ["ctest", "--test-dir", build, "-R", "^RuntimeAbiTests$", "--output-on-failure", "--no-tests=error", "-V"])
    rec.env.update(LUNA_RUNTIME_LIB=str(build / "libluna-runtime.a"), LUNA_SYSROOT=str(root / "luna-rs"))
    cargo = ["cargo", "build", "--locked", "--manifest-path", root / "luna-rs/Cargo.toml", "-p", "luna-cli"]
    rec.run("cli-debug", cargo, cwd=root)
    rec.run("cli-release", cargo + ["--release"], cwd=root)
    debug, release = root / "luna-rs/target/debug/luna", root / "luna-rs/target/release/luna"
    rec.env.update(LUNA_E1_RELEASE_CLI=str(release), LUNA_DOCS_CLI=str(debug))
    save(rec.evidence / "compiler-runtime-hashes.json", dict(tested_commit=PIN, debug_cli=digest(debug), release_cli=digest(release), runtime=digest(build / "libluna-runtime.a")))
    rec.run("sysroot", [debug, "build-sysroot", "--quiet"], cwd=root)
    providers(rec, provenance["llvm-host-target"])
    if probe_only:
        # Keep harness inputs and every generated probe artifact outside checkout.
        inputs = Path(__file__).resolve().parent / "comptime"
        external = rec.evidence / "comptime"
        shutil.copytree(inputs, external)
        rec.run("comptime-vm-boundary", [sys.executable, external / "run_qualification.py",
                "--checkout", root, "--sysroot", root / "luna-rs", "--debug", debug,
                "--release", release, "--runtime", build / "libluna-runtime.a",
                "--label", "ubuntu-acquisition1"], cwd=rec.out, required=False)
        rec.run("protected-tree-diff", ["git", "diff", "--exit-code", "HEAD", "--", "luna-rs", "runtime", "tests", "CMakeLists.txt"], cwd=root)
        return
    batches = [
        ("focused-a-ownership", "luna-cli", ["generic_drop_cli", "async_cleanup_cli", "async_provider_export_cli"]),
        ("focused-a-casts", "luna-cli", ["cast_contract_cli", "cast_poison_cli", "raw_slice_view_a5_cli", "raw_slice_provenance_cli"]),
        ("focused-c-calls", "luna-cli", ["named_arguments_cli", "default_arguments_cli", "default_omission_cli", "call_arity_cli"]),
        ("focused-c-comptime", "luna-cli", ["impl_header_metadata_cli", "projection_fallback_cli", "comptime_dependency_precision_cli", "comptime_loans_cli"]),
        ("focused-d-diagnostics", "luna-cli", ["diagnostic_conformance_cli", "expression_span_precision_cli", "comptime_diagnostic_cascade_cli", "compiler_trust_diagnostics_cli_parity"]),
        ("focused-d-providers", "luna-cli", ["provider_tiers_cli", "retained_contracts_cli", "alpha_modules_cli", "module_const_storage_cli", "fresh_artifact_classification_cli"]),
        ("arithmetic-grammar-harness", "luna-cli", ["checked_arithmetic_qualification_cli", "wrapping_arithmetic_cli", "v01_grammar_acceptance_tests", "test_temp_workspace_lifecycle_cli"]),
        ("format-unicode", "luna-cli", ["stdlib_format_cli_acceptance", "char_scalar_cli_parity", "char_operator_cli_parity"]),
        ("whole-file-io", "luna-driver", ["whole_file_io_v1_acceptance_tests"]),
    ]
    for name, package, tests in batches:
        args = ["cargo", "test", "--locked", "--no-fail-fast", "--manifest-path", root / "luna-rs/Cargo.toml", "-p", package]
        for target in tests:
            args += ["--test", target]
        rec.run(name, args + ["--", "--nocapture"], cwd=root, required=False)
    rec.run("registry-parity", ["cargo", "test", "--locked", "--manifest-path", root / "luna-rs/Cargo.toml", "-p", "luna-common", "registry_parity"], cwd=root, required=False)
    rec.run("docs-build", [sys.executable, "docs/tools/build_v01_docs.py"], cwd=root, required=False)
    rec.run("docs-validator", [sys.executable, "docs/tools/validate_v01_docs.py"], cwd=root, required=False)
    rec.run("docs-examples", [sys.executable, "docs/tools/check_v01_examples.py", "--gate", "--cli", debug, "--runtime-lib", build / "libluna-runtime.a", "--out", rec.evidence / "docs-examples"], cwd=root, required=False)
    quick = rec.evidence / "quickstart"
    quick.mkdir()
    # README's program.ln input: a minimal canonical Luna native entry point.
    source, exe = quick / "program.ln", quick / "program"
    source.write_text("fn main() -> i32 { return 0; }\n")
    quick_build = rec.run("quickstart-build", [debug, "build", source, "-o", exe], cwd=quick, required=False)
    if quick_build["exit"] == 0:
        rec.run("quickstart-native", [exe], cwd=quick, required=False)
        rec.run("quickstart-object", ["file", exe, quick / "program.obj"], cwd=quick, required=False)
        save(quick / "hashes.json", {p.name: digest(p) for p in quick.iterdir() if p.is_file()})
    if not skip_workspace:
        workspace = ["cargo", "test", "--locked", "--manifest-path", root / "luna-rs/Cargo.toml", "--workspace"]
        rec.run("workspace-triage", workspace + ["--no-fail-fast"], cwd=root, required=False)
        # This is a default-parallel exact-pin execution, not a full R5 verdict.
        rec.run("workspace-final", workspace, cwd=root, required=False)
    rec.run("protected-tree-diff", ["git", "diff", "--exit-code", "HEAD", "--", "luna-rs", "runtime", "tests", "CMakeLists.txt"], cwd=root)


def asan(rec):
    root, build = rec.checkout, rec.out / "runtime-asan"
    rec.env["ASAN_OPTIONS"] = "detect_leaks=1:halt_on_error=1"
    rec.run("asan-config", ["cmake", "-S", root / "runtime", "-B", build, "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Debug", "-DCMAKE_C_COMPILER=gcc", "-DCMAKE_CXX_COMPILER=g++", "-DCMAKE_C_FLAGS=-fsanitize=address -fno-omit-frame-pointer", "-DCMAKE_EXE_LINKER_FLAGS=-fsanitize=address"])
    rec.run("asan-build", ["cmake", "--build", build, "--parallel"])
    rec.run("asan-instrumentation", ["ldd", build / "abi_tests"])
    row = rec.run("asan-abi", ["ctest", "--test-dir", build, "-R", "^RuntimeAbiTests$", "--output-on-failure", "--no-tests=error", "-V"], required=False)
    text = rec.text(row)
    partial = "[PASS] test_internal_read_all_growth_and_error" in text
    no_report = "ERROR: AddressSanitizer" not in text and "ERROR: LeakSanitizer" not in text and "AddressSanitizer:DEADLYSIGNAL" not in text
    sanitized = "libasan.so" in rec.text(next(x for x in rec.rows if x["gate"] == "asan-instrumentation"))
    passed = row["exit"] == 0 and partial and no_report and sanitized
    save(rec.evidence / "runtime-asan-scope.json", dict(tested_commit=PIN, scope="C runtime and RuntimeAbiTests only; generated Luna/compiler/stdlib are not certified sanitized", asan_options=rec.env["ASAN_OPTIONS"], partial_read_error_cleanup_executed=partial, no_sanitizer_report=no_report, linked_libasan=sanitized, runtime_sha256=digest(build / "libluna-runtime.a"), abi_tests_sha256=digest(build / "abi_tests"), status="PASS" if passed else "FAIL"))
    if not passed:
        raise RuntimeError("Runtime ASan partial-read/instrumentation gate failed")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lane", choices=["normal", "runtime-asan", "comptime-vm"], required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--skip-workspace", action="store_true",
                        help="Skip the workspace suite: the GitHub-hosted 6h job cap cannot cover it; "
                             "the exact-pin workspace evidence is acquired in the Windows lane at the same commit")
    args = parser.parse_args()
    rec = Recorder(args.out.resolve())
    error = None
    try:
        provenance = rec.acquire()
        if args.lane in ("normal", "comptime-vm"):
            normal(rec, provenance, probe_only=args.lane == "comptime-vm", skip_workspace=args.skip_workspace)
        else:
            asan(rec)
    except Exception:
        error = traceback.format_exc()
        (rec.evidence / "failure.txt").write_text(error)
        print(error, flush=True)
    finally:
        if rec.checkout.exists():
            rec.run("final-status", ["git", "status", "--porcelain"], cwd=rec.checkout, required=False)
        rec.run("disk-final", ["df", "-h"], required=False)
        save(rec.evidence / "gates.json", rec.rows)
        failed = [r["gate"] for r in rec.rows if r["status"] != "PASS"]
        save(rec.evidence / "summary.json", dict(tested_commit=PIN, harness_commit=os.environ.get("GITHUB_SHA"), lane=args.lane, actual_native_target=rec.native_target, advertised_target=TARGET, scope="Descriptive execution on actual native target; no advertised-target equivalence or normalization" if rec.native_target and rec.native_target != TARGET else "Executed lane scope only", status="FAIL" if error or failed else "PASS_IN_EXECUTED_LANE_SCOPE", failure=error, failed_gates=failed, r5_verdict="NOT DETERMINED BY THIS LANE", release="BLOCKED PENDING COMPLETE R5 RECONCILIATION", gate_count=len(rec.rows)))
        save(rec.evidence / "evidence-hashes.json", {str(p.relative_to(rec.evidence)): digest(p) for p in sorted(rec.evidence.rglob("*")) if p.is_file() and p.name != "evidence-hashes.json"})
    return 1 if error or failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
