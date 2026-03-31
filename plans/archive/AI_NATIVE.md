# JAPL AI-Native Abstractions

> Making "AI-native" a language feature, not branding.

## The Reviewer's Challenge

> "To earn the AI-native label, JAPL needs first-class abstractions for structured model I/O, tool contracts, provenance, quotas/budgets, deterministic replay, and effect-tracked nondeterministic inference."

## What "AI-Native" Means as Language Features

Just like SQL is data-native (queries are first-class), JAPL should be AI-native: **LLM calls, tool execution, and agent coordination are first-class language constructs with type safety and effect tracking.**

## The Six Abstractions

### 1. LLM Call as an Effect

LLM inference is nondeterministic I/O. In JAPL, it should be an effect — like IO or Process.

```japl
// LLM is an effect, tracked by the type system
fn summarize(text: String) -> ![LLM] String {
  llm("Summarize this: " <> text)
}

// Pure functions CANNOT call LLMs
fn clean(s: String) -> String {
  llm("fix: " <> s)  // COMPILE ERROR: LLM effect in pure function
}

// Effect composition
fn analyze(doc: String) -> ![IO, LLM] Report {
  let summary = summarize(doc)   // LLM effect
  write_file("report.txt", summary)  // IO effect
  Report(summary)
}
```

**Why this matters:** You can statically see which functions call LLMs. No hidden API calls. No surprise latency. No unexpected costs.

### 2. Structured Model I/O (Typed Prompts + Typed Outputs)

Prompts and outputs are typed, not raw strings.

```japl
// Define a structured prompt type
type SentimentPrompt = {
  text: String,
  options: List(String)
}

// Define a structured output type
type Sentiment =
  | Positive(Float)
  | Negative(Float)
  | Neutral

// LLM call with typed input and output
fn classify(text: String) -> ![LLM] Sentiment {
  llm_structured(
    SentimentPrompt({ text: text, options: ["positive", "negative", "neutral"] }),
    type: Sentiment  // compiler generates JSON schema from JAPL type
  )
}

fn main() -> ![IO, LLM] Unit {
  match classify("JAPL is amazing") {
    Positive(score) => println("Positive: " <> show(score))
    Negative(score) => println("Negative: " <> show(score))
    Neutral => println("Neutral")
  }
}
```

**Why this matters:** The compiler generates JSON schemas from JAPL types. Invalid LLM responses are caught at deserialization, not three layers deep in application code.

### 3. Tool Contracts

Agent tools are typed functions with explicit capabilities.

```japl
// A tool is a function with a contract
tool search_web(query: String) -> ![Net] List(SearchResult) {
  // implementation
}

tool read_file(path: String) -> ![IO] String {
  // implementation
}

// An agent declares which tools it can use
agent Researcher {
  tools: [search_web, read_file]

  fn run(question: String) -> ![LLM, Net, IO] Answer {
    let results = search_web(question)
    let content = results |> map(fn(r) { read_file(r.url) })
    llm_structured("Synthesize: " <> join(content), type: Answer)
  }
}
```

**Why this matters:** Tool capabilities are visible in the type system. An agent can't use a tool it wasn't given. Effects track what tools can do (Net, IO, DB). The supervisor can restrict capabilities per agent.

### 4. Budget / Quota Types

LLM usage has costs. Make them first-class.

```japl
// Budget is a resource type (linear — consumed, not copied)
type Budget = resource {
  max_tokens: Int,
  max_cost_cents: Int,
  used_tokens: Int,
  used_cost_cents: Int
}

// LLM calls consume budget
fn summarize(text: String, budget: Owned(Budget)) -> ![LLM] (String, Owned(Budget)) {
  let (result, tokens_used) = llm_with_tracking("Summarize: " <> text)
  let updated_budget = consume(budget, tokens_used)
  (result, updated_budget)
}

// Budget enforcement is compile-time via linearity
fn run_agent(budget: Owned(Budget)) -> ![LLM] Report {
  let (summary, budget) = summarize(doc, budget)    // budget consumed
  let (analysis, budget) = analyze(summary, budget)  // budget consumed again
  // summarize(doc2, budget) — would work, budget still owned
  Report(summary, analysis)
}

// Budget exhaustion is a typed failure
type BudgetError =
  | TokensExhausted(Int)
  | CostExhausted(Int)
```

**Why this matters:** You can't accidentally spend unlimited money on LLM calls. Budget is a linear resource — passed explicitly, consumed on each call, compiler ensures it's not duplicated.

### 5. Deterministic Replay

All nondeterministic decisions (LLM responses, tool results) can be recorded and replayed.

```japl
// Replay mode: record all LLM calls
fn run_with_replay(input: String) -> ![LLM, Replay] Output {
  let result = llm("Process: " <> input)
  // In record mode: saves (prompt, response) to replay log
  // In replay mode: returns cached response instead of calling LLM
  result
}

// Test with recorded responses
test "agent produces correct output" {
  with_replay("test_fixtures/agent_run_1.replay") {
    let output = run_with_replay("test input")
    assert output.quality > 0.8
  }
}
```

**Why this matters:** You can test AI workflows deterministically. Record a run, replay it in tests. No flaky tests from LLM nondeterminism.

### 6. Agent as Supervised Process

Agents ARE JAPL processes. Supervision handles LLM failures.

```japl
// An agent is just a process with tools and a budget
fn agent_loop(tools: ToolSet, budget: Owned(Budget)) {
  receive {
    Task(question, reply_to) =>
      match run_with_budget(question, tools, budget) {
        Ok((answer, remaining_budget)) =>
          send(reply_to, Answer(answer))
          agent_loop(tools, remaining_budget)
        Err(BudgetExhausted) =>
          send(reply_to, Error("budget exhausted"))
          // process exits, supervisor can restart with new budget
      }
    Shutdown =>
      done()
  }
}

// Supervisor manages agent fleet
supervisor AgentPool {
  strategy: OneForOne
  child agent_loop(web_tools, Budget(10000, 500))
  child agent_loop(code_tools, Budget(50000, 2000))
  child agent_loop(data_tools, Budget(20000, 1000))
}
// If an agent crashes (LLM error, timeout, budget exhaustion),
// supervisor restarts it with a fresh budget.
```

**Why this matters:** LLM calls fail. APIs time out. Budgets run out. JAPL's supervision model handles all of this — the same way Erlang handles telecom failures. An agent crash is just a process crash. Restart and continue.

## How This Connects to JAPL's Existing Features

```
JAPL Feature              AI Application
──────────────            ──────────────
Effects (LLM, IO, Net)    Track which functions call LLMs
Pattern matching          Route on structured LLM outputs
Typed failures            Handle LLM errors, budget exhaustion
Processes                 Each agent is a supervised process
Supervision               Restart crashed agents
Distribution              Spread agents across machines
Ownership                 Budget as linear resource (can't duplicate)
Message passing           Agent coordination via typed messages
```

JAPL already has every building block. The AI abstractions are compositions of existing features, not new runtime mechanisms.

## Implementation Plan

### Phase A: LLM Effect + Structured I/O

```
1. Add LLM to the effect system: Pure | IO | LLM | Process | Fail | Net
2. Add llm() builtin that takes a string prompt, returns string
3. Add llm_structured() that takes prompt + output type, returns typed value
4. Compiler generates JSON schema from JAPL ADTs
5. Runtime: LLM effect → host function → HTTP to OpenAI/Anthropic API
```

Agent team: 2 agents
- Agent 1: Effect system update + llm() host function in Rust runtime
- Agent 2: JSON schema generation from JAPL types + llm_structured()

### Phase B: Tool Contracts + Agent Type

```
1. Add `tool` keyword (syntactic sugar for effectful function with metadata)
2. Add `agent` keyword (process with declared tools)
3. Tool capability checking: agent can only use declared tools
4. Runtime: tool registry, capability enforcement
```

Agent team: 1 agent

### Phase C: Budget Types

```
1. Budget as a resource type (uses existing Owned<T> linearity)
2. llm_with_tracking() returns (result, tokens_used)
3. Budget consumption tracked via linear type system
4. Budget exhaustion as typed failure
```

Agent team: 1 agent

### Phase D: Replay

```
1. Replay effect: records nondeterministic decisions
2. Record mode: save (input, output) pairs to file
3. Replay mode: return cached outputs
4. Test integration: with_replay("fixture.replay") { ... }
```

Agent team: 1 agent

### Phase E: Agent Supervision Demo

```
1. Multi-agent application using all features
2. Agent fleet with supervision, budgets, tools
3. Distributed across nodes
4. Proves: JAPL is AI-native, not by branding, but by design
```

Agent team: 1 agent (application developer)

## Total: 6 agents across 5 phases

## Why This Makes JAPL Unique

No other language has:
- **LLM calls as a tracked effect** (you can see them in the type signature)
- **Budget as a linear resource** (compiler prevents overspending)
- **Structured I/O from language types** (no separate schema language)
- **Agent supervision** (LLM failures handled like Erlang handles telecom failures)
- **Deterministic replay** (test AI workflows without calling LLMs)

Python + LangChain gives you agents. JAPL gives you **type-safe, budget-controlled, supervised, distributed, replayable agents.**

That's not branding. That's a real language feature set.
