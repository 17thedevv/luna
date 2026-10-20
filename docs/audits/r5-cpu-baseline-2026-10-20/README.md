# R5 release-package CPU baseline repair

Date: 2026-10-20. **Release blocked; replacement qualification required.**
Candidate remains `8525067f`; main remains `401a95a8`; PR #7 is draft.
Maintainer authorization for `v0.1.0-alpha.1` remains conditional on passing
all qualification and clean Windows/Ubuntu package checks.

## Confirmed finding

[Package workflow 39383661819](https://github.com/17thedevv/luna/actions/runs/39383661819)
built both packages from `b515783782408b2279fc4c534c125182bb5a41cf`, whose
implementation trees equal the eight-lane pin `8525067f`.
Independent Ubuntu package smoke passed. Windows version checking passed,
then the public quickstart failed before native emission:

```text
Strict Rejection ... __lang_drop ... target CPU mismatch: expected znver3, got generic
error[E1004]: required component __lang_drop could not be bootstrapped
```

[Original smoke ZIPs, checksums and finding](evidence/finding.json) are preserved.
This is a release blocker, not a timeout or an acceptable native trap.
The eight `8525067f` qualification passes retain their executed scope but do
not certify portable distribution. No PASS transfers to the replacement pin.

## Contract and owning-stage cause

The [target identity contract](../../spec/0.1/modules.md) binds triple,
CPU/features, object format, ABI/data layout, width and endianness, with strict
CPU/features equality. Rejection is correct; accepting incompatible identities
would violate that contract.

`TargetConfig::default` canonicalized the supported triple but selected
`get_host_cpu_name` and `get_host_cpu_features`. Package build and consumer
machines therefore constructed different configured identities. A minimal
backend regression fails before the repair on the local `alderlake` host:
expected `generic`, observed `alderlake` (exit101). The public artifact-target
fixture uses an ordinary user provider, not a special stdlib container.

## Repair and validation plan

Select CPU `generic` with no optional host features for the two existing
advertised x86_64 GNU release targets. Unsupported target defaults remain
host-derived. Every strict target, object and integrity validator is retained;
no source fallback or implicit rebuild is introduced.

The CLI acceptance test asserts the configured baseline, relocated source and
artifact native execution, and all target mismatch rejections. Its feature
mutation now edits the serialized manifest structurally: an empty feature
string cannot be found with a zero-length byte-window search. CPU/features
negative controls must still reject E6001 for the original reasons.

Backend preflight after the repair: four target-contract tests PASS, including
baseline identity, invalid target/nonobject rejection, object/layout descriptors
and rejection of unsupported 32-bit lowering. Public CLI preflight PASS: relocated source/artifact native controls and all
22 check/build mismatch rejections (including CPU/features) retain E6001. Local preflight reuses the native runtime and is not
an exact-pin qualification claim.

Implementation pin selected: `b5cb77c3286f583c43c41d0e56f4136903ebb842`.
Preflight and docs consistency PASS. Qualification harness and package workflow
now select that pin, including generic CPU/empty-features checks on all49 bundles
and declaration-location additions in both focused diagnostic batches.
Reacquire all eight lanes and both independent package smokes; verify archives;
advance candidate only after closure. No main merge/tag/publication
before these gates pass. Keep failed package attempt1 as failure history.

## Replacement acquisition progress

The [current state](qualification-state.json) records the replacement runs.
ASan and comptime/VM Ubuntu archives are independently verified at `b5cb77c3`:
1025 file hashes, 47 compressed log hashes and 47 raw log hashes, with no mismatch.
The remaining six qualification lanes are pending; no eight-lane verdict is made.
The [lane ledger](evidence/pin-b5cb77c3/lane-state.json) lists every required
lane. Independent acquisition checks use `verify_ci_artifacts.py` with
`--require-generic-cpu`; [CPU/features mutation controls](evidence/pin-b5cb77c3/cpu-baseline-verifier-controls.json)
retain coherent ZIP/inventory digests and still reject the wrong baseline.

[Package run 39385567878](https://github.com/17thedevv/luna/actions/runs/39385567878)
builds the exact immutable source `cd9cd1884d2e49c0f03979597ca73e92043b7fe7`,
whose implementation trees match `b5cb77c3`. Both clean hosted consumer smokes
PASS. [Independent package verification](evidence/package-cd9cd188/release-package-verification.json)
reconciles all six original CI ZIP digests, sizes and CRCs, build stream hashes,
two distribution payloads with 105 files and 49 canonical providers each, and
seven consumer steps per target. Missing-provider controls still reject E1004.
[Four verifier controls](evidence/package-cd9cd188/verifier-controls.json)
reject wrong source, wrong qualified pin, wrong outer digest and changed smoke
stdout even after repairing its outer ZIP digest.

The exact Windows ZIP also passes a
[supplemental local consumer](evidence/package-cd9cd188/supplemental-local-windows-smoke.json)
on `alderlake`, using only its packaged runtime/providers after relocation.
Its CI builder reported `znver3`. This local check is separate from the two
required hosted smokes and does not replace qualification. The prior failed
Windows package remains historical failure evidence.

Compiler Change

    Capability: Default artifacts for an advertised target have one CPU baseline.
    Why stdlib exposed it: A clean package could not bootstrap its language provider.
    Why it is generic: Target configuration applies to every provider and user program.
    User-defined provider: api::answer in artifact_target fixtures.
    Tests: Backend baseline/negative descriptors; public relocated provider and strict
        CPU/features/object/ABI/integrity controls; separate-machine package smoke.
    New intrinsic/lang_item?: NO. Versions remain compiler25/metadata9/MVIR5/format2.
    Stdlib-specific branch?: NO.

No serialized schema or language syntax changes. Existing target contracts
already encode CPU/features, so incompatible old artifacts continue to reject.
