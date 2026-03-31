# JAPL Development Roadmap

> Consolidated from gap analysis, peer reviews, deep technical review, WASM backend plan, wasmCloud integration status, and AI-native design.
>
> Date: 2026-03-31 | Linear: japl-lang-org (JAP-1 through JAP-28)

---

## Strategic Position

**JAPL is a general-purpose language for distributed systems, built around typed processes, supervision, and WASM portability. AI is one especially natural application area.**

The current implementation compiles JAPL to WASM, runs it on a Rust runtime with real processes, supervision, distribution plumbing, and LLM effects. That is substantive progress. The gap is not vision — it is enforcement depth, runtime maturity, stdlib completeness, and a rigorous distributed model.

---

## Current State Summary

### What Works (verified)
- JAPL -> WAT -> WASM -> wasmtime pipeline (43 tests pass)
- Process spawning, supervision (OneForOne/AllForOne/RestForOne), mailbox messaging
- TCP-based distribution with cookie-authenticated handshake
- LLM host function in runtime
- WASM Component Model output (`--target component`)
- `wasmtime serve` serves HTTP from JAPL Components
- 12/12 verification apps pass on WASM
- 248/248 compiler tests pass

### What's Prototype-Grade
- Processes mapped to OS threads with 64MB stacks (not lightweight)
- Bump-only allocator, no memory reclamation
- 256-byte fixed closure payload copying
- Distribution uses raw bytes, no typed contracts
- Checker is too permissive (unknown identifiers → placeholders, effects opt-in)
- Stdlib is 17 files / 393 lines — demos, not real libraries
- Apps bypass stdlib, redeclare `foreign "japl" fn` directly

### Known Blockers
1. **wash 2.0.1 HTTP routing bug** — `wash dev` loads component but returns 404 for all requests (wasmtime serve works fine)
2. **Closure + HTTP handler WAT bug** — `call_indirect (type $closure_0)` undefined when both closures and HTTP handler coexist (JAP-19)
3. **Process + provider wiring** — japl-provider implements spawn/send/receive over NATS but can't be tested until Blocker 1 resolved

---

## Gap Scorecard

| Category | Built | Partial | Missing | Total |
|----------|-------|---------|---------|-------|
| Core Language | 11 | 2 | 0 | 13 |
| Type System | 2 | 4 | 5 | 11 |
| Concurrency | 7 | 0 | 4 | 11 |
| Supervision | 6 | 1 | 1 | 8 |
| Distribution | 0 | 0 | 14 | 14 |
| Tooling | 5 | 2 | 8 | 15 |
| Stdlib | 0 | 7 | 11 | 18 |
| Module System | 0 | 1 | 5 | 6 |
| **Total** | **31** | **17** | **48** | **96** |

---

## Wave 1: Foundation (JAP-1)

> Make the stdlib importable and provide core generic abstractions.

| Issue | Title | Priority | Agent Role |
|-------|-------|----------|------------|
| JAP-6 | Compiler: stdlib search path + import resolution | Urgent | Compiler engineer |
| JAP-7 | Generic Option\<T\>, Result\<T, E\>, List\<T\> | Urgent | Stdlib engineer |
| JAP-8 | String and Bytes APIs | Urgent | Stdlib + Rust engineer |
| JAP-9 | Module naming cleanup + visibility enforcement | High | Compiler + stdlib engineer |
| JAP-10 | Harden checker: strict failures, branch typing, effect enforcement | Urgent | Compiler engineer |

**Agent team: 4 agents**
- Agent 1 (Compiler): JAP-6 + JAP-9 — import resolution, visibility, qualified names
- Agent 2 (Type system): JAP-10 — checker hardening, strict failures, default effect enforcement
- Agent 3 (Stdlib): JAP-7 — generic Option/Result/List with combinators
- Agent 4 (Stdlib): JAP-8 — String APIs (split/join/trim/contains) + Bytes module

**Exit criteria:**
- `import String` resolves from any `.japl` file
- Option/Result/List are generic, not Int-only
- Unknown identifiers fail hard, effects enforced by default
- String has split/join/trim/contains/replace, Bytes has alloc/encode/decode

---

## Wave 2: Systems Core (JAP-2)

> Replace raw FFI shims with typed wrappers. Build remaining collections. Fix runtime foundations.

| Issue | Title | Priority | Agent Role |
|-------|-------|----------|------------|
| JAP-11 | Typed File + Env modules (Result-returning APIs) | High | Stdlib engineer |
| JAP-12 | Typed Time + Crypto modules (safe wrappers) | High | Stdlib + Rust engineer |
| JAP-13 | Generic Map\<K,V\>, Set\<T\>, and real Test module | High | Stdlib engineer |
| JAP-14 | Redesign scheduler: logical processes over worker pool | High | Runtime engineer |
| JAP-15 | Fix memory/ABI: remove 256-byte closure hack, real reclamation | High | Runtime engineer |

**Agent team: 4 agents**
- Agent 1 (Stdlib): JAP-11 — File.read → Result\<String, FileError\>, Env.get → Option\<String\>
- Agent 2 (Stdlib): JAP-12 + JAP-13 — Time/Crypto typed wrappers + Map/Set/Test
- Agent 3 (Runtime): JAP-14 — tokio-based logical process scheduler, mailbox limits
- Agent 4 (Runtime): JAP-15 — real closure ABI, memory reclamation, bounds validation

**Exit criteria:**
- File/Env/Time/Crypto return typed values and Result errors
- Map/Set are generic, Test actually fails on assertion mismatch
- 10,000+ processes run without thread explosion
- No fixed-size closure hacks, long-running services don't OOM

---

## Wave 3: Service Stack (JAP-3)

> Build the networking and serialization layer. Fix known compiler bugs. Harden distribution.

| Issue | Title | Priority | Agent Role |
|-------|-------|----------|------------|
| JAP-16 | JSON module: value AST, parser, encoder | High | Stdlib engineer |
| JAP-17 | HTTP module: Request/Response types, routing, client | High | Stdlib engineer |
| JAP-18 | Net module: typed TcpListener, TcpStream, socket addresses | High | Stdlib + Rust engineer |
| JAP-19 | Fix closure + HTTP handler WAT bug (Blocker 3) | Urgent | Compiler engineer |
| JAP-20 | Harden wire protocol + typed distributed contracts | High | Runtime engineer |

**Agent team: 4 agents**
- Agent 1 (Compiler): JAP-19 — fix emit_wat.rs closure/HTTP table bug
- Agent 2 (Stdlib): JAP-16 — JSON parser/encoder with JsonValue ADT
- Agent 3 (Stdlib): JAP-17 + JAP-18 — HTTP Request/Response + typed Net module
- Agent 4 (Runtime): JAP-20 — durable node IDs, versioned wire protocol, typed message schemas

**Exit criteria:**
- `Json.parse(str)` → Result\<JsonValue, ParseError\>
- HTTP server buildable with only stdlib imports
- TCP echo server with typed Net module
- Closure+HTTP apps compile without WAT errors
- Distributed KV store works across two machines with typed messages

---

## Wave 4: Process Stack (JAP-4)

> Build JAPL's core differentiator: typed, supervised, distributed process programming.

| Issue | Title | Priority | Agent Role |
|-------|-------|----------|------------|
| JAP-21 | Typed Pid\<T\> + mailbox utilities + timeout/select | High | Compiler + stdlib engineer |
| JAP-22 | Supervisor abstractions + monitors/links/registries | High | Stdlib engineer |
| JAP-23 | Codecs + process-safe serialization + retries/backoff | Medium | Stdlib engineer |
| JAP-24 | Structured logging + config/env parsing + diagnostics | Medium | Stdlib engineer |

**Agent team: 3 agents**
- Agent 1 (Compiler + Stdlib): JAP-21 — Pid\<T\> in type system, typed send/receive, timeout
- Agent 2 (Stdlib): JAP-22 — Supervisor in JAPL, monitors/links, Registry
- Agent 3 (Stdlib): JAP-23 + JAP-24 — Codecs, Retry, Timer, Log, Config, diagnostics

**Exit criteria:**
- `send(pid, WrongType)` → compile error
- Supervisor written in JAPL restarts crashed children
- `Registry.lookup("counter")` returns named process
- Structured JSON logging works
- ADT codec round-trips correctly with version tags

---

## Wave 5: AI Stack (JAP-5)

> Make "AI-native" a real language feature set, not branding.

| Issue | Title | Priority | Agent Role |
|-------|-------|----------|------------|
| JAP-25 | LLM effect + structured I/O (JSON schema from JAPL types) | Medium | Compiler + runtime engineer |
| JAP-26 | Tool contracts + Budget types (linear resource) | Medium | Compiler + stdlib engineer |
| JAP-27 | Replay + Provenance (deterministic testing for AI workflows) | Medium | Stdlib engineer |
| JAP-28 | Agent supervision demo: multi-agent app with full AI stack | Medium | Application engineer |

**Agent team: 3 agents**
- Agent 1 (Compiler + Runtime): JAP-25 — LLM effect in type system, structured I/O, JSON schema gen
- Agent 2 (Compiler + Stdlib): JAP-26 + JAP-27 — tool/agent keywords, Budget linearity, Replay/Provenance
- Agent 3 (Application): JAP-28 — multi-agent demo with supervision, budgets, tools, replay

**Exit criteria:**
- `llm("prompt")` tracked as LLM effect in type signature
- `llm_structured(prompt, type: Sentiment)` returns typed value
- Budget can't be duplicated (linear enforcement)
- `with_replay("fixture.replay")` enables deterministic AI tests
- Multi-agent demo: supervised agents with budgets, tools, and provenance logging

---

## Total Agent Team Across All Waves

| Wave | Agents | Focus |
|------|--------|-------|
| Wave 1 | 4 | Compiler (2) + Stdlib (2) |
| Wave 2 | 4 | Stdlib (2) + Runtime (2) |
| Wave 3 | 4 | Compiler (1) + Stdlib (2) + Runtime (1) |
| Wave 4 | 3 | Compiler+Stdlib (1) + Stdlib (2) |
| Wave 5 | 3 | Compiler+Runtime (1) + Compiler+Stdlib (1) + App (1) |
| **Total** | **18 agent-slots** | |

---

## Dependencies

```
Wave 1 (Foundation)
  └─ Wave 2 (Systems Core)
       └─ Wave 3 (Service Stack)
            └─ Wave 4 (Process Stack)
                 └─ Wave 5 (AI Stack)
```

Each wave depends on the previous. Within each wave, sub-tasks can run in parallel.

---

## wasmCloud Integration Status

### Working
- JAPL → WAT → core WASM → WASM Component pipeline
- Component exports `wasi:cli/run@0.2.3`
- `wasmtime serve` handles HTTP correctly
- japl-provider spawn/send/receive over NATS (self-test passes)
- HTTP adapter exports `wasi:http/incoming-handler@0.2.3`

### Blocked
- **wash 2.0.1 HTTP routing**: Component loaded but never registered as workload handler. `wasmtime serve` works as workaround.
- **Closure + HTTP WAT bug** (JAP-19): function table type conflict when closures and HTTP handler coexist
- **Provider wiring**: Can't test japl-provider integration until wash routing fixed

### Architecture Decision
The language defines semantics, backends implement them:
- `spawn`, `send`, `receive` are language primitives with stable semantics
- `--target local`: embedded wasmtime, OS threads
- `--target component`: WIT interface `japl:runtime/processes@0.1.0`, implemented by japl-provider over NATS
- Language code is identical regardless of target

---

## AI-Native Design (Wave 5 Detail)

### The Six Abstractions
1. **LLM Call as Effect** — tracked in type system, pure functions can't call LLMs
2. **Structured Model I/O** — JSON schema generated from JAPL ADTs, typed prompts and outputs
3. **Tool Contracts** — `tool` keyword, agents declare capabilities, enforced by type system
4. **Budget / Quota Types** — linear resource (can't duplicate), tracks tokens and cost
5. **Deterministic Replay** — record/replay nondeterministic decisions for testing
6. **Agent as Supervised Process** — agents ARE processes, supervision handles LLM failures

### What Makes JAPL Unique
No other language combines:
- LLM calls as a tracked effect
- Budget as a linear resource (compiler prevents overspending)
- Structured I/O from language types (no separate schema language)
- Agent supervision (LLM failures handled like Erlang handles telecom failures)
- Deterministic replay (test AI workflows without calling LLMs)

---

## Strategic Risks

1. **Overreach** — trying to be systems language + distributed runtime + AI language + safe language + ecosystem simultaneously. Mitigated by wave ordering: foundation first, AI last.
2. **Safety claims ahead of implementation** — ownership/linearity parsed but not enforced in codegen. Mitigated by JAP-10 (checker hardening) in Wave 1.
3. **wasmCloud dependency** — wash 2.0.1 has routing bugs. Mitigated by `wasmtime serve` as proven alternative.
4. **Stdlib not canonical** — apps bypass stdlib entirely. Mitigated by JAP-6 (import resolution) as Wave 1 priority.

---

## Archived Plans

Previous individual plan files have been consolidated into this document:
- `AI_NATIVE.md` → Wave 5 AI-Native Design section
- `GAP_ANALYSIS.md` → Gap Scorecard + full feature matrix
- `WASM_BACKEND.md` → wasmCloud Integration Status section
- `WASMCLOUD_INTEGRATION.md` → wasmCloud Integration Status section
- `WASMCLOUD_STATUS.md` → wasmCloud Integration Status section
- `wasmcloud-blockers.md` → Known Blockers section
- `codex-review.md` → Current State Summary
- `codex-ai-distributed-contender-review.md` → Strategic Position
- `codex-general-purpose-distributed-review.md` → Strategic Position
- `codex-deep-technical-review-roadmap.md` → Gap Scorecard + Wave details
