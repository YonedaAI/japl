# Stack Exchange Posts

## Stack Overflow: Self-Answered Q&A

**Title:** How to design a type system that minimizes runtime decisions?

**Tags:** type-theory, programming-languages, compiler-design, static-analysis, type-systems

**Question:**

I'm designing a statically-typed programming language and want to minimize the number of decisions deferred to runtime. For example:

- Null pointer checks → option types
- Array bounds checks → dependent-length types
- Resource cleanup → linear types
- Dynamic dispatch → monomorphization

Is there a systematic framework for identifying which runtime decisions can be lifted to compile time, and for determining the minimal set of decisions that genuinely require runtime information?

**Answer:**

Yes. We formalize this in a framework called the **Minimal Runtime Axiom** (MRA).

The key insight: runtime decisions form a lattice ordered by their information requirements. At the top are decisions requiring genuinely external input (user input, network data, hardware state). At the bottom are decisions fully determined by the source code.

**The framework has three steps:**

**Step 1: Classify runtime decisions by information source.**

| Runtime Decision | Information Source | Can Lift? |
|---|---|---|
| Null check | Value absence | Yes — option types |
| Bounds check | Array length vs index | Yes — dependent types |
| Resource cleanup | Resource lifetime | Yes — linear types |
| Dynamic dispatch | Receiver type | Partially — monomorphization |
| GC | Memory liveness | Partially — region inference |
| User input parsing | External data | No — genuinely runtime |
| Network failure handling | External state | No — genuinely runtime |

**Step 2: For each "Yes" or "Partially" decision, identify the type system feature that captures the information statically.**

- **Option types** capture value presence/absence: `Option<User>` forces handling of the absent case at every use site.
- **Dependent types** (or refinement types) capture numeric relationships: `Array<T, N>` with index type `Fin<N>` makes out-of-bounds a type error.
- **Linear types** capture resource lifetimes: a linear value must be used exactly once, ensuring acquire-use-release.
- **Effect types** capture side effects: `fn read() -> String with IO` makes effects visible in the type.
- **Process types** capture message protocols: `Process<Msg>` ensures only valid messages are sent.

**Step 3: The minimal runtime kernel is what remains — decisions depending on genuinely external information.**

The categorical formalization: the type system is a functor from source programs to behavioral specifications. The faithfulness of this functor determines the runtime kernel size. A more faithful functor = smaller runtime = fewer runtime failures.

**Practical implementation:**

We built a language called JAPL that implements this framework. It combines option types (no nulls), linear types (no resource leaks), process types (no message mismatches), and algebraic effects (no untracked side effects).

The result: most categories of runtime failure become compile-time type errors.

Full 25-page paper: https://minimal-runtime-axiom.vercel.app
JAPL source: https://github.com/YonedaAI/japl

---

## CS Theory Stack Exchange (cstheory.stackexchange.com)

**Title:** Is there a categorical axiom that subsumes both Godel's incompleteness and the halting problem?

**Tags:** lo.logic, ct.category-theory, computability

**Question:**

Godel's incompleteness theorems and the undecidability of the halting problem are often presented as related but distinct results. Both involve self-reference, and both establish limits on what a system can determine about itself.

Has anyone formalized a single categorical axiom from which both results (and potentially other self-referential impossibility results) follow as instances?

Specifically, I am looking for a result of the form: "For any endofunctor F on a category C satisfying [conditions], F cannot be simultaneously [property 1] and [property 2]" — where instantiating C and F appropriately yields Godel, halting, and potentially other impossibility results.

We have published a candidate formalization called the Yoneda Constraint (https://yoneda-constraint.vercel.app) that derives such an axiom from the Yoneda Lemma, showing that Godel's incompleteness, the halting problem, the quantum measurement problem, and the AI alignment problem are all instances. But I am curious whether similar unified treatments exist in the literature, or whether there are known obstacles to such a unification.

---

## Programming Languages Stack Exchange

**Title:** Design tradeoffs in dual-backend compilation (TypeScript + C from one type checker)

**Tags:** compilers, language-design, code-generation, type-systems

**Question:**

We built a compiler for JAPL (https://github.com/YonedaAI/japl) that compiles the same source language to both TypeScript and C, sharing a single type checker.

Key design decisions we made:

1. **Target-independent IR after type checking.** The type checker produces a typed AST that both backends consume. This forced us to avoid target-specific assumptions in the type system.

2. **No target-specific types.** Every type in the language has a representation in both TypeScript and C. This excluded some features (e.g., JS Promises as a built-in type).

3. **Uniform memory model.** The language uses linear types for resources, which map to both TypeScript's GC (with explicit cleanup) and C's manual management.

4. **Separate runtime libraries per target.** The standard library has per-target implementations behind a common interface.

Questions for the community:

- What pitfalls have others encountered with multi-target compilation?
- How do mature multi-target compilers (e.g., Haxe, Nim) handle type system features that exist in one target but not another?
- Is there an established approach to testing equivalence of codegen across targets?
