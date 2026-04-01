# wasmCloud Integration Guide

> **Status**: The JAPL provider is a standalone NATS sidecar, not yet a native
> wasmCloud capability provider. The component → provider link described below
> is the intended architecture. Converting the provider to use
> `wasmcloud-provider-sdk` is required to make it a true wasmCloud capability.

## Architecture Overview

```
JAPL Source (.japl)
    |
    v
JAPL Compiler (--target component)
    |
    v
WASM Component (.wasm)          japl-provider (native binary)
    |                                |
    +--- wasmCloud host ---+         |
    |                      |         |
    v                      v         v
japl:runtime/processes  <-link->  NATS subjects
                                     |
                                     v
                              japl.runtime.spawn
                              japl.runtime.send.<pid>
                              japl.runtime.receive.<pid>
                              japl.runtime.self-pid
```

A compiled JAPL component imports `japl:runtime/processes` via the WIT
component model. The `japl-provider` is a standalone NATS sidecar (not yet
a native wasmCloud capability). It must be started separately and translates
NATS request/reply messages against a shared process table. Converting it
to use `wasmcloud-provider-sdk` is required for true wasmCloud-managed linking.

## Provider Details

### Startup

The provider (`japl-provider/src/main.rs`) is a standalone Tokio binary:

1. Reads `NATS_URL` from the environment (default `nats://localhost:4222`).
2. Connects to NATS and subscribes to `japl.runtime.>`.
3. Spawns a self-test task that exercises spawn/send/receive over NATS.
4. Enters an event loop dispatching each incoming NATS message to a handler.

### NATS Subjects

| Subject | Payload (JSON) | Response (JSON) | Description |
|---------|----------------|-----------------|-------------|
| `japl.runtime.spawn` | `{ "closure_data": [bytes] }` | `{ "pid": <u64> }` | Create a new process, returns its PID |
| `japl.runtime.send.<pid>` | `{ "message": [bytes] }` | `"ok"` or `"err"` | Deliver a message to the process mailbox |
| `japl.runtime.receive.<pid>` | `{}` | `{ "message": [bytes] }` | Block until a message is available, then return it |
| `japl.runtime.self-pid` | any | `{ "pid": 0 }` | Placeholder -- returns 0 (not yet context-aware) |

### Process Table

- Processes are in-memory structs with a sequential PID counter starting at 1.
- Each process has a `Vec<Vec<u8>>` mailbox (FIFO, unbounded).
- A `tokio::sync::Notify` wakes the receive handler when a message arrives.
- There is no persistence -- all processes are lost on provider restart.

## WIT Interface

The runtime interface is defined in `wit/japl-runtime/world.wit`:

```wit
package japl:runtime@0.1.0;

interface processes {
    spawn: func(closure-data: list<u8>) -> u64;
    send: func(pid: u64, message: list<u8>);
    receive: func() -> list<u8>;
    self-pid: func() -> u64;
}

interface logging {
    println: func(message: string);
}
```

The app world that imports this is in `wit/japl-app/world.wit`:

```wit
world runtime-app {
    import japl:runtime/processes@0.1.0;
    import japl:runtime/logging@0.1.0;
    export handler;
}
```

### WIT vs. Provider Alignment

The WIT interface and the NATS provider are semantically aligned:

| WIT function | NATS subject | Notes |
|---|---|---|
| `spawn(closure-data)` | `japl.runtime.spawn` | Matched |
| `send(pid, message)` | `japl.runtime.send.<pid>` | PID encoded in subject |
| `receive()` | `japl.runtime.receive.<pid>` | Provider needs caller PID from context |
| `self-pid()` | `japl.runtime.self-pid` | Returns placeholder 0 |

Key difference: `receive()` in WIT takes no arguments (the current process is
implicit), but the NATS subject requires a PID suffix. The wasmCloud link
layer or a thin adapter must inject the caller's PID.

## How to Start the Provider

```bash
# Start NATS (if not already running)
nats-server &

# Build and run the provider
cd japl-provider
cargo run --release
# or with a custom NATS URL:
NATS_URL=nats://my-nats:4222 cargo run --release
```

The provider prints a self-test pass/fail on startup.

## Deploying a JAPL App with WADM

```bash
# 1. Compile JAPL source to a WASM component
japl build --target component my_app.japl -o /tmp/japl_app.wasm

# 2. Build the provider binary (or point to a pre-built path)
cd japl-provider && cargo build --release

# 3. Deploy using wash
wash app deploy deploy/japl-provider.wadm.yaml
```

The WADM manifest (`deploy/japl-provider.wadm.yaml`) declares:
- A `component` named `app` loaded from the compiled WASM.
- A `capability` named `japl-provider` that supplies `japl:runtime/processes`.
- A link connecting the two.

## What Works

- Provider compiles and runs as a standalone NATS service.
- Spawn, send, and blocking receive all function correctly (self-test passes).
- WIT interfaces are defined and semantically match the provider API.
- WADM component manifest can be deployed via `wash app deploy` (provider runs as separate sidecar).

## Current Blockers and Limitations

1. **self-pid is a placeholder.** Returns 0; needs wasmCloud call-context
   integration to return the actual PID of the calling component instance.

2. **No real closure execution.** `spawn` creates a mailbox but ignores
   `closure_data`. The provider does not load or execute WASM closures --
   it is a message-routing broker only.

3. **receive() PID gap.** The WIT `receive()` takes no arguments (implicit
   self), but the NATS subject needs an explicit PID. A bridge adapter is
   needed in the wasmCloud link layer.

4. **No persistence.** The process table is in-memory. Provider restart
   loses all processes and pending messages.

5. **No process supervision.** No monitors, links, or crash restart
   semantics. Processes are fire-and-forget mailboxes.

6. **Provider is not yet a wasmCloud native capability provider.** It runs
   as a standalone NATS service. Wrapping it with the wasmCloud provider SDK
   (`wasmcloud-provider-sdk`) would enable proper lifecycle management,
   health checks, and link-definition handling.

7. **logging interface not implemented.** The WIT defines a `logging`
   interface but the provider has no corresponding NATS handler.

8. **Unbounded mailboxes.** No backpressure or size limits on process
   mailboxes; a fast sender can exhaust memory.
