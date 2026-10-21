<!-- luna-doc-role: guidance -->

# Project license and distribution notices

Maintainer decision, 2026-10-21: **GPL-3.0-only** for first-party Luna project
source, including the compiler, runtime, standard library and Lunar. The root
[LICENSE](../LICENSE) is the unmodified GNU GPL version 3 text from the
[GNU Project](https://www.gnu.org/licenses/gpl-3.0.txt), SHA-256
`3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986`.
The first-party grant and copyright notice are in [NOTICE](../NOTICE).
The license's example notice mentioning later versions is explanatory text;
the project's actual grant is version 3 only.

Luna grants no additional runtime/linking exception. The
[GCC Runtime Library Exception](https://www.gnu.org/licenses/gcc-exception-3.1-faq.en.html)
belongs to applicable upstream components and is not automatically an exception
for Luna's own runtime or standard library. Project documentation must not
promise unrestricted redistribution of linked Luna runtime/stdlib code under
an unrelated license.

Third-party Rust dependencies, LLVM/Clang/LLD, GNU/MinGW and redistributed
platform libraries retain their individual notices and terms. Applying the
project license does not replace those terms. The existing
[Windows notices](../distribution/windows/installer-notices.txt) describe the
external components; release preparation must retain the exact payload notices
and corresponding-source provenance, not just generic product names.

New binary distributions must carry LICENSE/NOTICE, identify the exact source
commit and provide corresponding project source, build scripts and component
source locations matching the distributed binaries. A final third-party
inventory and package/source verification remain part of the stable release
gate. This file records the chosen project grant, not a completed audit of every
redistributed component.

Published alpha archives predate this decision. Their byte content, checksums
and no-project-license-declared historical notices are preserved. Do not edit
their evidence to imply they carried this LICENSE when originally built.
