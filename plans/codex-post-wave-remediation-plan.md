# JAPL Post-Wave Remediation Plan

Date: 2026-03-31

## Purpose

This plan assumes the first seven remediation waves are complete enough to establish the current platform shape, and focuses only on the remaining gaps that still block JAPL from honestly claiming:

- a coherent stdlib surface
- a credible runtime
- wasmCloud as the primary distributed execution path
- typed process/distributed semantics
- AI-native runtime semantics beyond wrappers

This is not a full rebuild plan. It is a gap-closing plan.

## Current Honest State

What is now true:

- the unified `japl` binary exists
- the compiler/runtime stack is real
- the stdlib is broad
- the checker has improved
- local process concurrency works
- distribution infrastructure exists in the repo
- wasmCloud/NATS/provider work exists in parallel

What is still not true enough:

- wasmCloud is not yet the primary working distributed path
- `deploy` is not a real wasmCloud deployment path
- typed process support is still shallow
- some stdlib modules are still stubs/placeholders
- distribution is more implemented than integrated
- the docs still do not match the actual architecture

## Non-Negotiable Direction

The primary distributed architecture must be:

> JAPL source -> compiler -> WASM Component/backend artifacts -> wasmCloud host + JAPL provider(s) -> distributed execution

Not:

> JAPL source -> compile -> local embedded runtime + optional distribution sidecar

The local runtime remains essential for:

- fast iteration
- tests
- development
- fallback execution

But it should be treated as the **dev/runtime reference path**, not the strategic distributed product surface.

## Strategic Decisions

### Decision 1: wasmCloud is primary

Going forward:

- `japl deploy` must target wasmCloud as the first-class deployment path
- `serve` is a local convenience mode
- custom TCP distribution is secondary or compatibility infrastructure, not the main distributed identity

### Decision 2: stdlib must target the primary runtime model

That means:

- process/distribution stdlib APIs should reflect provider/component semantics where possible
- AI/tool/budget/replay/provenance should ultimately bind to runtime/provider behavior, not only local host wrappers

### Decision 3: typed processes must become typed protocols

`Pid` is not enough. JAPL needs:

- typed process handles
- typed mailbox expectations
- typed remote boundaries

---

## Remaining Gap Categories

## Gap A: wasmCloud is not yet the real deployment path

Symptoms:

- `japl deploy` still falls back to local `serve`
- distribution/provider/component code exists but is not the primary execution path
- wasmCloud remains adjacent rather than central

Root problem:

- architecture and CLI promise more than the shipped deploy path actually does

## Gap B: typed process semantics are still shallow

Symptoms:

- `Pid` exists, but `Pid` and `Int` are still treated as compatible
- message payloads are still effectively untyped at runtime boundaries
- no real typed mailbox protocol enforcement

Root problem:

- process syntax advanced faster than process semantics

## Gap C: stdlib still contains placeholder modules

Highest-risk modules:

- `Config`
- `Tool`
- `Supervisor`
- `LLM` structured semantics
- some systems modules with compile-only validation

Root problem:

- module count increased faster than runtime-backed semantics

## Gap D: local runtime is still more mature than distributed runtime

Symptoms:

- OS-thread-per-process still defines concurrency
- local runtime APIs are what apps actually depend on
- distributed path is not yet the default operating assumption

Root problem:

- dev runtime and product runtime have not been cleanly separated

## Gap E: docs and verification still understate the remaining risk

Symptoms:

- README still describes outdated architecture
- some tests are compile-only for critical modules
- distributed verification exists, but not yet as the primary shipped confidence mechanism

Root problem:

- messaging and testing have not been normalized around the new intended architecture

---

## Post-Wave Plan

## Phase 8: Make wasmCloud the Primary Distributed Path

### Objective

Turn wasmCloud from “parallel architecture work” into the primary distributed runtime path.

### Required outcomes

1. `japl deploy` must perform real wasmCloud-oriented deployment work
2. at least one JAPL app with processes must run through the wasmCloud/provider path
3. local `serve` must be clearly positioned as dev/local mode

### Work items

1. Replace `deploy -> serve::serve` fallback as the primary behavior
   - compile to component/backend artifact
   - provision or validate runtime dependencies
   - launch or connect to wasmCloud
   - use JAPL provider path where required

2. Define the canonical JAPL wasmCloud deployment contract
   - component format
   - provider interface(s)
   - wiring for process operations
   - logging/diagnostics expectations

3. Ensure one real distributed app works end to end
   - recommended target: message queue, scheduler, or KV workflow
   - not just HTTP-only request handling

4. Clarify what remains local-only
   - embedded runtime process model
   - `serve`
   - local-only host capabilities

### Exit criteria

- `japl deploy` uses the wasmCloud path as the intended primary mode
- a process-using JAPL app works in that path
- docs describe `run`/`serve` as local modes and `deploy` as the distributed production path

### Agent execution

`Agent A: Deploy Pipeline`
- Own `japl/src/main.rs`, deploy orchestration, component/provider invocation path
- Remove local-serve fallback as the default deploy implementation

`Agent B: wasmCloud Integration`
- Own provider wiring, component contract alignment, runtime/provider handshake semantics
- Ensure JAPL runtime/provider surface is actually usable in deployed mode

`Agent C: Distributed Proof App`
- Own one production-quality distributed demo app and its verification path
- Must prove actual distributed execution, not just startup

---

## Phase 9: Typed Process Protocols

### Objective

Finish the move from actor-like syntax to typed process semantics.

### Required outcomes

1. `Pid` is not treated as “basically Int”
2. send/receive are modeled through process-safe contracts
3. cross-node messaging follows the same typed model

### Work items

1. Tighten checker semantics
   - remove `Pid`/`Int` interchangeability except where deliberately bridged
   - make process APIs type-specific
   - make unknown protocol shapes fail earlier

2. Design typed mailbox protocol surface
   - `Pid<T>` or equivalent typed-handle model
   - typed send/receive contract
   - typed reply patterns where feasible

3. Unify local and distributed message expectations
   - same process/message abstractions across local runtime and wasmCloud path

4. Update stdlib process modules
   - `Process`
   - `Supervisor`
   - `Registry`
   - `Codec`
   - `Retry`

### Exit criteria

- process APIs are no longer effectively int-based
- stdlib process modules describe real typed semantics
- distributed and local process communication use the same conceptual contract

### Agent execution

`Agent D: Checker + Types`
- Own `Type::Pid`, checker compatibility rules, typed send/receive semantics

`Agent E: Process Stdlib`
- Own stdlib process-facing modules and align them with the checker/runtime contract

`Agent F: Message ABI / Protocol`
- Own runtime message representation and protocol-safe boundary rules

---

## Phase 10: Finish Placeholder Stdlib Modules

### Objective

Eliminate the remaining stdlib modules that still function mostly as placeholders or demos.

### Priority modules

1. `Config`
2. `Tool`
3. `Supervisor`
4. `LLM`
5. `File`
6. `Net`

### Work items

1. `Config`
   - implement actual environment/config retrieval path
   - remove “always defaults” behavior

2. `Tool`
   - define real execution semantics
   - integrate with runtime/provider layer
   - propagate real success/failure

3. `Supervisor`
   - decide what supervision means on the local runtime
   - decide what supervision means on wasmCloud/provider
   - stop exposing fake restart APIs if they cannot be honored

4. `LLM`
   - make `llm_structured` validation better than prefix checks
   - enforce structured output expectations at runtime

5. `File` and `Net`
   - move beyond compile-only confidence
   - ensure functional tests exist

### Exit criteria

- no stdlib module still requires a warning label to explain that it mostly returns defaults or formatted strings
- critical systems modules are functionally tested, not just compile-tested

### Agent execution

`Agent G: Config + Env`
- Own environment/config retrieval and associated runtime bridge

`Agent H: Tool + LLM`
- Own real tool execution semantics and structured output validation

`Agent I: Systems Modules`
- Own `File`, `Net`, and associated end-to-end tests

---

## Phase 11: Runtime Model Clarification

### Objective

Separate the local dev/runtime story from the distributed production/runtime story cleanly.

### Required outcomes

1. local runtime is explicitly the dev/reference runtime
2. wasmCloud/provider is explicitly the distributed runtime
3. custom TCP distribution is either:
   - kept as an experimental/reference backend, or
   - demoted and documented as non-primary

### Work items

1. Document runtime modes
   - `run`
   - `serve`
   - `deploy`

2. Clarify support matrix
   - local-only features
   - distributed-only features
   - common semantics

3. Reassess custom TCP distribution
   - keep for tests/research?
   - keep as optional backend?
   - freeze and de-emphasize?

4. Reassess OS-thread-per-process model
   - acceptable for local dev
   - not the long-term distributed/process scalability story

### Exit criteria

- there is no ambiguity about what JAPL’s primary distributed architecture is
- the runtime story is coherent enough for contributors and users

### Agent execution

`Agent J: Runtime Mode Cleanup`
- Own runtime mode matrix, CLI semantics, and documentation alignment

`Agent K: Distribution Backend Policy`
- Own decision and implementation plan for custom TCP distribution relative to wasmCloud

---

## Phase 12: Verification and Claims Closure

### Objective

Make the project’s verification and documentation match the actual architecture and guarantees.

### Required outcomes

1. distributed tests validate the primary distributed path
2. compile-only tests are minimized for critical modules
3. docs match the real shipped architecture

### Work items

1. Update README and architecture docs
   - remove stale `japl-runtime`/old architecture references
   - describe unified crate and real command behavior
   - describe wasmCloud as the primary distributed path

2. Rebuild verification tiers
   - unit tests
   - integration tests
   - distributed deployment tests
   - stdlib functional tests

3. Add release-gate checks
   - no placeholder/stub stdlib modules in “working” category
   - distributed smoke test for deploy path
   - structured LLM validation tests

### Exit criteria

- public docs match current implementation
- verification prioritizes the actual shipped architecture
- the project can make stronger claims without caveats

### Agent execution

`Agent L: Docs Alignment`
- Own README, architecture docs, and truth-table style feature matrix

`Agent M: Verification`
- Own verify suite, distributed integration tests, and release gates

---

## Recommended Execution Order

The right order is:

1. Phase 8: wasmCloud primary path
2. Phase 9: typed process protocols
3. Phase 10: finish placeholder stdlib modules
4. Phase 11: runtime model clarification
5. Phase 12: verification and claims closure

This ordering matters because:

- making wasmCloud primary determines what the stdlib and process APIs should target
- typed process semantics should be shaped by the actual distributed runtime strategy
- docs and verification should be updated after the architecture is finalized

## Suggested Team Structure

### Workstream 1: Distributed Product Path

- Agent A
- Agent B
- Agent C

Goal:
- ship a real wasmCloud-first deploy path

### Workstream 2: Typed Process Core

- Agent D
- Agent E
- Agent F

Goal:
- turn `Pid` into real process/protocol semantics

### Workstream 3: Stdlib Reality

- Agent G
- Agent H
- Agent I

Goal:
- remove the remaining placeholder modules

### Workstream 4: Closure

- Agent J
- Agent K
- Agent L
- Agent M

Goal:
- align runtime policy, docs, and verification

---

## Milestones

### Milestone 1: wasmCloud Is Real

Achieved when:

- `japl deploy` actually deploys through the wasmCloud/provider path
- one distributed process app is verified end to end

### Milestone 2: Typed Processes Are Real

Achieved when:

- `Pid` is not treated as interchangeable with `Int`
- process communication follows typed protocol expectations

### Milestone 3: Stdlib Is Honest

Achieved when:

- no major stdlib module still carries stub semantics
- apps can rely on stdlib instead of direct foreigns

### Milestone 4: Claims Are Honest

Achieved when:

- docs, CLI behavior, tests, and architecture all tell the same story

---

## Immediate Next Actions

1. Rework `japl deploy` so it no longer routes to `serve::serve` as the primary outcome.
2. Decide how provider/component process semantics map to JAPL process operations.
3. Remove the remaining placeholder behavior from `Config`, `Tool`, and `Supervisor`.
4. Tighten the checker so `Pid` is no longer just a decorated integer.
5. Rewrite README around the unified crate and wasmCloud-primary distributed architecture.

## Final Assessment

The first seven waves got JAPL to the point where the remaining work is mostly about integration honesty and architectural commitment.

The highest-value next move is not “more features.” It is:

> make wasmCloud the actual primary distributed path, then align typed processes, stdlib semantics, verification, and docs around that reality.

That is the shortest path from “impressive evolving prototype” to “coherent distributed language platform.”
