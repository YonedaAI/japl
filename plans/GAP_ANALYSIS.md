# JAPL Gap Analysis: Claims vs Reality

> **One-liner:** JAPL numbers are strict, checked, and explicit — no silent overflow, no implicit promotion, no runtime surprises.

---

## Severity Scale

- **RED** — Claimed, not built. Core promise broken.
- **YELLOW** — Partially built, not wired end-to-end.
- **GREEN** — Built and working.

---

## 1. Core Language

| Feature | Claim | Reality | Status |
|---------|-------|---------|--------|
| Immutable let bindings | Yes | Works in interpreter + codegen | GREEN |
| First-class functions | Yes | Lambda, closures work | GREEN |
| Algebraic data types | Yes | Sum types + constructors | GREEN |
| Pattern matching | Exhaustive | Works but no exhaustiveness enforcement | YELLOW |
| Result/Option | Yes | Types exist, ? operator parsed | GREEN |
| Pipe operator `\|>` | Yes | Works (desugars to fn application) | GREEN |
| Record types | Yes | Literals, access, update work | GREEN |
| List types | Yes | Literals work, no persistent impl | YELLOW |
| Type inference | Bidirectional, local | Works for most expressions | GREEN |
| Type annotations | Yes | Parsed, checked | GREEN |
| Strict evaluation | Yes | Both targets are strict | GREEN |
| Numbers (Int/Float/Byte) | Checked, explicit | Overflow protection, no implicit promotion | GREEN |
| Literal syntax | Hex, binary, scientific, separators | Just added | GREEN |
| String interpolation | Not claimed | Not built | — |

## 2. Type System

| Feature | Claim | Reality | Status |
|---------|-------|---------|--------|
| Parametric polymorphism | Yes | Generalization + instantiation work | GREEN |
| Traits / type classes | Yes | Defined in env, parsed | YELLOW |
| Trait implementations | Yes | Parsed but not resolved during codegen | YELLOW |
| Row polymorphism | Yes | Unification supports row vars | YELLOW |
| Opaque types | Yes | Parsed, not enforced | YELLOW |
| Capability types | Yes | Not built | RED |
| Effect tracking | Pure/IO/Process/Fail | Checker tracks effects | YELLOW |
| Effect enforcement | Yes | Checker warns but codegen ignores | RED |
| Linearity checking | Yes | Checker has it, codegen ignores | RED |
| Ownership types | Owned<T>, Ref<T> | Parsed, not enforced at runtime | RED |
| Exhaustiveness checking | Yes | Not implemented | RED |

## 3. Concurrency (LOCAL)

| Feature | Claim | Reality | Status |
|---------|-------|---------|--------|
| Lightweight processes | Yes | TS runtime: async functions as processes | GREEN |
| Process spawn | Yes | `spawn()` works | GREEN |
| Message send | Yes | `send(pid, msg)` works | GREEN |
| Message receive | Yes | `receive()` works (blocking) | GREEN |
| Typed mailboxes | Yes | Mailbox<unknown> — NOT typed | RED |
| Selective receive | Yes | Runtime supports predicate matching | GREEN |
| Process isolation | Full isolation | Shared JS heap — no isolation | RED |
| Process priorities | Yes | Not implemented | RED |
| Reduction counting | Yes (BEAM-style) | Not implemented | RED |
| Process links | Yes | Implemented in runtime | GREEN |
| Process monitors | Yes | Implemented in runtime | GREEN |

## 4. Supervision

| Feature | Claim | Reality | Status |
|---------|-------|---------|--------|
| OneForOne | Yes | Runtime implements it | GREEN |
| AllForOne | Yes | Runtime implements it | GREEN |
| RestForOne | Yes | Runtime implements it | GREEN |
| Restart intensity | Yes | Max restarts / window | GREEN |
| Restart policies | Permanent/Transient/Temporary | Implemented | GREEN |
| Declarative supervisor | `supervisor { ... }` syntax | Parsed but not wired to runtime | YELLOW |
| Typed crash reasons | Yes | CrashReason enum exists | GREEN |
| Supervision from JAPL code | Yes | Cannot actually write supervisor in JAPL that runs | RED |

## 5. Distribution — THE BIG GAP

| Feature | Claim | Reality | Status |
|---------|-------|---------|--------|
| Location-transparent PIDs | Yes | PIDs are local strings only | RED |
| Remote spawn | `spawn_remote(node, fn)` | Not built | RED |
| Remote send | Send to remote PID | Not built | RED |
| Node discovery | Built-in mesh | Not built | RED |
| Node connection | TCP/TLS between nodes | Not built | RED |
| Node health monitoring | Phi accrual detector | Not built | RED |
| Message serialization | Type-derived from ADTs | Not built | RED |
| Protocol versioning | Schema evolution | Not built | RED |
| Wire protocol | Binary frame format | Not built | RED |
| Distributed supervision | Cross-node supervisors | Not built | RED |
| Process migration | Move process between nodes | Not built | RED |
| Service registry | Named services | Not built | RED |
| Split-brain handling | Strategies | Not built | RED |
| Cluster membership | Gossip protocol | Not built | RED |

**Distribution score: 0/14 features built. This is the biggest gap in JAPL.**

## 6. Tooling

| Feature | Claim | Reality | Status |
|---------|-------|---------|--------|
| `japl build` | Yes | Works (TS + C targets) | GREEN |
| `japl run` | Yes | Works (TS target) | GREEN |
| `japl test` | Yes | Basic, finds test blocks | YELLOW |
| `japl fmt` | Yes | Stub only | RED |
| `japl new` | Yes | Scaffolds project | GREEN |
| `japl check` | Yes | Type checks | GREEN |
| `japl.toml` | Yes | Parsed | GREEN |
| REPL | Yes | Not built | RED |
| LSP | Yes | Not built | RED |
| Package manager | Yes | Not built | RED |
| Package registry | Yes | Not built | RED |
| Documentation generator | Yes | Not built | RED |
| Cross-compilation | Yes | Not built (C target is local only) | RED |
| Static binary output | Go-like | C compiles but no build system for static linking | YELLOW |
| Profiler | Yes | Not built | RED |
| Debugger | Not explicitly | Not built | — |

## 7. Standard Library

| Module | Claim | Reality | Status |
|--------|-------|---------|--------|
| Core | Yes | Written, not compiled/tested via JAPL compiler | YELLOW |
| String | Yes | Written as .japl file | YELLOW |
| List | Yes | Written as .japl file | YELLOW |
| Map | Yes | Not written | RED |
| Set | Yes | Not written | RED |
| Option | Yes | Written as .japl file | YELLOW |
| Result | Yes | Written as .japl file | YELLOW |
| IO | Yes | Not built (foreign only) | RED |
| Process | Yes | Written as .japl file (not runnable) | YELLOW |
| Supervisor | Yes | Not written in JAPL | RED |
| Json | Yes | Not built | RED |
| Http | Yes | Not built | RED |
| Time | Yes | Not built | RED |
| File | Yes | Not built | RED |
| Crypto | Yes | Not built | RED |
| Db | Yes | Not built | RED |
| Test | Yes | Written as .japl file | YELLOW |
| Debug | Yes | Not built | RED |
| Net (TCP/UDP) | Yes | Not built | RED |

## 8. Multi-file / Module System

| Feature | Claim | Reality | Status |
|---------|-------|---------|--------|
| Module declarations | Yes | Parsed | YELLOW |
| Import statements | Yes | Parsed but not resolved | RED |
| Multi-file compilation | Yes | Single-file only | RED |
| Public/private visibility | Yes | Parsed but not enforced | RED |
| Circular dependency detection | Implied | Not built | RED |
| Module-qualified names | `Module.function` | Not resolved | RED |

---

## Summary Scorecard

| Category | GREEN | YELLOW | RED | Total |
|----------|-------|--------|-----|-------|
| Core Language | 11 | 2 | 0 | 13 |
| Type System | 2 | 4 | 5 | 11 |
| Concurrency | 7 | 0 | 4 | 11 |
| Supervision | 6 | 1 | 1 | 8 |
| Distribution | 0 | 0 | 14 | 14 |
| Tooling | 5 | 2 | 8 | 15 |
| Stdlib | 0 | 7 | 11 | 18 |
| Module System | 0 | 1 | 5 | 6 |
| **Total** | **31** | **17** | **48** | **96** |

**31 built, 17 partial, 48 missing.**

**The honest assessment:** JAPL is a working compiler with a local process runtime. It is NOT yet a distributed language. The biggest gap is distribution (0/14), followed by stdlib (0/18 fully working) and module system (0/6).

---

## Plan: Make JAPL 100% Distributed

**Goal:** `japl run --node alpha --listen :9000 app.japl` on Machine A, `japl run --node beta --connect alpha:9000 app.japl` on Machine B. Processes communicate across machines transparently.

### Phase D1: Wire Protocol + Serialization (Foundation)

```
runtime/src/
  wire/
    protocol.ts      # Binary message framing
    serialize.ts      # Value → bytes (from type structure)
    deserialize.ts    # Bytes → value
    codec.ts          # Codec registry (per ADT)

Wire frame:
  [4 bytes: length][1 byte: msg type][8 bytes: from PID][8 bytes: to PID][N bytes: payload]

Message types:
  0x01  SEND           pid, serialized value
  0x02  SPAWN_REQUEST  module, function, args
  0x03  SPAWN_RESPONSE pid
  0x04  LINK           pid_a, pid_b
  0x05  EXIT           pid, reason
  0x06  MONITOR        watcher, target
  0x07  NODE_DOWN      node_id
  0x08  PING
  0x09  PONG
  0x0A  HANDSHAKE      node_id, cookie

Serialization (type-derived):
  Int    → tag 0x01 + 8 bytes (i64 big-endian)
  Float  → tag 0x02 + 8 bytes (f64)
  String → tag 0x03 + 4 bytes length + UTF-8 bytes
  Bool   → tag 0x04 + 1 byte
  Byte   → tag 0x05 + 1 byte
  List   → tag 0x06 + 4 bytes length + N elements
  Record → tag 0x07 + 4 bytes field count + (key + value) pairs
  Tagged → tag 0x08 + tag string + field count + fields
  Pid    → tag 0x09 + 8 bytes node hash + 8 bytes local id
  Unit   → tag 0x0A
  Nil    → tag 0x0B
```

**Tests:** 30+ (serialize/deserialize round-trip for every type)

### Phase D2: Node Identity + Connections

```
runtime/src/
  node/
    node.ts           # Node identity (name, address, cookie)
    connection.ts     # TCP connection management
    handshake.ts      # Node authentication (shared cookie)
    health.ts         # Heartbeat / phi accrual failure detection
    registry.ts       # Node registry (who's connected)

Node identity:
  NodeId = { name: string, host: string, port: number }

  // Machine A
  const node = createNode({ name: "alpha", listen: ":9000", cookie: "secret" });

  // Machine B
  const node = createNode({ name: "beta", connect: "alpha:9000", cookie: "secret" });

Connection lifecycle:
  1. TCP connect
  2. Handshake (exchange node IDs + verify cookie)
  3. Heartbeat loop (every 5s)
  4. If 3 missed heartbeats → NODE_DOWN event
  5. Reconnect with exponential backoff

Config (japl.toml):
  [node]
  name = "alpha"
  cookie = "secret"
  listen = ":9000"
  connect = ["beta:9001"]    # optional: known peers
```

**Tests:** 20+ (handshake, heartbeat, reconnect, node-down detection)

### Phase D3: Distributed Process Operations

```
runtime/src/
  distributed/
    dpid.ts           # Distributed PID (node + local id)
    router.ts         # Message router (local vs remote)
    remote_spawn.ts   # Spawn process on remote node
    remote_send.ts    # Send message to remote process

PID structure:
  type DistributedPid = {
    node: string;      // node name ("alpha", "beta")
    local: string;     // local process id (UUID)
  }

  // Local PID:  { node: "alpha", local: "abc-123" }
  // Remote PID: { node: "beta",  local: "def-456" }

Message routing:
  send(pid, msg):
    if pid.node === self_node → local delivery (current behavior)
    if pid.node !== self_node → serialize msg → TCP send to pid.node

Remote spawn:
  spawn_remote(node_name, fn, args):
    1. Serialize fn reference + args
    2. Send SPAWN_REQUEST to target node
    3. Target node spawns locally, returns PID
    4. Return DistributedPid to caller

Location transparency:
  // This code works IDENTICALLY whether worker is local or remote:
  let pid = spawn(worker_loop(init))     // local
  let pid = spawn_remote("beta", worker_loop(init))  // remote
  send(pid, Tick("user-1"))              // same API
  // The runtime routes automatically
```

**Tests:** 25+ (remote spawn, remote send, PID routing, network failure)

### Phase D4: Distributed Supervision

```
runtime/src/
  distributed/
    dist_supervisor.ts  # Supervisor that manages remote children
    node_monitor.ts     # React to NODE_DOWN events

Distributed supervisor behavior:
  supervisor DistApp {
    strategy: OneForOne
    child spawn_on("alpha", http_server())
    child spawn_on("beta", worker_pool())
    child spawn_on("alpha", db_connection())
  }

When node "beta" goes down:
  1. All PIDs on "beta" marked as failed
  2. Supervisor receives EXIT for each child on "beta"
  3. Restart strategy applies:
     - If Permanent: try to respawn on another node
     - If Transient: restart only if crash was abnormal
     - If Temporary: don't restart

Node failover:
  supervisor DistApp {
    strategy: OneForOne
    child {
      start: spawn_on("beta", worker_pool()),
      fallback: spawn_on("alpha", worker_pool()),  # if beta unavailable
    }
  }
```

**Tests:** 15+ (cross-node supervision, node-down restart, failover)

### Phase D5: CLI + Developer Experience

```
# Machine A — starts node, listens for connections
japl run --node alpha --listen :9000 src/main.japl

# Machine B — connects to alpha, runs its part
japl run --node beta --connect alpha:9000 src/worker.japl

# Or via japl.toml (zero CLI flags):
[node]
name = "alpha"
listen = ":9000"
cookie = "changeme"
peers = ["beta:9001"]

# Then just:
japl run src/main.japl
```

Minimal config for two-machine setup:

```toml
# Machine A: japl.toml
[node]
name = "alpha"
listen = ":9000"
cookie = "yoneda"

# Machine B: japl.toml
[node]
name = "beta"
listen = ":9001"
connect = ["alpha-host:9000"]
cookie = "yoneda"
```

### Phase D6: End-to-End Proof

A distributed counter that runs across two machines:

```
// counter.japl — runs on alpha
fn counter_loop(n: Int) {
  receive {
    Inc(reply_to) => {
      send(reply_to, n + 1)
      counter_loop(n + 1)
    }
    Get(reply_to) => {
      send(reply_to, n)
      counter_loop(n)
    }
  }
}

fn main() {
  let counter = spawn(counter_loop(0))
  register("counter", counter)
  println("Counter running on " <> node_name())
  sleep_forever()
}
```

```
// client.japl — runs on beta
fn main() {
  let counter = lookup_remote("alpha", "counter")
  send(counter, Inc(self()))
  let val = receive()
  println("Counter value: " <> show(val))
}
```

```bash
# Terminal 1 (Machine A)
$ japl run --node alpha --listen :9000 counter.japl
Counter running on alpha

# Terminal 2 (Machine B)
$ japl run --node beta --connect machineA:9000 client.japl
Counter value: 1
```

---

## Full Update Plan (ALL gaps, prioritized)

### Priority 1: Distribution (Phases D1-D6)
**This is the core promise. Without it, JAPL is just another local FP language.**

| Phase | What | Agent Team | Est |
|-------|------|-----------|-----|
| D1 | Wire protocol + serialization | 2 agents | 1 session |
| D2 | Node identity + connections | 2 agents | 1 session |
| D3 | Distributed process ops | 2 agents | 1 session |
| D4 | Distributed supervision | 1 agent | 1 session |
| D5 | CLI integration | 1 agent | 0.5 session |
| D6 | E2E proof (two machines) | 1 agent | 0.5 session |

### Priority 2: Module System (required for real programs)

| Phase | What | Agents |
|-------|------|--------|
| M1 | Multi-file compilation + import resolution | 2 |
| M2 | Public/private visibility enforcement | 1 |
| M3 | Module-qualified names (Module.fn) | 1 |

### Priority 3: Effect + Linearity Enforcement

| Phase | What | Agents |
|-------|------|--------|
| E1 | Wire effect checker to codegen (reject impure in pure) | 1 |
| E2 | Wire linearity checker to codegen (enforce Owned<T>) | 1 |
| E3 | Exhaustiveness checking for pattern match | 1 |

### Priority 4: Standard Library (real, compiled, tested)

| Phase | What | Agents |
|-------|------|--------|
| S1 | Core + String + List + Map + Set (compile + test) | 2 |
| S2 | IO + File + Json + Http (with foreign bindings) | 2 |
| S3 | Process + Supervisor + Net (TCP/UDP) | 2 |
| S4 | Time + Crypto + Db | 2 |

### Priority 5: Tooling

| Phase | What | Agents |
|-------|------|--------|
| T1 | Formatter (Wadler-Lindig pretty printer) | 1 |
| T2 | REPL | 1 |
| T3 | Package manager (japl add, japl publish) | 2 |
| T4 | LSP (completion, hover, go-to-definition) | 2 |

---

## Execution Order

```
Session N:   D1 (wire protocol) + D2 (nodes)     ← DISTRIBUTION FOUNDATION
Session N+1: D3 (remote ops) + M1 (modules)      ← MAKE IT WORK ACROSS MACHINES
Session N+2: D4 (dist supervision) + D5 (CLI)    ← MAKE IT USABLE
Session N+3: D6 (proof) + E1-E3 (enforcement)    ← PROVE IT + CORRECTNESS
Session N+4: S1-S2 (stdlib core)                  ← REAL PROGRAMS
Session N+5: S3-S4 + T1-T2 (stdlib + tools)      ← POLISH
Session N+6: T3-T4 (package mgr + LSP)            ← ECOSYSTEM
```

After this plan completes: **JAPL is a real distributed functional language.**
