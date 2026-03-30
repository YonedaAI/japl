# Distribution Test Report

Date: 2026-03-30
Runtime: japl-runtime (Rust + wasmtime)
Build: cargo build succeeded with 1 warning (dead_code on `Spawn` variant)

## Test 1: Two-node TCP connection
Status: **PARTIAL PASS**

Alpha starts, listens, prints "Hello from JAPL!" and exits before beta connects when
run manually with `sleep 2` between launches. The scheduler exits as soon as the sole
process (`_start`) completes, tearing down the TCP listener thread.

When the existing `test_distribution.sh` script runs (tighter timing), the connection
succeeds:

```
[alpha] Listening on 0.0.0.0:9876
[alpha] Distribution layer active
[alpha] Accepted connection from node 'beta'
[beta] Connected to node 'alpha' at localhost:9876
[beta] Distribution layer active
Hello from JAPL!
[beta] Connection to 'alpha' lost: failed to fill whole buffer
Hello from JAPL!
```

The TCP handshake completes (Handshake -> HandshakeOk), both nodes print output, but
beta's reader thread reports "Connection to 'alpha' lost" when alpha's process exits
and closes the socket.

**Root Cause:** The scheduler loop (`Scheduler::run`) exits when `alive_count == 0`.
It does not account for the distribution layer keeping the node alive as a service.
A `--listen` node should stay alive even after its wasm program finishes.

## Test 2: Local process communication
Status: **FAIL**

```
[parent] pid = 0
[parent] spawned child pid = 1
[parent] sent 42 to child 1
[pid 1] could not find export '__process_entry': failed to find function export `__process_entry`
```

Then hangs indefinitely (parent blocks on `receive()` waiting for a reply that never arrives).

**Root Cause:** `processes.wasm` was compiled from Rust (`tests/wasm-src/processes.rs`)
and does NOT export `__process_entry` or `heap_ptr`. The `japl.spawn` host function
calls `SchedulerCommand::SpawnClosure` which tries to instantiate a new wasm module and
call `__process_entry(closure_ptr)`. Since the export doesn't exist, the child process
fails immediately.

The Rust test source was designed for a model where spawn re-runs `_start` and the child
checks `self_pid()` to differentiate itself. But the runtime's spawn implementation uses
`SpawnClosure` which requires `__process_entry` -- an export only the JAPL compiler
produces.

## Test 3: KV store with processes
Status: **PASS** (functional output correct, but process does not terminate)

```
=== JAPL Distributed Key-Value Store ===
Spawned 2 partitions
Stored key=1 on partition 1
Stored key=2 on partition 1
Stored key=3 on partition 2
Got key=1: 10
Got key=3: 30
Partition 1 size: 2
Partition 2 size: 1
=== KV Store Test Complete ===
```

All 8 expected operations completed correctly. The JAPL-compiled `kvstore.wasm` properly
exports `__process_entry` and `heap_ptr`, so spawn/send/receive all work end-to-end.

The process hangs after completion because the two partition child processes are stuck in
infinite `receive` loops and have no shutdown mechanism. The scheduler waits for all
processes to exit.

## Test 4: Distribution script (tests/test_distribution.sh)
Status: **PARTIAL PASS**

- Script Test 1 (two-node hello.wasm): PASS -- handshake completes, both nodes print
- Script Test 2 (processes.wasm + distribution): FAIL -- `__process_entry` not found
- Script Test 3 (local-only mode): hello.wasm PASS, processes.wasm FAIL (hangs)

The script itself does not exit cleanly because processes.wasm hangs in Test 3.

## Issues Found

1. **Scheduler does not keep node alive for distribution (Medium)**
   The scheduler exits when all wasm processes finish, even if `--listen` is active.
   A distributed node should remain alive to serve requests.
   File: `src/scheduler.rs`, `Scheduler::run()` line 190.

2. **processes.wasm missing `__process_entry` export (High)**
   The Rust-compiled test wasm uses a `self_pid()` dispatch model, but the runtime's
   `spawn` host function uses `SpawnClosure` which requires `__process_entry`.
   Either the test needs rewriting to export `__process_entry`, or the runtime needs
   a fallback spawn mode that re-runs `_start`.
   Files: `src/host.rs` (spawn), `src/scheduler.rs` (spawn_closure_process),
   `tests/wasm-src/processes.rs`.

3. **No graceful shutdown for child processes (Low)**
   Processes stuck in `receive` loops have no way to be notified that the parent
   exited. The scheduler should send `ProcessMessage::Shutdown` to remaining
   processes when the main process exits, or exit when the main process finishes.
   File: `src/scheduler.rs` line 228.

4. **Beta sees "Connection lost" on normal peer exit (Low)**
   When alpha exits normally, beta's reader thread logs an error. This is expected
   TCP behavior but should be handled gracefully (info log, not error).
   File: `src/distribution.rs` line 238.

## What Works

- **Wire protocol**: Encode/decode for all message types (Handshake, HandshakeOk, Send,
  SpawnRequest, SpawnResponse, Ping, Pong) is correctly implemented with framed TCP.
- **TCP handshake**: Two nodes successfully perform Handshake -> HandshakeOk exchange
  with cookie validation.
- **Connection tracking**: Nodes register each other in the connections HashMap after
  handshake.
- **Reader thread spawning**: Background reader threads are correctly started for each
  accepted/initiated connection.
- **JAPL-compiled wasm (kvstore.wasm)**: Full spawn/send/receive pipeline works
  end-to-end. Parent spawns children, sends typed messages, receives typed responses.
- **Host functions**: `japl.spawn`, `japl.send`, `japl.receive`, `japl.self_pid`,
  `japl.println` all function correctly with JAPL-compiled wasm.
- **hello.wasm standalone**: Runs and prints "Hello from JAPL!" correctly.

## What Doesn't Work

- **Cross-node message delivery**: Could not be tested end-to-end because hello.wasm
  doesn't spawn processes. The code path exists (`remote_send` ->
  `SchedulerCommand::Send`) but was not exercised with actual inter-node messages.

## Bug Fixes Applied (2026-03-30)

### Bug 1: Scheduler exits with --listen active (FIXED)
File: `src/scheduler.rs`
When `alive_count == 0` and a distribution layer is present, the scheduler now enters
a keepalive loop that continues processing commands from the channel. This allows
`--listen` nodes to remain alive as services after their initial wasm program finishes.

### Bug 2: Old processes.wasm incompatible with SpawnClosure (FIXED)
Files: `tests/process_test.japl`, `tests/process_test.wasm`, `tests/test_distribution.sh`
Replaced the Rust-compiled `processes.wasm` with a JAPL-compiled `process_test.wasm`
that properly exports `__process_entry` and `heap_ptr`. The test uses spawn/send/receive
with a Ping/Pong protocol. Output: "got pong: 42" / "parent done". Test scripts updated
to reference `process_test.wasm`.

### Bug 3: No graceful child shutdown (FIXED)
File: `src/scheduler.rs`
The scheduler now tracks when the main process (PID 0) exits. In non-distribution mode,
once the main process exits, the scheduler calls `std::process::exit(0)` to terminate
all threads (including children stuck in infinite receive loops). This fixes the KV store
hang -- it now exits cleanly after printing all results.

### Bug 4: Cross-node connection test (ADDED)
File: `tests/dist_test.sh`
New test script that exercises: (1) two-node TCP connection with handshake, (2) KV store
with process spawning. Both tests pass.

## Stdlib WASM Compilation Results (2026-03-30)

Compiler: `compiler/ts/dist/index.js build --emit-wat`

| Module       | Status | Notes                                           |
|-------------|--------|-------------------------------------------------|
| IO.japl     | PASS   | Compiles to WAT successfully                    |
| Math.japl   | PASS   | Compiles to WAT successfully                    |
| Option.japl | PASS   | Compiles to WAT successfully                    |
| Process.japl| PASS   | Compiles to WAT successfully                    |
| Result.japl | PASS   | Compiles to WAT successfully                    |
| String.japl | PASS   | Compiles to WAT successfully                    |
| Core.japl   | FAIL   | `pub` keyword not supported in WAT codegen      |
| List.japl   | FAIL   | `pub` keyword + list pattern syntax (`[]`, `..`) not in WAT codegen |
| Test.japl   | FAIL   | `pub` keyword + `assert` keyword not in WAT codegen |

**Summary**: 6/9 stdlib modules compile to WAT. The 3 failures all share the same root
cause: the `pub` visibility modifier is parsed as a token but the WAT code generator
does not handle `module` declarations with `pub` exports. List.japl additionally uses
list literal patterns (`[]`, `[h, ..t]`) and Test.japl uses `assert` -- both are
missing from the WAT codegen. These are codegen limitations, not syntax errors in the
`.japl` files.
