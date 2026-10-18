"""Extract LLVM 18 tools/headers/import library from the official Windows archive."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import tarfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    # Keep native tools and their DLLs, the C headers and the MSVC import
    # library. The unused static component libraries need not fill the runner.
    with tarfile.open(args.archive, mode="r|xz") as archive:
        for member in archive:
            parts = PurePosixPath(member.name).parts
            if not member.isfile() or len(parts) < 3:
                continue
            relative = Path(*parts[1:])
            selected = parts[1] in ("bin", "include") or relative.as_posix() == "lib/LLVM-C.lib"
            if not selected:
                continue
            target = (args.out / relative).resolve()
            if not target.is_relative_to(args.out.resolve()):
                raise RuntimeError("Archive member escapes toolchain directory")
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.extractfile(member) as source, target.open("wb") as destination:
                shutil.copyfileobj(source, destination)
    for name in ("bin/llvm-config.exe", "bin/llc.exe", "bin/LLVM-C.dll", "lib/LLVM-C.lib", "include/llvm-c/Core.h"):
        if not (args.out / name).is_file():
            raise RuntimeError("Missing LLVM development component: " + name)
    digest = hashlib.file_digest(args.archive.open("rb"), "sha256").hexdigest()
    (args.out / "acquisition.json").write_text(json.dumps(dict(
        source="https://github.com/llvm/llvm-project/releases/tag/llvmorg-18.1.8",
        asset="clang+llvm-18.1.8-x86_64-pc-windows-msvc.tar.xz",
        archive_sha256=digest, digest_scope="Observed acquisition digest; upstream asset has no published API digest",
        extraction="bin, include and lib/LLVM-C.lib"), indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
