# JAPL WASM Backend Plan

> JAPL → WAT → WASM → wasmtime (Rust runtime)
> No C. No JavaScript. No TypeScript output. WASM only.

## Architecture

```
JAPL Source (.japl)
       │
JAPL Compiler (TypeScript toolchain — parse, check, IR)
       │
   emit_wat.ts (IR → WAT text)
       │
   wat2wasm (WAT → WASM binary)
       │
JAPL Runtime (Rust, ~2000 lines on wasmtime)
  ├── Process scheduler (green threads via tokio)
  ├── Mailbox router (per-process message queues)
  ├── Supervisor (restart crashed WASM instances)
  ├── Host functions (spawn, send, receive, println, file I/O)
  ├── WASI capabilities (fs, net, clock)
  └── Distribution (TCP between runtime instances)
       │
   Native execution (wasmtime JIT → machine code)
```

## Cleanup: Remove Old Backends

### Files to DELETE:
```
compiler/ts/src/codegen/emit.ts           ← TypeScript emitter (REMOVE)
compiler/ts/src/codegen/emit_c.ts         ← C emitter (REMOVE)
compiler/ts/src/codegen/codegen.test.ts   ← TS codegen tests (REMOVE)
compiler/ts/src/codegen/codegen_c.test.ts ← C codegen tests (REMOVE)
compiler/c/                               ← entire C runtime (REMOVE)
runtime/                                  ← entire TS runtime (REMOVE)
compiler/ts/dist/                         ← compiled TS output (REBUILD)
test/apps/*.ts                            ← generated TS artifacts (REMOVE)
```

### Files to KEEP:
```
compiler/ts/src/lexer/        ← lexer (unchanged)
compiler/ts/src/parser/       ← parser (unchanged)
compiler/ts/src/checker/      ← type checker (unchanged)
compiler/ts/src/ir/           ← IR types + lowering (unchanged)
compiler/ts/src/modules/      ← module resolver (unchanged)
compiler/ts/src/cli/          ← CLI (update for WASM)
compiler/ts/src/codegen/      ← NEW: emit_wat.ts only
spec/                         ← language spec (unchanged)
stdlib/                       ← stdlib .japl files (unchanged)
test/apps/*.japl              ← test programs (unchanged)
plans/                        ← planning docs (unchanged)
```

### New files to CREATE:
```
compiler/ts/src/codegen/emit_wat.ts    ← WAT code generator
compiler/ts/src/codegen/wasm.test.ts   ← WASM codegen tests
japl-runtime/                          ← Rust runtime (new crate)
  Cargo.toml
  src/
    main.rs
    engine.rs
    process.rs
    scheduler.rs
    mailbox.rs
    supervisor.rs
    host.rs
    distribution.rs
    wire.rs
```

## Execution Loop

Every agent follows: **dev → test ALL → fix → repeat**

No agent declares "done" until:
1. Code compiles
2. Feature works end-to-end (JAPL source → WASM → correct output)
3. ALL existing tests still pass
4. A real app (not hello world) exercises the feature

## Waves

### WAVE 1: Codegen + Cleanup (3 agents)

Agent: cleanup
  - Delete emit.ts, emit_c.ts, compiler/c/, runtime/
  - Delete all .ts artifacts from test/apps/
  - Update .gitignore (remove TS/C patterns, add WASM patterns)
  - Update CLI to remove TS/C target options
  - Verify compiler still builds (lexer/parser/checker/IR intact)

Agent: wat-core
  - Create emit_wat.ts
  - Emit WAT for: functions, i64/f64 arithmetic, booleans, if/else, let
  - WASI import for fd_write (println)
  - Test loop: write JAPL → emit WAT → wat2wasm → wasmtime → verify output
  - Minimum: hello.japl and fibonacci.japl produce correct output

Agent: wat-types
  - Extend emit_wat.ts with: strings, records, tagged unions, lists
  - Pattern matching via br_table on discriminant
  - Field access via struct offsets or memory layout
  - Test loop: calculator.japl, json.japl, records.japl → correct output

### WAVE 2: Advanced Codegen (2 agents)

Agent: wat-closures
  - Function references, indirect calls, closure environments
  - Higher-order functions, lambda, pipe operator
  - Tail calls (WASM native return_call)
  - Module imports (multi-WASM linking)
  - Test loop: closures.japl, higher_order.japl, pipes.japl → correct output

Agent: wat-io
  - WASI for file I/O (path_open, fd_read, fd_write)
  - FFI mapped to WASI capabilities
  - show() for all value types
  - Test loop: file_processor.japl → reads/writes actual files

### WAVE 3: Rust Runtime (2 agents)

Agent: runtime-processes
  - Rust crate: japl-runtime
  - wasmtime embedding with WASI support
  - Host functions: spawn, send, receive, self
  - Process = separate WASM instance on tokio task
  - Mailbox = tokio mpsc channel per process
  - Test loop: concurrent_counter.japl → real threads, real messages

Agent: runtime-supervisor
  - Supervisor: monitor child WASM instances
  - WASM trap = process crash → restart
  - OneForOne, AllForOne, RestForOne strategies
  - Named process registry
  - Test loop: crash a process → supervisor restarts it

### WAVE 4: Distribution (2 agents)

Agent: distribution
  - TCP connections between japl-runtime instances
  - Binary wire protocol (serialize JAPL values)
  - Distributed PIDs (node + local ID)
  - Message routing (local → mailbox, remote → TCP)
  - Node handshake (cookie auth)
  - Health monitoring (heartbeat)
  - Test loop: two processes on different ports communicate

Agent: docker-proof
  - Dockerfile for japl-runtime
  - docker-compose: two containers, separate networks
  - Chaos monkey: kill node, verify restart
  - End-to-end: distributed counter across containers

### WAVE 5: CLI + Polish (1 agent)

Agent: cli-final
  - japl build app.japl → .wat → .wasm → build/app (bundled binary)
  - japl run app.japl → compile → run → cleanup
  - japl run --node alpha --listen :9000 app.japl
  - japl build --emit-wat app.japl (debug)
  - Remove ALL TS/C/JS references from CLI
  - Single binary output, no intermediates visible

### WAVE 6: Final Verification (1 agent)

Agent: verifier
  - ALL 12 apps compile to WASM and run correctly
  - Concurrent counter with real threads
  - Distributed test across containers
  - Chaos test (kill + restart)
  - 1000-process spawn test
  - NO shortcuts. Every failure documented and fixed.

## Agent Count: 11 agents, 6 waves
## Target: JAPL produces native binaries via WASM. No C. No JS. Real processes. Real distribution.
