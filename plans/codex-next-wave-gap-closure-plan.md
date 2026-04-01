# JAPL Next Wave Plan: Release Gate Closure

Date: 2026-03-31

## Purpose

This plan starts from the latest review, not from the claimed status summary.

The current state is:

- meaningful progress is real
- the deploy path is stricter than before
- docs are more honest than before
- the wasmCloud/provider architecture is clearer than before

But the release gate is still not fully closed.

This wave exists to close the remaining gaps and make the release checklist true in shipped behavior, not just in narrative.

## Core Remaining Gaps

The release gate is still open because:

1. wasmCloud verification is skippable
2. the provider is still sidecar-mode, not a native wasmCloud capability
3. `self_pid` is not truly runtime-derived
4. `Pid` still degrades to `Int`
5. some critical stdlib/runtime areas remain labeled limitations rather than completed features

There are also additional remaining gaps:

6. critical verification still allows compile-only coverage for `Net`
7. wasmCloud success currently proves component build more than full deployed process semantics
8. the deployed process ABI is still partially implicit and partly caller-driven
9. docs are more honest, but still describe an architecture that is not fully closed
10. the local runtime is still the most mature path, which means the distributed path is still trailing the dev path in robustness

## Non-Negotiable Rule For This Wave

This wave is complete only if the remaining release-gate failures are removed, not relabeled.

That means:

- no skippable distributed verification in the release gate
- no placeholder identity semantics
- no `Pid`/`Int` compatibility shortcut
- no critical feature being counted as done only because it is marked `LIMITED` or `SIMULATED`

## Execution Phases

---

## Phase 17: Make wasmCloud Verification Mandatory

### Objective

Turn wasmCloud verification from a best-effort readiness check into a release-blocking proof.

### Problems to solve

- `verify_all` currently prints `SKIP` for wasmCloud when the host is absent
- the current pass condition proves component build, not necessarily deployed process semantics
- the release verdict can still pass without a real wasmCloud execution proof

### Required changes

1. Make release verification fail if wasmCloud prerequisites are missing in release mode
   - no `SKIP` for release-gate execution
   - explicit `FAIL` when `wash`, host, or provider are unavailable

2. Add a real deployed proof test
   - compile a process-using JAPL app as a component
   - deploy through the wasmCloud path
   - verify process messaging succeeds through the provider path

3. Split developer convenience from release mode
   - local developers can still run a softer check if needed
   - release verification must be strict

### Exit criteria

- release verification fails if wasmCloud is not available
- the verification suite proves deployed process behavior, not only component build
- the release verdict depends on wasmCloud execution success

### Agent execution

`Agent A: Verification Gate`
- Own [`test/verify/verify_all.py`](/Users/mlong/Documents/Development/japl/test/verify/verify_all.py)
- Remove skippable wasmCloud release behavior

`Agent B: Distributed Proof Test`
- Own a real process-using proof app and its deploy verification path
- Must prove message send/receive through deployed mode

`Agent C: CI/Release Wiring`
- Own release scripts and docs describing strict vs local verification modes

---

## Phase 18: Close the Provider Architecture Gap

### Objective

Move the provider from “usable sidecar” to an architecture that satisfies the release claim.

### Problems to solve

- provider is still sidecar-mode
- docs still explicitly say native wasmCloud provider support is future work
- process identity and receive semantics are partly caller-supplied

### Required changes

1. Decide the shipped provider model for this release
   - preferred: native wasmCloud capability/provider implementation
   - fallback only if unavoidable: sidecar remains, but release/docs must stop claiming native-quality integration

2. Make process identity runtime-derived
   - `self_pid` must come from runtime/provider context
   - no caller-provided identity echo path

3. Make `receive()` context-derived as well
   - no hidden PID injection assumptions that live outside the real runtime contract

4. Finish operational behavior
   - health endpoint
   - mailbox limits/backpressure semantics
   - error reporting
   - startup readiness

### Exit criteria

- `self_pid` is runtime-derived
- `receive()` is coherent with the actual deployed call context
- provider mode is either truly native or explicitly documented as non-native with reduced claims
- the release docs match the shipped provider model exactly

### Agent execution

`Agent D: Provider Runtime`
- Own [`japl-provider/src/main.rs`](/Users/mlong/Documents/Development/japl/japl-provider/src/main.rs)
- Remove caller-echo identity behavior and close receive-context gaps

`Agent E: Provider Architecture`
- Own provider SDK migration or explicit sidecar-boundary decision
- Produce implementation or hard scope reduction

`Agent F: ABI/Contract`
- Own [`docs/message-abi.md`](/Users/mlong/Documents/Development/japl/docs/message-abi.md) and WIT/runtime contract alignment

---

## Phase 19: Finish Typed Process Semantics

### Objective

Close the remaining type-system loophole around process identities and process APIs.

### Problems to solve

- `Pid` still degrades to `Int`
- process operations are only partially typed
- runtime payload transport is still more dynamic than the language claims suggest

### Required changes

1. Remove `Pid`/`Int` compatibility in the checker
2. Audit process-related builtins and stdlib APIs for accidental `Int` bridging
3. Add negative tests proving arithmetic and generic `Int` APIs reject `Pid`
4. Clarify what is and is not typed at the mailbox boundary

### Exit criteria

- `Pid` is not compatible with `Int`
- checker rejects invalid `Pid` arithmetic and wrong-typed call sites
- process APIs do not silently backslide into integer semantics

### Agent execution

`Agent G: Type Checker`
- Own [`japl/src/compiler/checker.rs`](/Users/mlong/Documents/Development/japl/japl/src/compiler/checker.rs)
- Remove compatibility shortcuts and add regression tests

`Agent H: Compiler Surface Audit`
- Own process builtins, typing docs, and affected stdlib imports

---

## Phase 20: Convert “Honest Labels” Into Real Closure

### Objective

Stop counting critical limitations as completed merely because they are honestly labeled.

### Problems to solve

- `Config` is still `STUB`
- `Supervisor` is still `LIMITED`
- `Tool` is still `SIMULATED`
- other runtime/stdlib areas may still be effectively partial even if documented well

### Required changes

1. Classify critical modules by release importance
   - release-blocking modules must be implemented, not merely labeled
   - non-blocking modules may remain limited if clearly scoped

2. Finish or de-scope critical modules
   - [`stdlib/Config.japl`](/Users/mlong/Documents/Development/japl/stdlib/Config.japl)
   - [`stdlib/Supervisor.japl`](/Users/mlong/Documents/Development/japl/stdlib/Supervisor.japl)
   - [`stdlib/Tool.japl`](/Users/mlong/Documents/Development/japl/stdlib/Tool.japl)
   - any runtime helper that remains partial but is described as core

3. Strengthen runtime-backed coverage
   - replace compile-only critical checks where feasible
   - especially for `Net`

### Exit criteria

- critical modules are either implemented or removed from core claims
- no release-critical feature is counted complete solely because it is labeled
- compile-only coverage is not used for critical runtime modules

### Agent execution

`Agent I: Stdlib Closure`
- Own critical stdlib modules and their tests
- Implement or narrow release claims

`Agent J: Runtime Coverage`
- Own runtime-backed tests for currently compile-only or weakly-covered surfaces

---

## Phase 21: Final Truth Alignment

### Objective

Close the remaining mismatch between shipped behavior, docs, and release reporting.

### Problems to solve

- docs still contain future-tense/provider-native language in places
- release tables currently collapse “strictly proven” and “honestly limited” into the same green status
- the local runtime is still implicitly treated as the fallback confidence base

### Required changes

1. Rewrite release reporting categories
   - `PASS`
   - `LIMITED`
   - `EXPERIMENTAL`
   - `NOT SHIPPED`

2. Update docs to reflect exact shipped behavior
   - README
   - feature matrix
   - distribution policy
   - wasmCloud integration guide

3. Ensure release summaries do not claim more than the code and tests prove

### Exit criteria

- docs and release summaries match actual verification evidence
- no architecture claim depends on future work
- the shipped story is coherent across README, docs, tests, and CLI behavior

### Agent execution

`Agent K: Docs Truth`
- Own [`README.md`](/Users/mlong/Documents/Development/japl/README.md) and distributed docs

`Agent L: Release Summary`
- Own release-gate output and reporting language

---

## Priority Order

1. Phase 17: mandatory wasmCloud verification
2. Phase 18: provider/runtime identity and architecture closure
3. Phase 19: typed `Pid` closure
4. Phase 20: critical stdlib/runtime completion or de-scope
5. Phase 21: final documentation and release truth alignment

## Hard Closure Checklist

This wave is done only if all of the following are true:

1. wasmCloud verification is mandatory in release mode
2. deployed verification proves process behavior, not just component build
3. provider identity is runtime-derived, not caller-echoed
4. `Pid` is not compatible with `Int`
5. no critical module is counted complete only because it is honestly labeled
6. no critical module remains compile-only in release verification
7. docs and release summaries describe only shipped behavior

## Immediate First Tasks

1. Remove `SKIP` as an acceptable release outcome for wasmCloud verification.
2. Replace `self_pid` caller-echo behavior with runtime-derived identity.
3. Remove `Pid`/`Int` compatibility in the checker and add negative tests.
4. Reclassify `Config`, `Supervisor`, `Tool`, and `Net` as implement-or-de-scope items.
5. Rewrite release reporting to distinguish `PASS` from `LIMITED`.

## Bottom Line

The next wave should not add new surface area.

It should finish the truth gap:

> make the distributed wasmCloud path mandatory and provable, close the remaining process/type loopholes, and stop counting labeled limitations as completed features.

