# Every Bug at Runtime Is a Theorem You Didn't Prove

You have seen this before:

```
TypeError: Cannot read property 'name' of undefined
```

Or this:

```
NullPointerException at com.app.UserService.getUser(UserService.java:47)
```

Or this:

```
panic: runtime error: index out of range [5] with length 3
```

Each of these is a decision that was deferred to runtime. And each one could have been prevented at compile time — with the right type system.

## Runtime Is Proof of Ignorance

At YonedaAI, we published a 25-page paper called *The Minimal Runtime Axiom* (MRA) that formalizes a simple idea: **the optimal runtime for a program is the minimal set of decisions that genuinely cannot be resolved at compile time.** Everything else — every null check, every bounds check, every type tag, every garbage collection cycle — is overhead from incomplete static reasoning.

This is not aspirational. It is a formal claim with a formal proof, grounded in category theory.

## The Hierarchy of Runtime Failures

Consider the decisions most languages defer to runtime, ordered by how easily they can be lifted to compile time:

**Null pointer dereference.** A value might be absent. The fix has been known for decades: option types. `Option<User>` makes the absence explicit in the type. You cannot forget to handle it because the type checker refuses to compile code that does. Languages with option types do not have null pointer exceptions. This is not a runtime decision — it is a compile-time proof that was never written.

**Array out-of-bounds.** You index into an array with a value that exceeds its length. Dependent types can prevent this: `Array<User, 5>` knows its length. An index of type `Fin<5>` (a natural number less than 5) makes out-of-bounds access a type error, not a runtime crash.

**Type errors in dynamic languages.** Calling a method on the wrong type. Passing a string where a number is expected. Static type systems eliminate this entire category. This is well understood.

**Resource leaks.** A file handle opened but never closed. A database connection acquired but never released. Linear types prevent this: a value of linear type must be used exactly once. The type checker ensures every resource is acquired, used, and released — at compile time.

**Serialization failures.** Data sent between services in the wrong format. Type-directed serialization generates the correct encoding from the types. If the types match, the encoding matches. No runtime schema validation needed.

**Concurrency errors.** Data races, deadlocks, message type mismatches. Session types and process types can encode communication protocols in the type system. A message sent to the wrong process is a type error, not a runtime crash.

## The MRA Formally

The Minimal Runtime Axiom defines a lattice of runtime decisions, ordered by their information requirements. At the top: decisions that require genuine runtime information (user input, network responses, hardware state). At the bottom: decisions that are fully determined at compile time.

The axiom states that for any program, there exists a minimal runtime kernel — the set of decisions that provably require runtime information — and every decision outside this kernel represents a failure of the static analysis to capture available information.

We prove this using functors between the category of types and the category of runtime behaviors. The compile-time type system is a functor that maps source programs to runtime behaviors. The more faithful this functor, the smaller the runtime kernel.

## What This Means in Practice

You do not need to adopt category theory to benefit from the MRA. The practical takeaway is a design principle:

**For every runtime check in your code, ask: could this have been a type?**

If the answer is yes, your type system is leaving safety on the table. The check might still be correct. Your program might still work. But you are relying on programmer discipline where you could be relying on mathematical proof.

The MRA does not claim all runtime decisions can be eliminated. User input is inherently runtime. Network failures are inherently runtime. Hardware interrupts are inherently runtime. The axiom identifies the boundary between what must be runtime and what is merely habit.

## JAPL Implements This

JAPL, our programming language, is designed around the MRA. Its type system includes option types (no nulls), linear types (no resource leaks), process types (no message mismatches), and algebraic effects (no untracked side effects). The goal: minimize the runtime kernel to only those decisions that genuinely require runtime information.

The result is a language where most categories of runtime failure are type errors instead.

---

*The full 25-page paper: https://minimal-runtime-axiom.vercel.app*

*Source: https://github.com/YonedaAI/minimal-runtime-axiom*

*JAPL, the language that implements the MRA: https://github.com/YonedaAI/japl*
