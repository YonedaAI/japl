# JAPL Process Type Safety

## Pid Type
- `spawn(fn() -> Unit)` returns `Pid`, not `Int`
- `send(pid, msg)` requires `Pid` as first argument
- `self_pid()` / `self()` returns `Pid`
- `Pid` is NOT compatible with `Int` — no implicit conversion

## What This Prevents
- Arithmetic on process IDs: `pid + 1` is a type error
- Passing raw integers as PIDs: `send(42, msg)` is a type error
- Storing PIDs in Int-typed variables: `let x: Int = spawn(...)` is a type error

## Runtime Representation
At the WASM level, Pid IS an i64 (same as Int). The distinction is purely
at the type checker level. The lowerer and WAT emitter treat Pid and Int
identically as i64 values.

## Process-Related Builtins
| Builtin | Signature | Returns |
|---------|-----------|---------|
| spawn | fn(fn() -> Unit) -> Pid | New process PID |
| send | fn(Pid, T) -> Unit | Unit |
| receive | fn() -> T | Message value |
| self_pid | fn() -> Pid | Current process PID |
| self | fn() -> Pid | Alias for self_pid |
