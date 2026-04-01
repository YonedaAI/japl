# JAPL Feature Matrix

## Legend
- ✓ Working and tested
- ~ Partial (works but incomplete)
- ✗ Stub or not implemented
- P Planned

| Category | Feature | Status | Verification | Notes |
|----------|---------|--------|-------------|-------|
| **Core Language** | ADTs, pattern matching | ✓ | stdlib tests (30 modules) | |
| | Closures, HOFs | ✓ | closures, higher_order tests | |
| | Type inference | ✓ | 28 negative checker tests | Bidirectional |
| | Pid type | ✓ | negative checker tests | Distinct from Int; arithmetic on Pid is a type error |
| | Effect tracking | ✓ | implicit in stdlib tests | IO, LLM, Process, Fail |
| | Generics | ~ | generics_fn test | Works for most cases |
| **Stdlib** | Option/Result/List | ✓ | stdlib tests (30 modules) | Full combinator sets |
| | String/Math/IO | ✓ | stdlib tests (30 modules) | |
| | Map/Set | ✓ | stdlib tests (30 modules) | Int + String keyed |
| | Json/Http | ~ | stdlib tests (30 modules) | Encode works, parse stub |
| | Config/Env | ✓ | env_get_str host function | Env reads real vars, Config delegates |
| | File | ✓ | read/write/exists tests | |
| | Process/Supervisor | ~ | apps (kvstore, msgqueue, agents) | Spawning works; restart not implemented (no monitor/link) |
| | LLM | ~ | stdlib/LLM test | Basic JSON prefix validation; no schema enforcement |
| | Tool | ~ | stdlib/Tool test | Simulated execution; no real dispatch backend |
| | Budget/Replay/Provenance | ~ | stdlib tests | Wrappers, not full runtime backing |
| **Runtime** | Local processes | ✓ | apps (kvstore, msgqueue, agents) | OS threads |
| | Mailbox messaging | ✓ | process spawn/send/receive tests | 10K limit, FIFO |
| | Graceful shutdown | ✓ | implicit in app tests | cmd_rx draining |
| | Distributed mode | ✓ | --distributed + HTTP gateway test | |
| | HTTP gateway | ✓ | test_http_kvstore.py (14 tests) | |
| | wasmCloud provider | ~ | component build + NATS proof | Sidecar mode (NATS); native wasmCloud provider deferred |
| | wasmCloud deploy | ~ | component build + --dry-run | Fails closed without wasmCloud; --local for explicit local mode |
| **Tooling** | Package manager | ~ | japl init + deps smoke | init/deps, no registry |
| | Benchmarks | ✓ | bench.py | |
| | Doc generation | ✓ | gendocs.py | |
