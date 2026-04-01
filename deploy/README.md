# JAPL Deploy

## Manifests

Generated manifests describe the JAPL component for wasmCloud deployment.

## Provider Requirement

The JAPL provider is a standalone NATS sidecar that manages process
spawn/send/receive. It must be running before deployment:

    cd japl-provider && cargo run

## Commands

    # Preview the manifest
    japl deploy --dry-run app.japl

    # Deploy to wasmCloud
    japl deploy app.japl

    # Local-only (no wasmCloud required)
    japl deploy --local app.japl
