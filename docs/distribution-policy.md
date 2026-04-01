# JAPL Distribution Architecture Policy

## Decision: wasmCloud is the Primary Distributed Path

The canonical distributed deployment is:
JAPL source → WASM Component → wasmCloud host + JAPL provider (NATS)

## Custom TCP Distribution (--node-name, --peer)

Status: **Experimental / Development Infrastructure**

The custom TCP distribution layer in `japl/src/runtime/distribution.rs`:
- Provides direct node-to-node TCP connections
- Implements cookie-based authentication
- Has PING/PONG health monitoring
- Frame handling for SEND/SPAWN/EXIT is TODO

This layer is retained for:
- Development and testing of distributed concepts
- Reference implementation of JAPL's distribution protocol
- Fallback when wasmCloud is not available

It is NOT the primary distributed product surface.

## JAPL Provider (NATS)

Status: **Primary distributed path (sidecar mode)**

The JAPL provider in `japl-provider/`:
- Manages process spawn/send/receive over NATS
- Standalone Tokio binary (not yet a native wasmCloud capability)
- Self-tests on startup

Future: Convert to use `wasmcloud-provider-sdk` for native wasmCloud integration.

## Recommended Usage

| Scenario | Approach |
|----------|----------|
| Local development | `japl run` (embedded runtime) |
| HTTP prototyping | `japl serve` (local HTTP) |
| Distributed testing | `japl run --node-name --peer` (custom TCP) |
| Production deployment | `japl deploy` (wasmCloud + NATS) |
