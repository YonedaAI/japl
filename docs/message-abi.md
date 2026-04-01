# JAPL Message ABI

## Overview

Messages between JAPL processes are serialized as raw byte sequences copied
from the sender's WASM linear memory to the receiver's WASM linear memory.
The runtime treats messages as opaque byte blobs; type interpretation happens
entirely at the WASM level via pattern matching.

## Variant (Message) Struct Layout

All messages use the **variant struct** layout in WASM linear memory:

```
Offset  Size   Field
──────  ────   ─────
0       u32    tag           — variant discriminant (little-endian)
4       u32    field_count   — number of payload fields (little-endian)
8       i64    field_0       — first field value (boxed as i64)
16      i64    field_1       — second field value
...
8+8*N   i64    field_N       — last field value

Total size = 8 + 8 * field_count  (bytes)
```

All multi-byte integers are **little-endian** (WASM native byte order).
Pointer fields are byte offsets into the sender's linear memory; they are
**not** relocated on receive — the byte blob is copied verbatim and the
receiver re-interprets it in its own address space (which is the same
linear memory when processes share a module instance via the scheduler).

## String Layout (referenced by pointer fields)

JAPL strings are length-prefixed:

```
Offset  Size       Field
──────  ────       ─────
0       u32        length   — byte length of the UTF-8 data (little-endian)
4       u8[length] data     — raw UTF-8 string bytes
```

## Send Path

1. JAPL source calls `send(pid, msg)`.
2. The compiler emits `call $japl.send` with two i64 arguments:
   - `pid`     — target process ID
   - `msg_ptr` — byte offset of the variant struct in linear memory
3. The host function `japl.send` reads the 8-byte header at `msg_ptr`:
   - Extracts `tag` (u32 at offset 0) and `field_count` (u32 at offset 4).
   - Computes `total_size = 8 + 8 * field_count`.
   - Copies `total_size` bytes starting at `msg_ptr` into a `Vec<u8>`.
4. The host sends a `SchedulerCommand::Send` containing the target PID and
   the copied byte vector to the scheduler via an `mpsc` channel.
5. The scheduler looks up the target process's `mpsc::Sender<ProcessMessage>`
   and delivers a `ProcessMessage::Deliver(Vec<u8>)`.
6. Before delivery the scheduler checks the per-process mailbox size counter.
   If the mailbox is at capacity (`DEFAULT_MAX_MAILBOX_SIZE = 10,000`), the
   message is **dropped** and an error is logged.

### Error handling on send

- If WASM memory cannot be obtained, the raw `msg_ptr` bytes (8 bytes,
  little-endian i64) are sent as the message payload — a defensive fallback.
- If the 8-byte header is out of bounds, the same fallback applies.
- If the full variant body exceeds memory bounds, the safely-readable
  prefix is copied.

## Receive Path

1. JAPL source enters a `receive { Pattern => body }` expression.
2. The compiler emits `call $japl.receive`, which returns an i64:
   - On success: the byte offset where the message was written.
   - On error/shutdown: `-1`.
3. The host function `japl.receive`:
   a. Checks the process's local `mailbox` deque (a `VecDeque<Vec<u8>>`).
      If a message is already queued, it pops and uses it.
   b. Otherwise, **blocks** on `receiver.recv()` (an `mpsc::Receiver`)
      until a `ProcessMessage::Deliver(bytes)` arrives.
      A `ProcessMessage::Shutdown` or channel disconnect returns `-1`.
   c. Reads the current `heap_ptr` global (bump allocator pointer).
   d. Copies the message bytes into linear memory at `heap_ptr`.
   e. Advances `heap_ptr` by the 8-byte-aligned message size.
   f. Returns the original `heap_ptr` value (the message's address).
4. Pattern matching on the variant tag and fields happens at the WASM level
   in compiler-generated code.

### Memory allocation

The receiver's bump allocator (`heap_ptr` exported global) is advanced by:

```
aligned_size = (msg_bytes.len() + 7) & !7
new_heap_ptr = heap_ptr + aligned_size
```

There is **no reclamation** — received messages permanently consume heap
space until the process exits.

## Closure Layout (for `spawn`)

When `spawn(fun)` is called, the closure struct is also copied through the
runtime. Its layout differs from messages:

```
Offset  Size   Field
──────  ────   ─────
0       i64    table_index   — WASM function table index for call_indirect
8       i64    capture_0     — first captured variable (boxed as i64)
16      i64    capture_1     — second captured variable
...

Total size = 8 + 8 * num_captures
```

The compiler passes both the closure pointer and its byte size.  If the
exact size cannot be determined statically, a safe upper bound of
`MAX_CLOSURE_SIZE = 2048` bytes is used.

## Process Identity

- `ProcessId` is a `u64`, allocated sequentially by the scheduler.
- `japl.self_pid()` returns the caller's PID as an i64.
- `japl.is_alive(pid)` queries the scheduler for process liveness.
- `japl.mailbox_size(pid)` queries the current mailbox depth.

## Implications

- **No envelope or type tag** — the receiver must know the expected message
  type via its `receive` patterns.  A mismatched send silently delivers
  bytes that will fail pattern matching.
- **No runtime type checking** at the message boundary.
- **Pointer fields are not relocated** — this is safe only because all
  processes currently share the same module instance and linear memory
  within a single scheduler.
- **Mailbox backpressure** is enforced at the scheduler level with a
  configurable cap (default 10,000 messages).  Excess messages are dropped.
- **Blocking receive** — a process calling `receive` blocks its OS thread
  until a message arrives or the scheduler initiates shutdown.

## Deployed Mode (NATS Provider)

When running on wasmCloud via the `japl-provider`, processes are managed by
a NATS-backed process table instead of the local wasmtime scheduler.

### Message format

In deployed mode, all NATS payloads are **JSON-encoded** rather than the raw
variant-struct bytes used in local mode.  The provider serializes/deserializes
with `serde_json`.

| Operation | NATS Subject                  | Request Body                       | Response Body                |
|-----------|-------------------------------|------------------------------------|------------------------------|
| spawn     | `japl.runtime.spawn`          | `{"closure_data": [u8, ...]}`      | `{"pid": <u64>}`             |
| send      | `japl.runtime.send.<pid>`     | `{"message": [u8, ...]}`           | `"ok"` or `"err"`            |
| receive   | `japl.runtime.receive.<pid>`  | `{}`                               | `{"message": [u8, ...]}`     |
| self-pid  | `japl.runtime.self-pid`       | (empty)                            | `{"pid": <u64>}`             |

### Process identity

PIDs are allocated by the provider's `ProcessTable.next_pid` counter
(a simple `u64` incremented under a `tokio::sync::Mutex`).  PID `0` is
reserved as an error/placeholder value.

### Message delivery

The provider uses NATS **request/reply** semantics via `async_nats`.
A send operation publishes to `japl.runtime.send.<pid>`, and the provider
replies with `"ok"` or `"err"`.  Receive blocks by awaiting a
`tokio::sync::Notify` signal on the target process's mailbox.

### Differences from local mode

| Aspect             | Local Mode                           | Deployed Mode                         |
|--------------------|--------------------------------------|---------------------------------------|
| Message encoding   | Raw variant-struct bytes (binary)    | JSON-encoded byte arrays              |
| Transport          | `mpsc` channels (in-process)         | NATS request/reply (networked)        |
| Process table      | Scheduler `HashMap` + `mpsc`         | Provider `HashMap` + `Notify`         |
| Mailbox cap        | 10,000 messages (configurable)       | Unbounded (provider Vec)              |
| self-pid           | Returns caller's actual PID          | Returns placeholder `0` (no context)  |
| Blocking receive   | Blocks OS thread on `mpsc::recv`     | Blocks async task on `Notify`         |
| Pointer relocation | Not needed (shared linear memory)    | Not applicable (JSON payloads)        |

## ABI Stability

### Stable

These operations have fixed semantics across both local and deployed modes.
Their WIT signatures and behavior are committed and will not change in
backwards-incompatible ways.

- **`spawn`** — Create a new process.  Returns a non-zero PID on success.
- **`send`** — Deliver a message to a process mailbox.  Returns success/failure.
- **`receive`** — Block until a message arrives.  Returns raw message bytes.

### Experimental

These operations exist in both modes but have known limitations or semantics
that may change.

- **`self-pid`** — In deployed mode, NATS request/reply does not carry
  caller context, so the provider returns `0`.  Semantics will change when
  caller identity is threaded through the NATS subject or headers.
- **`mailbox-size`** — Returns current mailbox depth.  Semantics are
  well-defined locally but undefined when mailboxes span multiple provider
  instances in a distributed deployment.
- **`is-alive`** — Queries the process table.  Only reflects liveness within
  a single provider instance; cross-provider liveness is not yet defined.
- **`logging.println`** — Writes to stdout.  Output destination in deployed
  mode (provider stderr, structured log, NATS subject) is not yet stable.

## Future: Typed Protocols

- `Pid<T>` would encode the expected message type at the type level.
- `send(pid: Pid<T>, msg: T)` would be enforced at compile time.
- A `Codec` module would handle serialization for cross-node messages,
  adding type tags and version headers to the byte format.
- Cross-node messaging would require pointer relocation or a
  serialization/deserialization pass for pointer-containing fields.
