# JAPL Distributed Hello World

Demonstrates the wasmCloud deployment path for JAPL process-based apps.

## Local execution

```bash
japl run apps/distributed/hello_distributed.japl
```

## Component compilation

```bash
japl build apps/distributed/hello_distributed.japl --target component --out /tmp
```

## Distributed deployment (wasmCloud)

```bash
japl deploy apps/distributed/hello_distributed.japl
```

## Architecture

- **Local mode**: embedded wasmtime runtime with OS threads for processes
- **Component mode**: compiles to WASM Component for portable deployment
- **Deployed mode**: wasmCloud host + JAPL provider (processes over NATS)

The same source code runs in all three modes. The `spawn`/`send`/`receive`
primitives are handled by the embedded runtime locally, and by the JAPL
provider (japl-provider) when deployed to wasmCloud.
