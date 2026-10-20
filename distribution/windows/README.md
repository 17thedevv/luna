# Windows online installer

`lunainstaller.exe` installs the existing Luna **0.1.0-alpha.1** package plus
private LLVM 18.1.8 tools and a pinned WinLibs GNU/MinGW toolchain. It requires
Windows x86_64, Windows 10 1809 or newer, an Internet connection and about 1.5 GB
for the installed components. No administrator account or Rust installation is
needed by the user. The default destination is `%LOCALAPPDATA%\Programs\Luna`.

Every downloaded archive is checked against the SHA-256 embedded from
[`installer-lock.json`](installer-lock.json) before extraction. Downloads use
Inno Setup's HTTPS downloader. A corrupt or changed upstream archive stops
setup. Component versions are fixed; setup does not fetch a moving `latest`.

The original Luna compiler, runtime and 49 providers remain byte-identical to
the published Windows ZIP. A static launcher forwards arguments, console I/O
and exit status, and sets toolchain paths only in its child process. A small
linker driver invokes Clang/LLD 18 against the private MinGW GNU libraries.
This supports Unicode installation paths without relying on GCC's ANSI path
handling. The LLVM DLLs use app-local copies of their runtime dependencies.
Existing machine toolchains and global `LLVM_HOME`/`CC` are not changed.

The optional user PATH task adds only `{app}\bin`. A reinstall does not duplicate
it. Uninstall removes an entry only if setup added it, preserves a pre-existing
entry and unrelated PATH edits, and keeps files created by the user. Start Menu
provides a Luna Command Prompt for immediate use. Other terminals must be reopened.

## Build

On a Windows build machine with Rust 1.98.0 and Python 3.13:

```powershell
python distribution/windows/acquire_inno.py --out C:\luna-build\inno
python distribution/windows/build_installer.py --iscc C:\luna-build\inno\compiler\ISCC.exe --out C:\luna-build\dist
```

The build emits `lunainstaller.exe`, its `.sha256` and `build-summary.json`.
No signing certificate is configured; the setup executable is unsigned.
The LLVM payload is produced once from the locked official upstream archive:

```powershell
python distribution/windows/prepare_llvm_payload.py --archive C:\luna-build\clang+llvm-18.1.8-x86_64-pc-windows-msvc.tar.xz --out C:\luna-build\luna-llvm18-link-tools-windows-x86_64.zip
```

Its deterministic ZIP contains only the required LLVM executables, DLLs,
license and provenance. Keep the upstream archive and license hashes pinned.
`installer-lock.json` records the payload digest and individual file hashes.
Upstream license terms and Luna's existing `LICENSE-NOTICE.txt` still apply;
the installer introduces no project license grant.

## Verification

`.github/workflows/luna-windows-installer.yml` builds the actual production setup
and exercises it on a disposable Windows hosted profile. The smoke downloads
from production URLs, installs in a Unicode path with spaces, compiles and runs
the packaged examples with host LLVM/GNU removed from PATH and stale development
overrides set, checks original payload hashes, verifies CLI failure propagation,
reinstalls and uninstalls. It also tests pre-existing PATH ownership, unrelated
PATH edits, user-file preservation, and an actual tampered download rejection.

`smoke_installer.py` deliberately refuses registry tests outside GitHub Actions.
`build_installer.py --test-base-url http://127.0.0.1:PORT` creates a fixture-only
build for the negative downloader test; its summary marks `test_build: true`.
Publish only a production build whose corresponding smoke passed.
