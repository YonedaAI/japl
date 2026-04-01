# JAPL Feature Matrix

## Legend
- ✓ Working and tested
- ~ Partial (works but incomplete)
- ✗ Stub or not implemented
- P Planned

## Proof Levels
- **LOCAL**: Proven with `japl run` (local wasmtime, OS threads)
- **PROVIDER**: Proven with NATS provider mechanics (spawn/send/receive over NATS)
- **DISTRIBUTED**: Proven with `japl run --distributed` end-to-end with real apps
- **DEPLOYED**: Proven with wasmCloud deployment (`japl deploy` + wash)

| Category | Feature | Status | Proof Level | Verification | Notes |
|----------|---------|--------|-------------|-------------|-------|
| **Core Language** | ADTs, pattern matching | ✓ | LOCAL | stdlib tests (30 modules) | |
| | Closures, HOFs | ✓ | LOCAL | closures, higher_order tests | |
| | Type inference | ✓ | LOCAL | 28 negative checker tests | Bidirectional |
| | Pid type | ✓ | LOCAL | negative checker tests | Distinct from Int; arithmetic on Pid is a type error |
| | Effect tracking | ✓ | LOCAL | implicit in stdlib tests | IO, LLM, Process, Fail |
| | Generics | ~ | LOCAL | generics_fn test | Works for most cases |
| **Stdlib** | Option/Result/List | ✓ | LOCAL | stdlib tests (30 modules) | Full combinator sets |
| | String/Math/IO | ✓ | LOCAL | stdlib tests (30 modules) | |
| | Map/Set | ✓ | LOCAL | stdlib tests (30 modules) | Int + String keyed |
| | Json/Http | ~ | LOCAL | stdlib tests (30 modules) | Encode works, parse stub |
| | Config/Env | ✓ | LOCAL | env_get_str host function | Env reads real vars, Config delegates |
| | File | ✓ | LOCAL | read/write/exists tests | |
| | Process/Supervisor | ~ | LOCAL | apps (kvstore, msgqueue, agents) | Spawning works; restart not implemented (no monitor/link) |
| | LLM | ~ | LOCAL | stdlib/LLM test | Basic JSON prefix validation; no schema enforcement |
| | Tool | ~ | LOCAL | stdlib/Tool test | Simulated execution; no real dispatch backend |
| | Budget/Replay/Provenance | ~ | LOCAL | stdlib tests | Wrappers, not full runtime backing |
| **Runtime** | Local processes | ✓ | LOCAL | apps (kvstore, msgqueue, agents) | OS threads |
| | Mailbox messaging | ✓ | LOCAL | process spawn/send/receive tests | 10K limit, FIFO |
| | Graceful shutdown | ✓ | LOCAL | implicit in app tests | cmd_rx draining |
| | Distributed mode | ✓ | DISTRIBUTED | --distributed + HTTP gateway test | kvstore + msgqueue proven |
| | HTTP gateway | ✓ | DISTRIBUTED | test_http_kvstore.py (14 tests) | External client access proven |
| | wasmCloud provider | ~ | PROVIDER | component build + NATS proof | Sidecar mode (NATS); native wasmCloud provider deferred |
| | wasmCloud deploy | ~ | BLOCKED | component build only | wash 2.0.1 config parsing blocks `wash dev`/`wash build` |
| **Tooling** | Package manager | ~ | LOCAL | japl init + deps smoke | init/deps, no registry |
| | Benchmarks | ✓ | LOCAL | bench.py | |
| | Doc generation | ✓ | LOCAL | gendocs.py | |
