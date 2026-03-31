# JAPL Peer Review

## Original Findings (with remediation status)

### 1. No single canonical compiler/runtime story

**Original:** Multiple compiler tracks (Rust crates, TS compiler, self-hosted JAPL compiler), unclear which is authoritative. README pointed at Rust workspace but actual CLI was TypeScript.

**Status: FIXED.**
- Deleted: `compiler/crates/` (9 Rust crates), `compiler/japl/` (self-hosted attempt), `compiler/runtime/` (TS runtime stubs), `compiler/target/` (Rust artifacts)
- One compiler: `compiler/ts/` (TypeScript toolchain: lexer → parser → checker → IR → WAT codegen)
- One runtime: `japl-runtime/` (Rust + wasmtime: process scheduler, mailboxes, TCP distribution)
- One pipeline: `.japl → WAT → WASM → wasmtime/japl-runtime`

**Still needed:** README.md is stale — still describes old Rust workspace architecture. Must be rewritten.

### 2. Distribution claim materially ahead of implementation

**Original:** Wire format only sends i64 payloads. Not typed distributed processes.

**Status: PARTIALLY FIXED.**
- Runtime now serializes tagged variant structs from WASM memory (not just raw i64)
- `host.rs`: send() reads variant bytes from sender memory, receive() writes into receiver memory
- TCP layer exists with cookie-authenticated handshake, framed wire protocol
- KV store demo works with typed Put/Get/Size messages between processes

**Still needed:**
- Remote (cross-machine) message passing with typed values not yet tested end-to-end
- Distribution between two separate binaries with typed ADT messages not verified
- Spec should say "prototype distribution" not "native distribution"

### 3. Runtime safety weaker than spec claims

**Original:** Spec claims arbitrary-precision integers, region inference, deterministic resource release. Implementation has i64, placeholder GC, broken ResourceHandle::clone.

**Status: PARTIALLY FIXED.**
- Old TS runtime (with broken ResourceHandle) deleted entirely
- Current WASM backend: i64 integers (checked overflow in compiler), bump allocator in linear memory
- No arbitrary-precision integers — spec overclaims
- No region inference — spec overclaims
- No deterministic resource release — spec overclaims
- Linearity checked in compiler (--strict mode) but not enforced in WASM codegen

**Still needed:** Update spec to remove overclaims. Add honest feature matrix.

### 4. Standard library in two incompatible forms

**Original:** .japl stdlib files exist but tested stdlib is a Rust crate with erased Value types.

**Status: PARTIALLY FIXED.**
- Deleted: `compiler/runtime/japl-stdlib/` (old Rust stdlib crate)
- .japl stdlib files exist: Math, String, Option, Result, IO, Process, Test
- Math.japl compiles through WASM pipeline (verified)
- Other modules use features (generics, list patterns) that WAT codegen may not support yet

**Still needed:** Verify each stdlib module compiles through `japl build`. Fix or remove those that don't.

### 5. Tooling maturity behind

**Original:** 2 WASM CLI tests failing. No package manager, LSP, formatter, doc generator.

**Status: PARTIALLY FIXED.**
- 2 CLI tests FIXED — 248/248 tests now pass
- `japl build` → .wasm works
- `japl run` → compile + wasmtime works
- `japl check` → type check works
- `japl new` → scaffold works

**Still needed:** formatter (stub), REPL, LSP, package manager, doc generator. These are real gaps vs Go/Rust/Gleam.

### 6. "AI-native" not expressed as a language feature

**Original:** No first-class model/tool/provenance/cost/replay abstractions. Currently branding.

**Status: UNCHANGED.** This is still branding, not implementation.

**Decision needed:** Either add real AI abstractions (effect-tracked LLM calls, tool contract types, budget types, replay) or remove the "AI-native" claim.

## Current Architecture (post-cleanup)

```
compiler/ts/        THE compiler (lexer → parser → checker → IR → WAT)
japl-runtime/       THE runtime (Rust + wasmtime + processes + TCP)
stdlib/             Standard library (.japl files)
test/               Test programs (12 verified on WASM)
apps/               Applications (KV store with real processes)
spec/               Language specification
plans/              Development plans
papers/             Research papers (7 JAPL + MRA + Yoneda Constraint)
docs/               Website
```

No duplication. One compiler. One runtime. One pipeline.

## Updated Verification

- `npm test` in `compiler/ts/`: **248 passed, 0 failed** ✓
- `cargo build` in `japl-runtime/`: **compiles clean** ✓
- `japl run hello.japl`: **"Hello from JAPL!"** via wasmtime ✓
- `japl-runtime run processes.wasm`: **real threaded message passing** ✓
- `japl-runtime run kvstore.wasm`: **distributed KV store with typed messages** ✓
- 12/12 verification apps pass on WASM ✓

## Remaining Remediation Roadmap

### Phase 1: Fix the Story (CRITICAL)

| Action | Status | Effort |
|--------|--------|--------|
| Rewrite README for WASM pipeline | NOT DONE | 1 agent |
| Update spec: remove overclaims | NOT DONE | 1 agent |
| Honest feature matrix (working vs planned) | NOT DONE | same agent |
| Update website to match | NOT DONE | 1 agent |

### Phase 2: Strengthen Distribution

| Action | Status | Effort |
|--------|--------|--------|
| Typed message serialization across processes | DONE (local) | — |
| Typed messages across TCP (two binaries) | NOT TESTED | 1 agent |
| Distributed KV store across two machines | NOT DONE | 1 agent |
| Docker proof with typed messages | NOT DONE | 1 agent |

### Phase 3: Stdlib Through WASM

| Action | Status | Effort |
|--------|--------|--------|
| Math.japl compiles | DONE | — |
| String.japl compiles | NOT VERIFIED | 1 agent |
| Option.japl compiles | NOT VERIFIED | same |
| Result.japl compiles | NOT VERIFIED | same |
| IO.japl (WASI-based) | NOT DONE | same |
| Remove modules that can't compile | NOT DONE | same |

### Phase 4: Tooling Baseline

| Action | Status | Effort |
|--------|--------|--------|
| Formatter (basic) | STUB ONLY | 1 agent |
| REPL | NOT DONE | 1 agent |
| LSP (basic) | NOT DONE | 2 agents |
| Package manager | NOT DONE | 2 agents |

### Phase 5: AI-Native Decision

| Action | Status | Effort |
|--------|--------|--------|
| Design AI abstractions (effects, tools, budgets) | NOT DONE | research |
| OR: Remove "AI-native" claim | NOT DONE | 1 agent |

## Recommended Narrowed Identity

Per the original review's recommendation:

> "A typed actor language with immutable values, supervision, and explicit resource safety"

This is honest, defensible, and differentiated. Closer to Gleam/Erlang with a resource layer than "Rust + Go + Erlang combined."

## Bottom Line (Updated)

The fragmentation problem from finding #1 is fixed — one compiler, one runtime, one pipeline. The 2 broken CLI tests are fixed. The old dead code (22K+ lines) is deleted.

The remaining gaps are:
1. **Stale README/spec** — still describes old architecture, overclaims features
2. **Distribution** — works locally, not tested across machines with typed messages
3. **Stdlib** — files exist but most not verified through WASM pipeline
4. **Tooling** — minimal (build/run/check work, everything else is stub or missing)
5. **AI-native** — still branding, not implementation
