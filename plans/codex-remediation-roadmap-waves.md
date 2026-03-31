# JAPL Remediation Roadmap in Waves

Date: 2026-03-31

## Goal

Move JAPL from a capable prototype with uneven integration into a coherent platform where:

- the stdlib is the default programming surface
- runtime behavior matches language claims
- concurrency is operationally credible
- distributed features are either fully integrated or explicitly scoped
- AI-native APIs are tied to real semantics rather than wrappers and naming

This roadmap is prioritized around reducing architectural risk first, then expanding capability.

## Guiding Principles

1. Claims must follow implementation.
2. The stdlib must become the canonical API, not an optional demo layer.
3. Runtime and compiler invariants matter more than adding more surface APIs.
4. Distribution must be integrated into the actual execution path or removed from “working” claims.
5. AI-native features should be enforced through real runtime/type semantics, not just string helpers.

## Priority Order

1. Align shipped behavior with current claims
2. Make stdlib import/use real in production apps
3. Harden runtime process/mailbox/memory semantics
4. Decide and implement one actual distribution path
5. Strengthen typed process/distributed contracts
6. Lift AI abstractions from wrappers into enforceable semantics

---

## Wave 1: Truth, Surface, and Test Coverage

### Objective

Stop the project from drifting between “code that exists” and “features that ship.”

### Scope

- README and docs alignment
- stdlib import path validation
- verification coverage upgrades
- app migration off direct foreign declarations where possible

### Deliverables

1. Update [README.md](/Users/mlong/Documents/Development/japl/README.md)
   - remove references to `japl-runtime/` and old architecture
   - distinguish “working in shipped path” from “present in repo”
   - downgrade distribution/supervision wording where integration is incomplete

2. Expand [`test/verify/verify_all.py`](/Users/mlong/Documents/Development/japl/test/verify/verify_all.py)
   - add existing stdlib import tests:
     - [`test/programs/stdlib_import_test.japl`](/Users/mlong/Documents/Development/japl/test/programs/stdlib_import_test.japl)
     - [`test/programs/qualified_import_test.japl`](/Users/mlong/Documents/Development/japl/test/programs/qualified_import_test.japl)
     - [`test/programs/stdlib_option_test.japl`](/Users/mlong/Documents/Development/japl/test/programs/stdlib_option_test.japl)
   - add negative tests for import failures and unresolved stdlib modules

3. Migrate production apps to stdlib where already possible
   - replace direct string helpers in:
     - [`apps/http-kv/kv_server.japl`](/Users/mlong/Documents/Development/japl/apps/http-kv/kv_server.japl)
     - [`apps/genome/pipeline.japl`](/Users/mlong/Documents/Development/japl/apps/genome/pipeline.japl)
   - replace direct time/net wrappers where stdlib already exposes equivalents

4. Add a “repo truth table” document
   - shipped
   - present but not integrated
   - prototype
   - planned

### Exit Criteria

- docs describe the current system correctly
- verify suite includes stdlib import behavior
- at least 3 real apps use stdlib imports instead of direct `foreign "japl"` declarations

### Why This Wave Comes First

Without this, every later improvement will be difficult to assess because the public surface, app code, and runtime reality keep diverging.

---

## Wave 2: Make Stdlib the Real Programming Surface

### Objective

Turn the stdlib from “many modules exist” into “this is how JAPL programs are written.”

### Scope

- stabilize module APIs
- remove demo-first design
- replace raw FFI-facing APIs with typed wrappers
- make core collections and utilities credible

### Deliverables

1. Standardize stdlib module style
   - remove `main` functions from library modules where not needed
   - move demo behavior into `test/programs/` or examples
   - ensure every stdlib module is import-first, not run-first

2. Harden core modules
   - [Option.japl](/Users/mlong/Documents/Development/japl/stdlib/Option.japl)
   - [Result.japl](/Users/mlong/Documents/Development/japl/stdlib/Result.japl)
   - [List.japl](/Users/mlong/Documents/Development/japl/stdlib/List.japl)
   - [String.japl](/Users/mlong/Documents/Development/japl/stdlib/String.japl)
   - [Bytes.japl](/Users/mlong/Documents/Development/japl/stdlib/Bytes.japl)

   Required upgrades:
   - complete combinator set
   - consistent naming
   - better generic signatures where supported
   - explicit error behavior where not total

3. Replace toy collections
   - redesign [Map.japl](/Users/mlong/Documents/Development/japl/stdlib/Map.japl)
   - redesign [Set.japl](/Users/mlong/Documents/Development/japl/stdlib/Set.japl)

   Minimum target:
   - usable string-keyed map/set
   - non-demo APIs for insert/get/remove/contains/keys/values
   - no more “assoc list over Int only” as default collection story

4. Replace raw systems wrappers with safer library APIs
   - [File.japl](/Users/mlong/Documents/Development/japl/stdlib/File.japl)
   - [Net.japl](/Users/mlong/Documents/Development/japl/stdlib/Net.japl)
   - [Env.japl](/Users/mlong/Documents/Development/japl/stdlib/Env.japl)
   - [Time.japl](/Users/mlong/Documents/Development/japl/stdlib/Time.japl)
   - [Crypto.japl](/Users/mlong/Documents/Development/japl/stdlib/Crypto.japl)

   Replace pointer/length-oriented APIs with:
   - string-level wrappers
   - result-returning helpers
   - resource lifecycle helpers where possible

5. Finish the obviously incomplete modules
   - [Config.japl](/Users/mlong/Documents/Development/japl/stdlib/Config.japl) must stop being a stub
   - [Http.japl](/Users/mlong/Documents/Development/japl/stdlib/Http.japl) needs real request/response helpers
   - [Json.japl](/Users/mlong/Documents/Development/japl/stdlib/Json.japl) needs actual encode/decode capabilities

### Exit Criteria

- stdlib modules are primarily imported libraries, not executable demos
- apps stop redeclaring standard host functions directly
- `Map`, `Set`, `Config`, `Json`, and `Http` are no longer placeholder-grade

### Risk

If this wave is skipped, the language will keep accumulating modules without ever becoming coherent to use.

---

## Wave 3: Runtime Safety and Operational Hardening

### Objective

Make local concurrency and runtime behavior robust enough that stdlib and app work can rest on it safely.

### Scope

- scheduler semantics
- mailbox/backpressure
- process lifecycle
- host/guest ABI discipline
- memory behavior

### Deliverables

1. Remove “thread explosion by default” as an accepted runtime model
   - current process model in [scheduler.rs](/Users/mlong/Documents/Development/japl/japl/src/runtime/scheduler.rs) is still OS-thread-per-process
   - define next-step concurrency model:
     - worker pool + logical processes
     - or at minimum pooled execution for non-blocking processes

2. Formalize mailbox semantics
   - keep mailbox limits
   - add delivery semantics:
     - delivered
     - dropped
     - rejected
   - expose this in stdlib/process APIs rather than only scheduler internals

3. Harden process lifecycle and shutdown
   - remove `std::process::exit(0)` from normal scheduler control flow
   - make graceful shutdown a returned runtime state, not forced termination
   - support embeddable runtime use

4. Tighten host ABI and memory safety
   - audit all string/bytes/LLM/file/net host functions in [host.rs](/Users/mlong/Documents/Development/japl/japl/src/runtime/host.rs)
   - eliminate ad hoc assumptions and incomplete bounds checks
   - unify string/bytes layout handling
   - explicitly test allocation and out-of-bounds behavior

5. Add runtime-focused tests
   - mailbox saturation
   - process churn
   - graceful shutdown
   - LLM call fallback behavior
   - file/net failures

### Exit Criteria

- scheduler no longer exits the whole process as its primary shutdown mechanism
- mailbox semantics are explicit and test-covered
- runtime APIs can be used without relying on undefined behavior assumptions

### Why Before Distribution

A weak local runtime multiplied across nodes just creates a distributed failure generator.

---

## Wave 4: Decide and Implement One Real Distribution Path

### Objective

Resolve the current ambiguity between:

- custom TCP distribution in the runtime
- wasmCloud/provider/component direction
- `serve` as a deployment shortcut

JAPL needs one primary distribution story.

### Required Decision

Choose one of:

1. **Primary path: custom runtime distribution**
   - integrate [distribution.rs](/Users/mlong/Documents/Development/japl/japl/src/runtime/distribution.rs) into `japl run`
   - add CLI support for node identity, peer connect, cookie, listen port
   - make remote send/spawn visible and testable in the shipped binary

2. **Primary path: provider/component distribution**
   - move process semantics behind runtime/provider interfaces
   - stop describing the custom TCP layer as active product behavior

3. **Temporary path, explicitly called temporary**
   - local runtime only
   - distribution marked experimental/in-repo

### Deliverables if Custom Runtime Path Is Chosen

1. Wire `DistributionNode` into [`runtime::run`](/Users/mlong/Documents/Development/japl/japl/src/runtime/mod.rs)
2. Add CLI flags to [`main.rs`](/Users/mlong/Documents/Development/japl/japl/src/main.rs)
   - `--node-name`
   - `--listen-port`
   - `--peer`
   - `--cookie`
3. Replace the `spawn_remote` stub in [`serve.rs`](/Users/mlong/Documents/Development/japl/japl/src/serve.rs)
4. Add end-to-end multi-node verification
   - start two nodes
   - connect
   - remote spawn
   - remote message exchange
   - disconnect/reconnect behavior

### Deliverables if Provider Path Is Chosen

1. Make `japl deploy` use provider/component plumbing for real
2. Stop routing deployment through `serve::serve`
3. Ensure compiler backend emits the correct runtime/provider imports
4. Add one real deployed multi-process app proof

### Exit Criteria

- one actual distribution path is integrated into the user-facing execution path
- remote process/message semantics are no longer “present in repo only”

### Risk

If this wave is skipped, JAPL will continue to claim distribution while shipping only local concurrency as the real product.

---

## Wave 5: Typed Process and Distributed Contracts

### Objective

Move from “message-passing exists” to “message-passing is statically meaningful.”

### Scope

- typed process handles
- protocol-safe messaging
- remote contract validation
- supervisor/runtime surface alignment

### Deliverables

1. Introduce typed process abstractions
   - `Pid<T>`
   - typed mailbox/protocol expectations
   - typed reply patterns where possible

2. Strengthen checker handling in [checker.rs](/Users/mlong/Documents/Development/japl/japl/src/compiler/checker.rs)
   - `spawn`, `send`, `receive`, `self_pid` should stop collapsing to `Int`
   - protocol mismatches should become real checker errors

3. Upgrade process stdlib modules
   - [Process.japl](/Users/mlong/Documents/Development/japl/stdlib/Process.japl)
   - [Supervisor.japl](/Users/mlong/Documents/Development/japl/stdlib/Supervisor.japl)
   - [Registry.japl](/Users/mlong/Documents/Development/japl/stdlib/Registry.japl)
   - [Retry.japl](/Users/mlong/Documents/Development/japl/stdlib/Retry.japl)
   - [Codec.japl](/Users/mlong/Documents/Development/japl/stdlib/Codec.japl)

4. Tie runtime semantics to stdlib abstractions
   - mailbox size
   - spawn success/failure
   - remote delivery semantics
   - supervision semantics

### Exit Criteria

- concurrency/distribution types are no longer represented as raw integers and bytes at the language level
- stdlib process abstractions describe actual runtime semantics

### Why This Matters

Without this wave, JAPL remains actor-flavored syntax over an untyped transport core.

---

## Wave 6: AI Runtime Semantics, Not Just AI Modules

### Objective

Convert the AI stack from wrappers/data-types into a coherent system.

### Scope

- effect tracking
- structured output validation
- budgets
- replay
- provenance
- tool execution

### Deliverables

1. Make `llm_structured` real
   - validate returned structure
   - fail on invalid structured outputs
   - stop treating schema enforcement as prompt text only

2. Connect [Budget.japl](/Users/mlong/Documents/Development/japl/stdlib/Budget.japl) to runtime behavior
   - budget exhaustion should affect actual calls, not just library math

3. Make [Tool.japl](/Users/mlong/Documents/Development/japl/stdlib/Tool.japl) executable
   - real tool contracts
   - success/error propagation
   - integration with runtime/provider layer

4. Tie [Replay.japl](/Users/mlong/Documents/Development/japl/stdlib/Replay.japl) and [Provenance.japl](/Users/mlong/Documents/Development/japl/stdlib/Provenance.japl) to real runtime events
   - call logs
   - tool/LLM provenance
   - deterministic audit trail where possible

5. Extend checker effect enforcement
   - use `LLM`, `IO`, `Process`, `Fail` effects as meaningful boundaries
   - prevent AI features from becoming invisible side effects

### Exit Criteria

- AI modules are backed by runtime semantics
- LLM/tool/budget/replay/provenance features are not just pure-library facades

---

## Wave 7: Ecosystem and Packaging

### Objective

After core semantics are stable, make JAPL usable as a real language ecosystem.

### Scope

- package manager
- module publishing/distribution
- docs generation
- editor/LSP
- benchmark suites

### Deliverables

1. Package and dependency story
2. Public stdlib API documentation
3. Tooling support for imports/modules
4. Benchmark matrix
   - local process scalability
   - mailbox throughput
   - remote delivery
   - stdlib-heavy service apps

### Exit Criteria

- JAPL is no longer just a tightly coupled repo system
- users can write, import, and operate JAPL code without reading the compiler/runtime source

---

## Agent Teams and Parallel Execution Strategy

### Per-Wave Agent Breakdown

| Wave | Agents | Agent Roles | Worktree Isolation |
|------|--------|-------------|-------------------|
| **1** | A, B, C | A: test coverage, B: app migration + compiler fix, C: stdlib test harnesses | Yes (parallel) |
| **2** | D, E, F | D: core module hardening, E: Map/Set redesign, F: stub completion | Yes (parallel) |
| **3** | G, H, I | G: scheduler exit(0) removal, H: mailbox semantics, I: host ABI audit | Yes (parallel) |
| **4** | J, K | J: CLI distribution flags + DistributionNode wiring, K: multi-node tests | Yes (parallel) |
| **5** | L, M, N | L: Pid<T> in checker, M: process stdlib upgrade, N: runtime-stdlib tie | Sequential (L first) |
| **6** | O, P, Q | O: llm_structured validation, P: Budget+Tool runtime, Q: Replay+Provenance | Yes (parallel) |
| **7** | R, S | R: package manager, S: docs/LSP/benchmarks | Yes (parallel) |

### Parallel Tracks (which waves can run simultaneously)

```
Track A (Stdlib):  Wave 1 ──→ Wave 2 ──────────────────→ Wave 5 (stdlib half)
Track B (Runtime): ───────────────────→ Wave 3 ──→ Wave 4 → Wave 5 (runtime half)
Track C (AI):      ───────────────────────────────────────→ Wave 6
Track D (Ecosystem): ────────────────────────────────────────────→ Wave 7
```

**Safe parallelism:**
- Waves 2 + 3 can run in parallel (stdlib vs runtime, no overlap)
- Wave 4 depends on Wave 3 (runtime must be stable before distribution)
- Wave 5 depends on both Wave 2 and Wave 3 (typed processes touch compiler + stdlib + runtime)
- Wave 6 depends on Wave 5 (AI runtime needs typed process foundation)
- Wave 7 depends on Waves 1-6 (ecosystem builds on stable core)

### Review Gate Loop (per wave)

```
agents (parallel worktrees) → merge → cargo build → verify_all.py
    → Codex review gate → fix HIGH/MEDIUM findings → re-review
    → loop until no HIGH issues remain
    → mark Linear task Done → push → next wave
```

### Linear Tracking (japl-lang-org)

| Wave | Linear Issue | Status |
|------|-------------|--------|
| Wave 1 | JAP-29 | Done |
| Wave 2 | JAP-30 | Backlog |
| Wave 3 | JAP-31 | Backlog |
| Wave 4 | JAP-32 | Backlog |
| Wave 5 | JAP-33 | Backlog |
| Wave 6 | JAP-34 | Backlog |
| Wave 7 | JAP-35 | Backlog |

---

## Milestone View

### Milestone A: Coherent Local Language

Achieved when:

- docs are honest
- stdlib imports work and are used by real apps
- runtime shutdown/mailbox semantics are solid

This should be the immediate target.

### Milestone B: Credible Distributed Runtime

Achieved when:

- one distribution path is fully integrated
- multi-node tests pass through the shipped binary path
- process/distributed contracts are typed

This is the first point where “distributed by design” becomes a strong claim.

### Milestone C: Real AI-Native Platform

Achieved when:

- AI runtime effects are enforced
- tool and budget semantics are real
- replay and provenance are runtime-backed

This is the first point where “AI-native” becomes more than a library layer.

---

## Recommended Immediate Next Actions

1. Fix README and truth claims.
2. Add stdlib import tests to the verify suite.
3. Refactor 2-3 real apps to use stdlib imports.
4. Decide the primary distribution path.
5. Remove runtime `std::process::exit(0)` control flow.
6. Replace stub modules first:
   - `Config`
   - `Supervisor`
   - `Tool`
7. Upgrade `Map` and `Set` beyond int-only linked lists.

These are the highest-leverage tasks because they reduce mismatch between architecture, code, and user expectations.

## Final Assessment

JAPL now has enough substance that the right question is no longer “is there anything here?” The right question is “which parts become the product surface, and which parts stay prototype internals?”

This roadmap is designed to answer that by force:

- Wave 1 and Wave 2 make JAPL coherent to use.
- Wave 3 and Wave 4 make JAPL credible to run.
- Wave 5 and Wave 6 make JAPL credible to claim.

That is the shortest defensible path from current repo state to a language/runtime that can honestly claim to be concurrent, distributed, and AI-native by design.
