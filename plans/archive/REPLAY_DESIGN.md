# Deterministic Replay Design [PLANNED]

## Overview

Replay records nondeterministic decisions (LLM responses, process scheduling, external I/O) so that programs can be re-executed deterministically. This enables debugging, testing, and auditing of AI agent behavior.

## Replay as an Effect

```japl
type ReplayMode =
  | Record(FilePath)
  | Replay(FilePath)
  | Live

fn with_replay(mode: ReplayMode, body: fn() -> ![LLM, Replay] a) -> a {
  match mode with
    | Record(path) =>
        -- Intercept LLM calls, record prompts + responses
        let result = body()
        flush_replay_log(path)
        result
    | Replay(path) =>
        -- Replace LLM calls with recorded responses
        let log = load_replay_log(path)
        replay_with_log(log, body)
    | Live =>
        body()
}
```

## Record Mode

In record mode, every nondeterministic operation is intercepted and logged:

```japl
fn main() ![LLM, Replay] {
  with_replay(Record("session.replay"), fn() {
    let answer = llm("What is 2 + 2?")
    println(answer)
  })
}
```

The runtime wraps each LLM call:
1. Forward the prompt to the actual LLM API
2. Capture the response
3. Append `(prompt, response, timestamp)` to the replay log
4. Return the response to the program

## Replay Mode

In replay mode, LLM calls are served from the log:

```japl
fn main() ![Replay] {
  -- Note: no LLM effect needed in replay mode
  with_replay(Replay("session.replay"), fn() {
    let answer = llm("What is 2 + 2?")
    -- Returns the recorded response, no API call made
    println(answer)
  })
}
```

## File Format

Replay logs use a line-delimited JSON format (`.replay`):

```json
{"version": 1, "recorded_at": "2026-03-30T12:00:00Z", "program": "agent.japl"}
{"seq": 0, "kind": "llm", "prompt": "What is 2 + 2?", "response": "4", "tokens": 12, "latency_ms": 450}
{"seq": 1, "kind": "llm", "prompt": "Explain further", "response": "2 + 2 equals 4 because...", "tokens": 85, "latency_ms": 820}
{"seq": 2, "kind": "spawn", "process_id": 1, "function": "worker_loop"}
{"seq": 3, "kind": "receive", "process_id": 0, "message": {"tag": "Result", "fields": ["done"]}}
```

### Entry Types

| Kind      | Fields                                        | Description                    |
|-----------|-----------------------------------------------|--------------------------------|
| `llm`     | prompt, response, tokens, latency_ms          | LLM API call                   |
| `spawn`   | process_id, function                          | Process creation               |
| `send`    | from_pid, to_pid, message                     | Message passing                |
| `receive` | process_id, message                           | Message receipt                |
| `schedule`| process_id, ticks                             | Scheduler decision             |

## Test Integration

Replay enables deterministic testing of AI agents:

```japl
test "agent answers correctly" {
  with_replay(Replay("fixtures/agent_qa.replay"), fn() {
    let agent = spawn(fn() { agent_loop() })
    send(agent, Ask("What is JAPL?", self()))
    let response = receive()
    assert(response == "JAPL is a typed functional language")
  })
}
```

### Recording Test Fixtures

```bash
# Record a session
JAPL_REPLAY=record:fixtures/agent_qa.replay japl run agent.japl

# Run tests using recorded session
japl test  # automatically uses replay files in fixtures/
```

## Implementation Plan

1. **Phase 1**: Implement replay log writer/reader in the runtime (Rust)
2. **Phase 2**: Add `Replay` effect to the type system
3. **Phase 3**: Intercept LLM host calls in record mode
4. **Phase 4**: Serve recorded responses in replay mode
5. **Phase 5**: Add process scheduling replay for full determinism
6. **Phase 6**: CLI integration (`japl run --record`, `japl run --replay`)

## Relationship to Other Features

- **Budget**: Replay logs include token counts, enabling budget analysis of recorded sessions
- **Tool contracts**: Tool calls are recorded alongside LLM calls
- **Supervision**: Process lifecycle events are logged for debugging supervisor behavior
