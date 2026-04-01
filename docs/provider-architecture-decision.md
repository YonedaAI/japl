# JAPL Provider Architecture Decision

## Decision: NATS Sidecar Provider (shipped for v1.0)

The JAPL provider ships as a standalone NATS sidecar process. This is the
production distributed execution path for this release.

## Current State

`japl run --distributed` is the proven distributed mode:
- JAPL apps compile to WASM Components exporting `japl:app/handler@0.1.0`
- Process operations (spawn/send/receive) route through NATS to the provider
- External HTTP clients connect via `--http-port` gateway
- Verified with real apps: kvstore, msgqueue (14/14 HTTP gateway tests pass)
- Provider self-pid is session-derived (no caller echo in canonical path)

## wash 2.0.1 Blocker

wash 2.0.1 has a config parsing issue: `build.command is required in wash config`
is emitted regardless of wasmcloud.toml contents. This blocks:
- `wash dev` integration
- `wash build` for JAPL components

This does NOT block `japl run --distributed` or direct WASM component builds
via `wasm-tools component embed + new`, which work correctly.

## What the Sidecar Provides

- Process spawn/send/receive over NATS
- Health endpoint (japl.runtime.health)
- Process reset (japl.runtime.reset)
- Mailbox size limits (10K messages)
- Activity-based cleanup of stale processes
- Self-test on startup

## Why Not Native wasmCloud Provider (this release)

Converting to a native wasmCloud capability provider requires:
1. `wasmcloud-provider-sdk` v0.17.1 -- available on crates.io
2. `wit-bindgen-wrpc` -- for wRPC server stub generation from WIT
3. OCI container packaging -- for `wash app deploy` to load the provider
4. Rewriting NATS subscribe/publish logic to use wRPC server bindings
5. Testing with wash 2.0.1 host lifecycle management (blocked by config parsing)

This conversion is deferred: the sidecar mode is functionally equivalent for
single-host deployments and is proven with real JAPL apps running distributed
with external HTTP clients.

## What Native Provider Would Add

- wasmCloud-managed lifecycle (automatic restart on crash)
- wadm-managed scaling and placement
- Component-to-provider linking via WIT interfaces
- Multi-host distribution managed by wasmCloud lattice
- No separate startup -- deployed as part of the application manifest

## Conversion Path

1. Add wasmcloud-provider-sdk to japl-provider/Cargo.toml
2. Generate wRPC server stubs from wit/japl-runtime/processes.wit
3. Implement the Provider trait (init, shutdown, link handling)
4. Package as OCI artifact
5. Update WADM manifests to reference the OCI provider
6. Test with wash host (after config parsing issue resolved)

## Timeline

Targeted for the next release cycle after wash config parsing issue is resolved
and WIT interface stabilization is complete.
