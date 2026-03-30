# Budget Types Design [PLANNED]

## Overview

Budget tracking via linear types ensures that LLM token consumption is statically tracked and cannot be accidentally dropped or exceeded. A `Budget` is an `Owned<T>` resource that must be explicitly consumed.

## Core Design

### Budget as an Owned Resource

```japl
type Budget = Owned(Int)  -- linear: must be consumed exactly once

fn create_budget(tokens: Int) -> Budget {
  Owned(tokens)
}

fn llm_with_tracking(prompt: String, budget: Budget) -> (String, Int, Budget) {
  -- Returns: (result, tokens_used, remaining_budget)
  let Owned(remaining) = budget
  let (result, used) = llm_raw(prompt)
  if used > remaining {
    fail("Budget exceeded: used " <> show(used) <> " of " <> show(remaining))
  }
  (result, used, Owned(remaining - used))
}
```

### Usage Pattern

```japl
fn main() {
  let budget = create_budget(1000)

  let (answer1, used1, budget) = llm_with_tracking("What is JAPL?", budget)
  println("Used " <> show(used1) <> " tokens")

  let (answer2, used2, budget) = llm_with_tracking("What is WebAssembly?", budget)
  println("Used " <> show(used2) <> " tokens")

  -- Budget must be explicitly consumed or returned
  let Owned(remaining) = budget
  println("Remaining budget: " <> show(remaining))
}
```

### Compiler Enforcement

The compiler enforces budget consumption through the existing linear type system:

1. **Owned values cannot be dropped** -- If a `Budget` goes out of scope without being destructured, the compiler emits an error.
2. **Owned values cannot be duplicated** -- A budget cannot be used twice. Each `llm_with_tracking` call consumes the old budget and returns a new one.
3. **Effect tracking** -- `llm_with_tracking` carries the `LLM` effect, so budget-tracked LLM calls are still visible in the type system.

### Hierarchical Budgets

```japl
fn split_budget(budget: Budget, amount: Int) -> (Budget, Budget) {
  let Owned(total) = budget
  if amount > total {
    fail("Cannot split: requested " <> show(amount) <> " from " <> show(total))
  }
  (Owned(amount), Owned(total - amount))
}

fn agent_with_budget(task: String, budget: Budget) -> (String, Budget) {
  let (sub_budget, remaining) = split_budget(budget, 200)
  let (result, _used, sub_budget) = llm_with_tracking(task, sub_budget)
  let Owned(leftover) = sub_budget
  -- Return unused tokens to parent budget
  let Owned(parent_remaining) = remaining
  (result, Owned(parent_remaining + leftover))
}
```

### Implementation Plan

1. **Phase 1**: Define `Budget` as a library type using existing `Owned<T>` linear types
2. **Phase 2**: Add `llm_with_tracking` as a compiler builtin alongside `llm`
3. **Phase 3**: Add budget-aware process spawning (pass budgets across process boundaries)
4. **Phase 4**: Add compile-time budget inference for static analysis

### Relationship to Effects

Budget interacts with the effect system:

```japl
fn safe_query(prompt: String, budget: Budget) -> ![LLM] (String, Budget) {
  llm_with_tracking(prompt, budget)
}
```

The `LLM` effect annotation ensures callers know this function makes LLM calls, while the linear `Budget` type ensures the token cost is tracked.
