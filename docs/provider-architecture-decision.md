# JAPL Provider Architecture Decision

## Decision: Enhanced Sidecar Mode (current release)

The JAPL provider ships as a standalone NATS sidecar process for this release.

## Why Not Native wasmCloud Provider (this release)

Converting to native wasmCloud capability provider requires:
1. `wasmcloud-provider-sdk` v0.17.1 — available on crates.io
2. `wit-bindgen-wrpc` — for wRPC server stub generation from WIT
3. OCI container packaging — for `wash app deploy` to load the provider
4. Rewriting the NATS subscribe/publish logic to use wRPC server bindings
5. Testing with wash 2.0.1 host lifecycle management

This conversion is tracked but deferred: the sidecar mode is functionally
equivalent for single-host deployments and is proven with real JAPL apps
(kvstore, message queue) running distributed with external HTTP clients.

## What the Sidecar Provides
- Process spawn/send/receive over NATS
- Health endpoint (japl.runtime.health)
- Process reset (japl.runtime.reset)
- Mailbox size limits (10K messages)
- Activity-based cleanup of stale processes
- Self-test on startup

## What Native Provider Would Add
- wasmCloud-managed lifecycle (automatic restart on crash)
- wadm-managed scaling and placement
- Component-to-provider linking via WIT interfaces
- Multi-host distribution managed by wasmCloud lattice
- No separate startup — deployed as part of the application manifest

## Conversion Path
1. Add wasmcloud-provider-sdk to japl-provider/Cargo.toml
2. Generate wRPC server stubs from wit/japl-runtime/processes.wit
3. Implement the Provider trait (init, shutdown, link handling)
4. Package as OCI artifact
5. Update WADM manifests to reference the OCI provider
6. Test with wash 2.0.1 host

## Timeline
Targeted for the next release cycle after WIT interface stabilization.
