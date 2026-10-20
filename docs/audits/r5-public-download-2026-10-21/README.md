# Public download audit — 2026-10-21

Verdict: **COMPLETE** for public alpha downloads, installer qualification and
repository installation guidance; **PARTIAL** for deployment of the public website.
The released version is `0.1.0-alpha.1`; stable `0.1.0` is not published.
This audit does not certify every language feature.

Anonymous downloads of both platform ZIPs and SHA256SUMS return HTTP 200. Their
SHA-256 and byte sizes match GitHub. Both ZIP CRCs, every manifest payload hash,
internal checksum and AMD64 executable identity verify. Windows contains
`bin/luna.exe`; Ubuntu contains `bin/luna`. Each has 49 canonical providers,
105 payload files plus manifest/checksums.

The freshly downloaded Windows package passed seven local smoke steps, including
native quickstart/stdlib execution and the E1004 missing-provider rejection.
Ubuntu native execution is established by the existing hosted smoke, not rerun
on this Windows machine. Raw local evidence retains the harness's hosted-job
summary template; the wrapper explicitly records its actual local scope.

The audit found and corrected four delivery-documentation defects: the root
README and generated spec site still said release blocked; the website offered
source builds only; its prerequisite said LLVM 17+; and release notes displayed
filenames without direct download links. Download links now identify Windows,
Ubuntu, the executable location and the SHA256SUMS file. The binary-package
instructions use the packaged examples verified by the smoke harness.

Portable ZIP users install LLVM 18/GNU and the Windows Visual C++ runtime
separately. The new Windows online installer acquires its private prerequisites.
No original release ZIP, compiler implementation, tag target or protocol identity changes.

## Windows online installer

The published `lunainstaller.exe` is the exact executable qualified by
[hosted run 39474143676](https://github.com/17thedevv/luna/actions/runs/39474143676)
at installer source `1e22611c2f432e98baa57cbdbb66296935499799`.
It is 2,741,814 bytes, SHA-256
`adb15aa0c50a8d72bccaa447cfabd23c68010da547672c0db2c9a3184e7af5fd`.
The build is unsigned and uses Inno Setup 6.7.3, two static Rust launchers,
the original Luna ZIP, LLVM 18.1.8 and WinLibs GCC 14.2.0 / MinGW-w64 12 UCRT.
All downloads use fixed SHA-256 values embedded from the lock.

The public installer and five supplemental assets were downloaded anonymously
and checked against their API digests, sizes and the installer checksums.
Both CI artifact ZIP digests were independently recomputed before publication.
The build and smoke records identify the same executable and lock hashes.

The 17-step hosted smoke passed actual download/hash rejection, per-user install,
105 original payload hashes, 49 providers, private LLVM hashes, version, Start
Menu command prompt, native quickstart/stdlib builds and execution, `luna run`,
CLI failure status, reinstall without PATH duplication, uninstall, pre-existing
PATH preservation, unrelated PATH edits and user-file preservation. Installation
uses a Unicode path with spaces. Native tests remove host LLVM/GNU from PATH and
set conflicting development overrides. LLVM Clang/LLD links against the private
GNU/MinGW sysroot to avoid GCC driver path limitations.

Three unsuccessful hosted attempts are retained as superseded evidence, with
verified archive digests. The first two checked PATH before Inno's relocated
uninstaller finished. The third used WScript's ANSI-converted shortcut property;
the passing harness reads and executes the stored Unicode shortcut arguments.
None of those unsuccessful attempts is counted as acceptance.

[Installer verification](installer-verification.json) ·
[Raw passing smoke](installer-smoke-pass.zip) ·
[Superseded attempts](installer-attempts.json) ·
[Build record](installer-build.json) ·
[Installer source and build instructions](../../../distribution/windows/README.md).

## Public website deployment

The EN/VI website sources and local production build contain the download links
and current instructions. After PR #9 merged, the public Netlify site still
served the old source-only guide. The available Netlify dashboard session is
at its login page, with no configured local CLI credentials. Deployment is not
claimed complete. The [repository installation guide](../../installation.md)
is the current public fallback linked by the README and GitHub Release.

[Machine-readable verification](public-download-verification.json) ·
[Local smoke evidence](local-windows-smoke.zip) ·
[Original package verification](../r5-cpu-baseline-2026-10-20/evidence/package-cd9cd188/release-package-verification.json).
