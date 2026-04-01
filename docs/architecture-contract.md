# JAPL Architecture Contract

## Canonical Distributed Execution

wasmCloud is the canonical distributed execution engine for JAPL.

- `japl run`: local development runtime (embedded wasmtime, OS threads)
- `japl run --distributed`: distributed runtime (WASM + NATS provider)
- `japl deploy`: wasmCloud deployment (component + WADM manifest)

## Current State (v1.0)

The distributed runtime uses a federated model:
- WASM execution is local (wasmtime loads the module)
- Process operations (spawn/send/receive) route through NATS to the JAPL provider
- Each spawned process runs in its own WASM instance with NATS-backed host functions
- External access via HTTP gateway (--http-port)

This is analogous to Erlang's model: code runs on the local node,
messaging is distributed via the transport layer (NATS = distribution protocol).

## wasmCloud Integration

- `japl deploy` compiles to WASM Component, generates WADM manifest
- wasmCloud host is started for orchestration
- The JAPL provider is a NATS sidecar (not yet a native wasmCloud capability)
- Components export `japl:app/handler@0.1.0` for HTTP handling
- wash 2.0.1 config parsing blocks `wash dev` integration (tracked blocker)

## What "Distributed by Default" Means

When JAPL claims distributed semantics, these must be proven via:
1. `japl run --distributed` with real JAPL apps
2. Process messaging through NATS provider
3. External client verification (HTTP or NATS)

Local `japl run` proves local semantics only.

## Distributed Claim Checklist

For any feature claiming distributed behavior:
- [ ] Does it work with `japl run --distributed`?
- [ ] Is process messaging through NATS?
- [ ] Can an external client observe the behavior?
- [ ] Is it tested in the verification suite?
- [ ] Would the release gate fail if this broke?

## Shortcut Rejection

These do NOT prove distributed semantics:
- `japl run` (local only)
- Direct NATS CLI probing without a JAPL app
- `--dry-run` manifests
- Component build alone
