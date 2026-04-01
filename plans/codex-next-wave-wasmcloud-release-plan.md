# JAPL Next Wave Plan: wasmCloud Release Gate

Date: 2026-03-31

## Purpose

This plan takes the latest review as the baseline and defines the next execution wave needed to close the remaining gaps.

The primary requirement is:

> wasmCloud must become the actual primary distributed execution path for JAPL, not a documented intention or partial sidecar flow.

This plan also closes the remaining gaps in:

- typed process semantics
- stdlib runtime truthfulness
- runtime/distribution integration
- verification and release gating

## Important Framing

It is not technically honest to promise a literal "100% guarantee" up front.

What this plan does instead is stronger and more useful:

- removes fallback behavior that hides failure
- defines release-blocking gates
- requires end-to-end wasmCloud proof for process-using JAPL apps
- treats any unmet wasmCloud requirement as a ship blocker

If these gates pass, JAPL can honestly claim wasmCloud as the primary distributed path.

If they do not pass, the wave is not complete.

## Current Remaining Gaps

From the latest review, the unresolved gaps are:

1. `japl deploy` still falls back to local `serve`
2. wasmCloud provider is still sidecar-style, not native capability integration
3. process distribution semantics are not fully reachable through the deployed path
4. typed `Pid` exists, but typed process protocols do not
5. key stdlib modules still overstate runtime backing
6. verification still does not prove the primary distributed path strongly enough

## Non-Negotiable Release Rules

The next wave must follow these rules:

1. `japl deploy` must fail loudly if wasmCloud deployment cannot be completed
2. local `serve` must never be used as an implicit deploy fallback
3. at least one process-using JAPL application must run through the wasmCloud path in CI/release verification
4. docs may not claim a feature unless there is a corresponding automated verification path
5. stdlib modules may not present runtime-backed semantics they do not actually have

## Wave Structure

This next wave is split into four execution phases. Each phase has a hard exit gate.

---

## Phase 13: Make wasmCloud the Real Deploy Path

### Objective

Turn `japl deploy` into a true wasmCloud deployment command and remove the architecture ambiguity.

### Required changes

1. Remove implicit local fallback from `deploy`
   - no automatic fallback to `serve::serve`
   - if wasmCloud or provider setup fails, `deploy` exits non-zero

2. Make deploy artifacts canonical
   - component-target build output must be the deploy artifact
   - generated WADM manifest must match the actual runtime contract
   - provider requirements must be explicit, not implied

3. Make the provider path first-class in deploy orchestration
   - check NATS availability
   - check wasmCloud host availability
   - check provider availability and readiness
   - fail if the provider contract is not satisfiable

4. Split local and distributed modes cleanly
   - `run` and `serve` remain local/dev modes
   - `deploy` is the distributed mode
   - docs and CLI output must say this clearly

### Exit criteria

- `japl deploy` never silently becomes local `serve`
- failed wasmCloud setup causes a hard deploy failure
- a successful deploy actually exercises the wasmCloud/provider path
- README and runtime docs describe the shipped behavior accurately

### Agent execution

`Agent A: Deploy Command`
- Own [`japl/src/main.rs`](/Users/mlong/Documents/Development/japl/japl/src/main.rs)
- Remove deploy fallback behavior
- Add hard preflight checks and error surfaces

`Agent B: Deploy Contract`
- Own WADM generation and component/provider contract docs
- Align manifest generation with actual provider/runtime expectations

`Agent C: Runtime Mode Cleanup`
- Own [`README.md`](/Users/mlong/Documents/Development/japl/README.md) and [`docs/runtime-modes.md`](/Users/mlong/Documents/Development/japl/docs/runtime-modes.md)
- Ensure docs, CLI help, and mode semantics are consistent

---

## Phase 14: Close the wasmCloud Provider Gap

### Objective

Make the provider path functionally complete enough that deployed JAPL process apps are real, not partially simulated.

### Required changes

1. Eliminate placeholder semantics in the provider path
   - real `self-pid`
   - real `receive()` caller-context resolution
   - real logging interface support

2. Define one stable process ABI for deployed mode
   - process identity
   - message envelope
   - closure or entrypoint payload semantics
   - error and exit signaling

3. Decide and implement provider architecture
   - preferred: native wasmCloud capability provider via `wasmcloud-provider-sdk`
   - acceptable only if release-blocked and explicitly documented: sidecar mode with no fake "native" claim

4. Add bounded behavior and operational basics
   - mailbox size limits or backpressure
   - startup readiness checks
   - health and failure reporting

### Exit criteria

- provider path supports `spawn`, `send`, `receive`, and `self_pid` coherently
- logging works through the same deployed contract
- deployed process apps can exchange messages without hidden local-runtime assumptions
- provider mode is documented honestly as native or sidecar

### Agent execution

`Agent D: Provider Runtime`
- Own [`japl-provider/src/main.rs`](/Users/mlong/Documents/Development/japl/japl-provider/src/main.rs)
- Implement missing runtime semantics and operational guards

`Agent E: ABI and WIT`
- Own [`wit/japl-runtime/world.wit`](/Users/mlong/Documents/Development/japl/wit/japl-runtime/world.wit) and [`docs/message-abi.md`](/Users/mlong/Documents/Development/japl/docs/message-abi.md)
- Define the stable deployed-process ABI and caller-context model

`Agent F: wasmCloud Native Track`
- Own native-provider feasibility and implementation
- If native provider cannot ship this wave, produce an explicit blocker document and keep sidecar mode honest

---

## Phase 15: Finish Typed Process Semantics

### Objective

Move from nominal `Pid` typing to actual typed process contracts.

### Required changes

1. Remove `Pid`/`Int` compatibility shortcuts
   - `Pid` must be its own type, not a dressed-up integer

2. Introduce typed mailbox/protocol semantics
   - local send/receive
   - deployed send/receive
   - same conceptual model across both paths

3. Upgrade stdlib process APIs
   - [`stdlib/Process.japl`](/Users/mlong/Documents/Development/japl/stdlib/Process.japl)
   - [`stdlib/Supervisor.japl`](/Users/mlong/Documents/Development/japl/stdlib/Supervisor.japl)
   - [`stdlib/Registry.japl`](/Users/mlong/Documents/Development/japl/stdlib/Registry.japl)

4. Define what supervision means in deployed mode
   - restart semantics
   - ownership/supervisor boundaries
   - provider/runtime responsibilities

### Exit criteria

- `Pid` is no longer implicitly compatible with `Int`
- send/receive type rules reject protocol-invalid programs
- stdlib process modules describe only semantics that the runtime/provider actually provides
- supervision semantics are documented and tested

### Agent execution

`Agent G: Type System`
- Own [`japl/src/compiler/types.rs`](/Users/mlong/Documents/Development/japl/japl/src/compiler/types.rs) and [`japl/src/compiler/checker.rs`](/Users/mlong/Documents/Development/japl/japl/src/compiler/checker.rs)
- Remove compatibility shortcuts and add protocol typing

`Agent H: Runtime Message Model`
- Own [`japl/src/runtime/process.rs`](/Users/mlong/Documents/Development/japl/japl/src/runtime/process.rs) and related runtime message handling
- Align runtime envelopes with typed process expectations

`Agent I: Stdlib Process Surface`
- Own process-related stdlib modules
- Remove placeholder abstractions and align the public API with reality

---

## Phase 16: Close Stdlib and Verification Truth Gaps

### Objective

Finish the remaining stdlib/runtime honesty work and make wasmCloud verification the release gate.

### Required changes

1. Fix or downgrade remaining placeholder modules
   - [`stdlib/Config.japl`](/Users/mlong/Documents/Development/japl/stdlib/Config.japl)
   - [`stdlib/Tool.japl`](/Users/mlong/Documents/Development/japl/stdlib/Tool.japl)
   - [`stdlib/Supervisor.japl`](/Users/mlong/Documents/Development/japl/stdlib/Supervisor.japl)
   - AI wrappers if runtime guarantees are still partial

2. Replace compile-only critical checks with runtime verification
   - file APIs
   - network APIs
   - LLM/structured-output path
   - wasmCloud deploy path

3. Add one mandatory distributed proof suite
   - component build
   - deploy through wasmCloud
   - provider active
   - process messages exchanged
   - observable success/failure

4. Add release gate policy
   - release fails if wasmCloud proof suite fails
   - release fails if docs claim unsupported semantics
   - release fails if critical modules regress to compile-only coverage

### Exit criteria

- critical stdlib modules are either real or explicitly labeled limited
- `verify_all` includes real deployed-mode verification
- release gate depends on wasmCloud success, not local runtime success alone
- docs reflect shipped truth without aspirational language

### Agent execution

`Agent J: Stdlib Truth`
- Own remaining placeholder modules and their tests
- Either implement or explicitly narrow claims

`Agent K: Verification Gate`
- Own [`test/verify/verify_all.py`](/Users/mlong/Documents/Development/japl/test/verify/verify_all.py) and release-check wiring
- Add wasmCloud-primary proof cases

`Agent L: Docs and Feature Matrix`
- Own [`docs/feature-matrix.md`](/Users/mlong/Documents/Development/japl/docs/feature-matrix.md), [`docs/distribution-policy.md`](/Users/mlong/Documents/Development/japl/docs/distribution-policy.md), and README claims
- Ensure no unsupported release claims remain

---

## Suggested Execution Order

1. Phase 13 first
   - until `deploy` stops falling back to local serve, wasmCloud is not primary

2. Phase 14 second
   - until the provider path is coherent, deploy may succeed but semantics are still partial

3. Phase 15 third
   - typed process protocols should be built on the actual deployed runtime contract

4. Phase 16 last
   - verification and docs should lock the architecture after the runtime truth is in place

## Parallelization Plan

The work can be run as three coordinated tracks:

### Track 1: wasmCloud path

- Agent A
- Agent B
- Agent D
- Agent E
- Agent F

### Track 2: process semantics

- Agent G
- Agent H
- Agent I

### Track 3: truth and release gating

- Agent C
- Agent J
- Agent K
- Agent L

Track 3 should not finalize until Tracks 1 and 2 have landed enough behavior to verify.

## Hard Release Gate

The next wave is complete only if all of the following are true:

1. `japl deploy` fails closed and does not silently become local serve
2. a process-using JAPL app runs through the wasmCloud/provider path
3. the provider/runtime contract supports `spawn`, `send`, `receive`, and `self_pid` coherently
4. typed `Pid` no longer degrades to `Int`
5. critical stdlib modules are runtime-backed or honestly narrowed
6. `verify_all` or equivalent release verification proves the wasmCloud path
7. docs and CLI copy match the actual shipped behavior

If any one of these is false, the wave is not done.

## Immediate First Tasks

1. Remove deploy fallback to `serve::serve` and make deploy fail closed.
2. Decide whether this wave will ship a native wasmCloud provider or an explicitly supported sidecar provider.
3. Implement real `self_pid` and `receive()` caller-context handling in the provider path.
4. Add one deployable process app as the canonical distributed proof case.
5. Convert the release gate so wasmCloud success is mandatory.

## Bottom Line

The next wave should not try to broaden JAPL again.

It should do one thing:

> force the distributed story to become true in shipped behavior, with wasmCloud as the release-critical path, and then align typed processes, stdlib semantics, and verification around that reality.

