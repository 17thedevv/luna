<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current installation guidance. See the [versioned specification](spec/0.1/README.md) for language contracts.

# Install Luna 0.1.0-alpha.1

## Download

The current release is **0.1.0-alpha.1**, an alpha prerelease. Stable `0.1.0` has not been released.

## Windows installer (recommended)

[Download lunainstaller.exe](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/lunainstaller.exe) · [SHA-256](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/lunainstaller.exe.sha256)

Run setup, choose a directory and keep the user PATH option enabled. Setup downloads Luna, LLVM 18.1.8 and GNU/MinGW, checking each SHA-256 before extraction. No administrator account or Rust/Cargo is required. It needs Windows x86_64, Windows 10 1809 or newer, Internet, about 390 MB of downloads and 1.5 GB for the installed components. Reserve 3 GB of free space during setup.

The default directory is `%LOCALAPPDATA%\Programs\Luna`. Open **Luna Command Prompt** from Start or a new terminal, then run `luna --version`. Uninstall through Windows Settings → Apps → Luna. The setup executable is unsigned.

To check setup before running it, use `Get-FileHash .\lunainstaller.exe -Algorithm SHA256` in PowerShell and compare with the linked `.sha256` file.

Try the packaged example with the default installation:

```powershell
$package = "$env:LOCALAPPDATA\Programs\Luna\packages\luna-0.1.0-alpha.1-x86_64-pc-windows-gnu"
Copy-Item "$package\examples\quickstart.ln" .
luna run .\quickstart.ln --quiet
$LASTEXITCODE
```

If you chose a different directory, adjust `$package`. The example finishes with exit code `0`.

## Portable ZIPs

| Platform | Package | Executable |
|---|---|---|
| Windows x86_64 GNU | [Download Windows ZIP](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/luna-0.1.0-alpha.1-x86_64-pc-windows-gnu.zip) | `bin/luna.exe` |
| Ubuntu 24.04 x86_64 GNU | [Download Ubuntu ZIP](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/luna-0.1.0-alpha.1-x86_64-unknown-linux-gnu.zip) | `bin/luna` |

[SHA256SUMS](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/SHA256SUMS) · [Release notes](https://github.com/17thedevv/luna/releases/tag/v0.1.0-alpha.1)

The Windows ZIP contains the compiler executable, runtime and standard-library artifacts. Keep `bin`, `runtime` and `libs/external` together. Rust/Cargo is not required to use either package.

## Portable ZIP prerequisites

- Both platforms need **LLVM 18** and a native **x86_64 GNU C/C++ toolchain**.
- Windows needs `LLVM-C.dll` on `PATH` and the [Microsoft Visual C++ x64 runtime](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). The [official LLVM 18.1.8 distribution](https://github.com/llvm/llvm-project/releases/tag/llvmorg-18.1.8), `clang+llvm-18.1.8-x86_64-pc-windows-msvc.tar.xz`, includes the required tools and DLL. Set `LLVM_HOME` to the extracted directory containing `bin`.
- On Ubuntu 24.04, install `build-essential`, `llvm-18` and `unzip`.

LLVM and GNU tools are external prerequisites; they are not bundled. macOS, ARM and other targets do not have a qualified package in this release.

## Verify, extract and run

Download the ZIP for your platform and `SHA256SUMS` into the same writable directory.

### Windows PowerShell

Compare the hash printed below with the Windows entry in `SHA256SUMS`:

```powershell
Get-FileHash .\luna-0.1.0-alpha.1-x86_64-pc-windows-gnu.zip -Algorithm SHA256
Expand-Archive .\luna-0.1.0-alpha.1-x86_64-pc-windows-gnu.zip -DestinationPath .
$lunaPackage = (Resolve-Path .\luna-0.1.0-alpha.1-x86_64-pc-windows-gnu).Path

# Replace the LLVM/GNU directories with your installed locations.
$env:LLVM_HOME = 'C:\LLVM18'
$env:PATH = "$lunaPackage\bin;$env:LLVM_HOME\bin;C:\mingw64\bin;$env:PATH"

luna --version
luna build "$lunaPackage\examples\quickstart.ln" -o .\quickstart.exe
.\quickstart.exe
$LASTEXITCODE
luna build "$lunaPackage\examples\stdlib.ln" -o .\stdlib.exe
.\stdlib.exe
$LASTEXITCODE
```

### Ubuntu 24.04

```bash
sudo apt-get install build-essential llvm-18 unzip
sha256sum -c SHA256SUMS --ignore-missing
unzip luna-0.1.0-alpha.1-x86_64-unknown-linux-gnu.zip
export LUNA_HOME="$PWD/luna-0.1.0-alpha.1-x86_64-unknown-linux-gnu"
export LLVM_HOME=/usr/lib/llvm-18
export PATH="$LUNA_HOME/bin:$LLVM_HOME/bin:$PATH"
chmod +x "$LUNA_HOME/bin/luna"

luna --version
luna build "$LUNA_HOME/examples/quickstart.ln" -o quickstart
./quickstart
echo $?
luna build "$LUNA_HOME/examples/stdlib.ln" -o stdlib
./stdlib
echo $?
```

`luna --version` prints `luna 0.1.0`; the package manifest identifies the alpha distribution. Both examples finish with exit code `0` and no required console output.

Remove old `LUNA_SYSROOT` and `LUNA_RUNTIME_LIB` overrides before using the package. The compiler finds its runtime and 49 canonical providers relative to its executable. A missing `LLVM-C.dll` on Windows means the LLVM prerequisite is not on `PATH`.

## Your project

Copy `examples/quickstart.ln` into a writable project directory, edit it, and use `luna build` or `luna run`. The packaged `stdlib.ln` demonstrates an explicit `import <vec>;` and execution with the packaged library artifacts.

Continue with the [Language Tour](../luna-web/src/content/docs/language-tour/index.mdx) or [Standard Library](../luna-web/src/content/docs/stdlib/index.mdx). For source development, follow the [repository build instructions](https://github.com/17thedevv/luna#build-and-verify).
