# CALL-ARGUMENTS-v1 — named arguments and per-call defaults

Maintainer-authorized 2026-10-04 addition to the 0.1 completion scope:
Python-style named arguments and default values. The maintainer chose per-call
default evaluation in the function's defining scope. Implementation is pending;
parser recognition of historical labels is not evidence of correct binding.

## Source surface and binding

```luna
fn make_rect(width: i32, height: i32 = 20) -> i32 {
    return width * height;
}
fn main() -> i32 {
    dec a = make_rect(10);
    dec b = make_rect(height=30, width=10);
    dec c = make_rect(10, height=30);
    return 0;
}
```

A named argument uses `identifier = expression` at the outer call-argument
boundary. Existing parsed `identifier: expression` labels are retained as an
equivalent spelling; they must no longer be silently ignored. This does not
change struct field initialization, assignment expressions or parameter types.
Ordinary positional calls remain valid.

Positional arguments bind by parameter order and must precede named arguments.
Named arguments bind by the declared signature name, independently of their
source order. Each parameter receives at most one value. Duplicate/unknown
labels, positional-plus-named duplicates, excess arguments and missing required
parameters reject with typed source diagnostics. `self` is not a defaultable
parameter; a method-call receiver remains implicit under existing rules.
Required parameters precede defaulted parameters in a declaration. No `*args`,
`**kwargs`, keyword-only separator or argument splatting is introduced.

The contract applies to direct functions, inherent methods and statically
resolved trait calls, including generic and imported declarations. Trait call
labels/defaults come from the trait signature, not arbitrary implementation
parameter spellings. Implementations cannot change those defaults. Calls through
a structural function-pointer type retain its full positional arity: that type
does not carry declaration names/defaults, and a named/omitted-argument call
without declaration signature information must reject rather than guess.
Function-pointer ABI and ordinary full-arity calling conventions do not change.

## Evaluation, ownership and hygiene

Evaluate receiver/callee under the existing call rules, then explicit arguments
exactly once in source order. Reordering their binding MUST NOT reorder side
effects or repeat evaluations. Evaluate only omitted defaults, once per call,
in parameter declaration order after explicit arguments have been evaluated.

Default expressions resolve names in the defining scope. Previously declared
parameters are available according to ordinary ownership/borrow rules; later
parameters are not. A default may construct a fresh owned value. It is not a
shared object cached at function-definition time. Supplied arguments do not
evaluate the default at all. Moves, temporary cleanup, unsafe/async/comptime
admission, escape/provenance and lifetime checks apply normally; defaults grant
no privileged escape from those rules.

After binding, type inference, generic bounds, coercion and lifetime relations
refer to declaration parameter positions. Named source order is presentation,
not an ABI ordinal. Extern unsafe boundaries remain required even if defaults
or named arguments are used.

## Provider/artifact preservation

Public parameter names and default contracts are part of the canonical callable
interface. Changing them must participate in the appropriate interface/body
fingerprints. Portable default bodies retain definition-site identity, types,
generic substitutions and required helpers without exposing private names to
ordinary consumer lookup or retaining session-local IDs.

Source-only and freshly built artifact-only consumers must bind, evaluate and
reject identically in independent processes. Default semantics must not depend
on the original source remaining available. Selected invalid/stale artifacts
still reject; this feature is not an artifact rebuild policy.

## Required acceptance

CLI tests cover reordered labels, mixed positional/named, nested/generic calls,
methods/qualified traits, side-effect ordering, default omission/override,
definition-site names and earlier parameter references, fresh non-Copy values,
move/borrow/escape negatives, unknown/duplicate/missing arguments and function
pointer controls. Include a relocated provider graph and source/fresh-artifact
parity. Defaults may not be limited silently to literals to pass a small matrix.
