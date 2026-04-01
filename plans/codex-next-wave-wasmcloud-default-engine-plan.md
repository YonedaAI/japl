# JAPL Next Wave Plan: wasmCloud as the Default Distributed Engine

Date: 2026-04-01

## Purpose

This plan starts from the latest honest review:

- typed `Pid` separation: closed
- critical `Net` compile-only gap: closed
- docs/feature-matrix honesty: improved, not fully closed
- provider identity model: improved, not fully closed
- wasmCloud release proof: not fully closed
- native wasmCloud provider claim: not closed

The main correction for the next wave is architectural, not cosmetic:

> JAPL must treat wasmCloud as the layer that fulfills the language claim of being distributed by default.

Not:

- local runtime plus optional deploy
- NATS request/reply tests standing in for distributed execution
- component build standing in for deployed behavior
- sidecar/provider scaffolding standing in for a real distributed engine

If JAPL wants the equivalent of “the Erlang VM is the distributed runtime,” then for JAPL the equivalent claim must become:

> wasmCloud is the distributed execution engine for JAPL programs, and the JAPL runtime/provider contract is what makes distributed-by-default true.

This wave exists to make that statement true in code, tests, CLI behavior, and docs.

## Non-Negotiable Constraint

This is the most important instruction for future implementation agents:

### No shortcuts

The wave is **not** complete if any proof relies on:

- local `japl run` when the claim is about distributed mode
- direct NATS CLI/provider probing without a JAPL app actually deployed through wasmCloud
- `--dry-run` manifests instead of real deployment
- “sidecar works” being treated as equivalent to “distributed engine is complete”
- docs that say “primary” while the shipped path still depends on manual sidecar assumptions

The architecture must be evaluated as a language/runtime system, not as a collection of individually passing scripts.

## Core Architecture Decision

### Canonical execution model

JAPL has three modes:

1. `japl run`
   - local development runtime
   - fast feedback
   - not the canonical distributed engine

2. `japl serve`
   - local HTTP/dev mode
   - convenience only
   - not the canonical distributed engine

3. `japl deploy`
   - canonical distributed runtime path
   - MUST use wasmCloud
   - MUST rely on the JAPL provider/runtime contract
   - MUST be the mode that satisfies “distributed by default”

### Required semantic interpretation

When JAPL claims:

- distributed by default
- concurrent by design
- supervised processes
- typed process messaging

the distributed meaning of those claims must be fulfilled by the wasmCloud path, not by the local runtime alone.

The local runtime may remain the reference/dev implementation, but it cannot be the thing that secretly carries the truth of the language while wasmCloud remains an approximation.

## Remaining Gaps To Close

### Gap 1: wasmCloud proof is still weaker than the claim

Current problem:

- the verification suite proves component build and some provider behavior
- it does not yet prove a full JAPL process app deployed and running through wasmCloud as the default distributed engine

Required closure:

- a JAPL app must be deployed through `japl deploy`
- it must run under wasmCloud
- it must use process messaging through the provider/runtime contract
- the proof must be automated and release-blocking

### Gap 2: provider identity is still not fully runtime-owned

Current problem:

- `self_pid` is improved but still partly session/body driven

Required closure:

- process identity must come from the deployed runtime/provider context
- no fallback to caller-echo semantics in the canonical path

### Gap 3: provider mode and product claim still diverge

Current problem:

- the provider is still a sidecar
- docs still describe native wasmCloud provider support as future work

Required closure:

Choose one of two honest outcomes for this wave:

1. **Best outcome**
   - ship a true wasmCloud-native provider/capability path
   - then the architecture claim becomes strong

2. **Minimum acceptable outcome**
   - sidecar remains
   - but docs, release gate, and product claim are rewritten to say:
     - wasmCloud is the host/orchestration layer
     - JAPL distributed execution currently depends on a JAPL sidecar provider
     - native provider integration is not yet shipped

What is not acceptable:

- keeping sidecar mode while speaking as if native provider integration is done

### Gap 4: release reporting still over-compresses proof levels

Current problem:

- “PROVEN” currently includes cases whose distributed semantics are not fully proven via deployed JAPL apps

Required closure:

- split “provider behavior proven” from “distributed engine proven”
- split “component build proven” from “deployed app proven”
- split “local process model proven” from “distributed process semantics proven”

### Gap 5: docs still let the local runtime carry too much of the language truth

Current problem:

- parts of the docs still implicitly teach JAPL through the local runtime
- some pages still use stronger phrases than the shipped distributed engine justifies

Required closure:

- docs must distinguish:
  - local reference runtime
  - distributed default engine
  - experimental or deferred runtime layers

## Wave Phases

---

## Phase 22: Prove wasmCloud as the Actual Distributed Engine

### Objective

Make the release gate prove the real language claim:

> JAPL distributed execution happens through wasmCloud, not just through local runtime or direct provider tests.

### Required work

1. Create a true end-to-end deployed proof
   - JAPL source
   - `japl build --target component`
   - `japl deploy`
   - wasmCloud host
   - JAPL provider/runtime contract
   - observable distributed process behavior

2. Use real proof apps
   - `kvstore`
   - `msgqueue`
   - at least one agent/process workload

3. Require external interaction
   - HTTP client or equivalent external caller
   - process response path must be observable from outside the runtime

4. Ensure this is release-blocking
   - no skip in release mode
   - no alternate success path

### Exit criteria

- at least one JAPL process app is proven deployed through wasmCloud end to end
- release mode fails if this path is not working
- distributed-by-default claim now rests on actual deployed behavior

### Agent execution

`Agent A: wasmCloud Proof Harness`
- Own end-to-end deploy verification
- Replace partial/provider-only proof with actual deployed-app proof

`Agent B: Proof Apps`
- Own distributed proof apps and external client scenarios
- Ensure the apps demonstrate real process behavior, not static request handling

`Agent C: Release Gate`
- Own release-check enforcement
- Ensure no alternative gate can pass without the distributed proof

---

## Phase 23: Make Runtime Identity Truly Provider-Derived

### Objective

Remove the remaining ambiguity around process identity in deployed mode.

### Required work

1. Remove canonical-path fallback to caller-supplied PID semantics
2. Bind `self_pid` to actual runtime/provider session/process state
3. Ensure `receive()` and related operations derive identity from the same context model
4. Document the deployed identity lifecycle
   - process creation
   - process lookup
   - mailbox ownership
   - provider restart implications

### Exit criteria

- `self_pid` in distributed mode is runtime-derived only
- provider identity semantics are documented and tested
- no canonical-path identity depends on caller-echo behavior

### Agent execution

`Agent D: Provider Identity`
- Own `self_pid`, session tracking, and process identity semantics

`Agent E: ABI Contract`
- Own WIT/message ABI docs and runtime identity invariants

---

## Phase 24: Resolve the Provider Architecture Claim

### Objective

End the ambiguity between “wasmCloud default engine” and “provider still sidecar.”

### Required work

1. Decide the release architecture explicitly

Option A:
- native wasmCloud provider/capability ships now

Option B:
- sidecar remains for this release
- but language/docs/release claims are rewritten precisely:
  - wasmCloud is the host/orchestration layer
  - JAPL distributed execution depends on the JAPL provider sidecar
  - native capability integration is deferred

2. Align deploy path, manifests, and docs with that decision

3. Add architecture validation to release docs
   - what is shipped
   - what is not shipped
   - what fulfills the distributed claim

### Exit criteria

- there is exactly one truthful story for the distributed engine
- no doc says or implies more than the shipped architecture provides
- the provider mode matches the architecture section in README and docs

### Agent execution

`Agent F: Provider Architecture Decision`
- Own final architecture decision and implementation/documentation consequences

`Agent G: Deploy Contract Alignment`
- Own manifests, deploy metadata, and runtime-mode descriptions

---

## Phase 25: Reclassify Proof Levels Across the Product

### Objective

Stop mixing local-runtime truth, provider-only truth, and distributed-engine truth under the same green label.

### Required work

1. Split proof categories in release output
   - local runtime proven
   - provider mechanics proven
   - deployed distributed engine proven
   - experimental
   - deferred

2. Update feature matrix and README accordingly
3. Audit public-facing pages for overstated language
   - README
   - docs
   - marketing pages

### Exit criteria

- “PROVEN” means fully proven in the relevant execution mode
- local and distributed proof are no longer conflated
- public docs match release reporting exactly

### Agent execution

`Agent H: Release Reporting`
- Own report categories and proof labels

`Agent I: Docs Audit`
- Own README, feature matrix, runtime docs, and public site copy

---

## Phase 26: Lock In the “Distributed by Default” Contract

### Objective

Encode the core architectural rule so future work cannot regress into local-runtime shortcuts.

### Required work

1. Add an architecture contract document
   - wasmCloud is the canonical distributed engine
   - local runtime is for dev/reference
   - distributed claims must be satisfied in deploy mode

2. Add a checklist for any future feature claiming distributed semantics
   - does it work in `japl deploy`?
   - is it proven through wasmCloud?
   - is it only local?
   - is it experimental?

3. Add review guardrails for future agents
   - no feature can be marked distributed-complete based only on local runtime evidence
   - no review can accept direct NATS probing as a substitute for deployed JAPL proof

### Exit criteria

- the repo contains an explicit architectural constraint document
- future reviews have a written standard to enforce
- “distributed by default” has a concrete operational meaning in the project

### Agent execution

`Agent J: Architecture Contract`
- Own the constraint document and review checklist

`Agent K: Review Policy`
- Own contributor/reviewer guidance for distributed claims

---

## Hard Closure Checklist

This wave is done only if all are true:

1. release mode proves a real JAPL app deployed and running through wasmCloud
2. that proof exercises process behavior, not just component build or raw provider requests
3. `self_pid` in deployed mode is runtime-derived only
4. the provider architecture claim is fully honest and singular
5. docs no longer imply that local-runtime proof is enough for distributed claims
6. release reporting distinguishes local/runtime/provider/deployed proof levels
7. the repo contains a written rule that wasmCloud is the canonical distributed engine for JAPL

## Immediate First Tasks

1. Replace provider-only deploy proof with deployed-app proof.
2. Remove canonical-path fallback for `self_pid`.
3. Choose and document the exact provider architecture for this release.
4. Rewrite release reporting so “distributed proven” means wasmCloud-deployed JAPL proof.
5. Add an architecture contract document that future agents must follow.

## Execution Board

This is the concrete execution board for the wave.

### Track A: Distributed Engine Proof

Owner:
- Agent A
- Agent B
- Agent C

Files:
- [`test/verify/verify_all.py`](/Users/mlong/Documents/Development/japl/test/verify/verify_all.py)
- [`test/deploy/deploy_proof.py`](/Users/mlong/Documents/Development/japl/test/deploy/deploy_proof.py)
- [`apps/kvstore/kvstore.japl`](/Users/mlong/Documents/Development/japl/apps/kvstore/kvstore.japl)
- [`apps/msgqueue/queue.japl`](/Users/mlong/Documents/Development/japl/apps/msgqueue/queue.japl)
- [`apps/agents/agents.japl`](/Users/mlong/Documents/Development/japl/apps/agents/agents.japl)

Acceptance tests:
- `python3 test/verify/verify_all.py --release`
- deploy proof must include a real JAPL app lifecycle
- release mode must fail if wasmCloud proof is unavailable

Definition of done:
- `verify_all --release` proves a deployed JAPL app, not just provider mechanics
- no `SKIP` outcome is accepted for distributed proof in release mode

### Track B: Provider Identity and ABI

Owner:
- Agent D
- Agent E

Files:
- [`japl-provider/src/main.rs`](/Users/mlong/Documents/Development/japl/japl-provider/src/main.rs)
- [`docs/message-abi.md`](/Users/mlong/Documents/Development/japl/docs/message-abi.md)
- [`docs/wasmcloud-integration.md`](/Users/mlong/Documents/Development/japl/docs/wasmcloud-integration.md)

Acceptance tests:
- provider identity tests show `self_pid` is runtime-derived
- no canonical deployed path accepts caller-echo PID fallback
- ABI docs match the actual provider/runtime implementation

Definition of done:
- distributed identity is owned by runtime/provider state
- docs contain no contradictory identity semantics

### Track C: Deploy Contract and Architecture

Owner:
- Agent F
- Agent G

Files:
- [`japl/src/main.rs`](/Users/mlong/Documents/Development/japl/japl/src/main.rs)
- [`docs/distribution-policy.md`](/Users/mlong/Documents/Development/japl/docs/distribution-policy.md)
- [`docs/provider-architecture-decision.md`](/Users/mlong/Documents/Development/japl/docs/provider-architecture-decision.md)
- [`docs/runtime-modes.md`](/Users/mlong/Documents/Development/japl/docs/runtime-modes.md)

Acceptance tests:
- `japl deploy` behavior matches the documented provider architecture
- manifest/deploy metadata match the shipped provider mode
- docs describe one coherent distributed-engine story

Definition of done:
- no contradiction remains between deploy behavior, provider mode, and docs

### Track D: Truth Labels and Public Claims

Owner:
- Agent H
- Agent I

Files:
- [`README.md`](/Users/mlong/Documents/Development/japl/README.md)
- [`docs/feature-matrix.md`](/Users/mlong/Documents/Development/japl/docs/feature-matrix.md)
- [`docs/index.html`](/Users/mlong/Documents/Development/japl/docs/index.html)
- [`docs/release-process.md`](/Users/mlong/Documents/Development/japl/docs/release-process.md)

Acceptance tests:
- every `PROVEN` claim maps to an automated proof in the matching execution mode
- public docs do not use stronger language than release docs
- local runtime claims and distributed runtime claims are clearly separated

Definition of done:
- public and internal documentation tell the same truth

### Track E: Architecture Contract and Review Policy

Owner:
- Agent J
- Agent K

Files:
- new doc: `docs/architecture-contract.md`
- new doc: `docs/review-checklist-distributed.md`

Acceptance tests:
- contract explicitly states wasmCloud is the canonical distributed engine
- review checklist explicitly rejects shortcut evidence
- future reviewers have a written standard for distributed claims

Definition of done:
- architectural enforcement exists in-repo, not just in chat history

## Review Checklist

This checklist should be copied into the new review-policy document and used for future distributed/runtime reviews.

### Distributed claim checklist

For any claim of:

- distributed by default
- distributed process semantics
- supervised distributed execution
- provider-backed runtime behavior

the reviewer must answer all of these:

1. Does the proof use a real JAPL app?
2. Does the proof go through `japl deploy`?
3. Does the app actually run through wasmCloud?
4. Does the proof exercise process behavior, not just startup or compilation?
5. Is the proof automated?
6. Would the release gate fail if this path were broken?

If any answer is `no`, the claim is not `PROVEN`.

### Shortcut rejection checklist

The reviewer must reject any proof based only on:

- `japl run`
- `japl serve`
- direct provider NATS requests
- `--dry-run` manifests
- component build only
- manual reasoning without automated verification

These can support development confidence, but they do not prove distributed-by-default semantics.

### Provider checklist

The reviewer must verify:

1. Is `self_pid` runtime-derived?
2. Is `receive()` bound to real deployed identity/context?
3. Is provider mode documented honestly as native or sidecar?
4. Does the deploy path match that exact provider mode?

If not, the provider claim is still partial.

### Documentation checklist

The reviewer must verify:

1. README matches release docs
2. feature matrix matches tests
3. public site copy does not overstate runtime guarantees
4. `PROVEN`, `LIMITED`, and `EXPERIMENTAL` are used consistently

## Best-Practice Rule For Future Agents

Future agent execution should follow this rule:

> When evaluating JAPL distributed semantics, always treat wasmCloud deploy mode as the canonical execution mode. Local runtime evidence is useful only for local-runtime claims.

Corollary:

> Never close a distributed-systems issue using only local-runtime or direct-provider evidence.

## On Plugins and Enforcement

There is no plugin in this environment that can automatically enforce this architectural constraint.

The practical enforcement mechanism is:

- a written architecture contract in the repo
- a release gate that fails without wasmCloud proof
- reviewer guidance that forbids shortcut evidence

If desired, a future repo-local skill or policy document could be added for reviewer workflows, but the right enforcement today is through code, tests, docs, and release checks.

## Bottom Line

The next wave is not about adding features.

It is about forcing one architectural truth to become real:

> For JAPL, wasmCloud must be the thing that makes “distributed by default” true, the way a VM/runtime makes that true in a language like Erlang.

Until the repo proves that through deployed JAPL apps, the distributed claim is still only partially closed.
