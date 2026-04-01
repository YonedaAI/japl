# JAPL Feature Matrix

## Legend
- ✓ Working and tested
- ~ Partial (works but incomplete)
- ✗ Stub or not implemented
- P Planned

| Category | Feature | Status | Notes |
|----------|---------|--------|-------|
| **Core Language** | ADTs, pattern matching | ✓ | |
| | Closures, HOFs | ✓ | |
| | Type inference | ✓ | Bidirectional |
| | Effect tracking | ✓ | IO, LLM, Process, Fail |
| | Generics | ~ | Works for most cases |
| **Stdlib** | Option/Result/List | ✓ | Full combinator sets |
| | String/Math/IO | ✓ | |
| | Map/Set | ✓ | Int + String keyed |
| | Json/Http | ~ | Encode works, parse stub |
| | Config/Env | ~ | Env reads real vars, Config delegates |
| | File | ~ | Read works, write/exists via host fns |
| | Process/Supervisor | ~ | Spawn works, restart is stub |
| | LLM/Tool/Budget | ~ | Wrappers, not full runtime backing |
| **Runtime** | Local processes | ✓ | OS threads |
| | Mailbox messaging | ✓ | 10K limit, FIFO |
| | Graceful shutdown | ✓ | cmd_rx draining |
| | Distribution CLI | ~ | Flags exist, frame handling TODO |
| | wasmCloud deploy | ~ | Manifest generation, provider sidecar |
| **Tooling** | Package manager | ~ | init/deps, no registry |
| | Benchmarks | ✓ | bench.py |
| | Doc generation | ✓ | gendocs.py |
