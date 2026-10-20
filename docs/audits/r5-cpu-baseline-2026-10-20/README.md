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

Next: complete preflight and docs consistency; select an immutable implementation
pin; reacquire all eight lanes and both independent package smokes; verify all
archives; advance candidate only after closure. No main merge/tag/publication
before these gates pass. Keep failed package attempt1 as failure history.

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
