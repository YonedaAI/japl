# JAPL Provider Architecture Decision

## Decision: Sidecar Mode (current wave)

The JAPL provider remains a standalone NATS sidecar for this release.

## Rationale

- **wasmcloud-provider-sdk (v0.17.1) requires WIT-based wrpc bindings.** Custom
  capability providers must implement the provider SDK's `Provider` trait, handle
  link-definition callbacks, health check responses, and host-data initialization.
  The current provider is a plain Tokio binary that subscribes to raw NATS
  subjects — it shares none of that scaffolding.
- **Custom WIT interfaces are supported but require wrpc transport plumbing.**
  wasmCloud 1.x routes component-to-provider calls through wRPC (WIT-aware RPC
  over NATS). The provider must generate server stubs from the WIT interface
  (`japl:runtime/processes`) using `wit-bindgen-wrpc` and implement those stubs
  instead of hand-rolled NATS subject parsing. This is a non-trivial rewrite of
  the message dispatch layer.
- **The sidecar model is functionally equivalent for single-host deployments.**
  Both the sidecar and a native provider communicate over NATS. The component
  currently reaches the provider through raw NATS request/reply subjects, which
  works identically whether the provider is managed by wasmCloud or started
  separately.
- **Converting to a native provider is tracked for a future wave** once the WIT
  interface stabilizes (especially `receive()` PID injection and closure
  execution).

## What Sidecar Mode Provides

- Process spawn / send / receive over NATS request/reply
- Works alongside a wasmCloud host on the same NATS cluster
- Must be started separately: `cd japl-provider && cargo run`
- Self-test on startup validates the message path

## What Native Provider Would Add

- Managed lifecycle by wasmCloud (start, stop, health checks)
- Automatic discovery and link-definition handling between components and the
  provider — no manual NATS subject wiring
- Multi-host distribution managed by wadm (scalable deployments)
- Proper call-context propagation (solves the `self-pid` / `receive()` PID gap)

## Path to Native Provider

1. **Add `wasmcloud-provider-sdk` dependency** to `japl-provider/Cargo.toml` and
   implement the `Provider` trait (shutdown, health check, link put/delete).
2. **Generate wRPC server stubs** from `wit/japl-runtime/world.wit` using
   `wit-bindgen-wrpc`. Replace the manual NATS subject dispatch with generated
   handler implementations.
3. **Inject call context** so `receive()` and `self-pid()` can identify the
   calling component instance without an explicit PID in the subject.
4. **Package as an OCI artifact** or filesystem-based provider archive so wadm
   can deploy it declaratively.
5. **Update `deploy/japl-provider.wadm.yaml`** to reference the native provider
   image instead of expecting a separate sidecar process.
6. **Validate with `wash dev`** end-to-end: component links to provider, spawn /
   send / receive work through wRPC.

## Timeline

Estimated: Phase 16 or later, after the WIT interface is stable and
`receive()` PID injection is resolved. Blocked on finalizing typed process
semantics (Phase 15).
