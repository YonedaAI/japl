---
platform: linkedin
type: post
status: pending
tags: [programming-languages, type-theory, language-design]
---
Most programming languages get numbers wrong. Here is why, and what we are doing about it in JAPL.

The standard approach: give developers Int and Float, add implicit promotion, and hope for the best. This is what C, Go, Python, and JavaScript all do in various forms. It works until it does not — silent integer overflow has caused spacecraft failures, financial errors, and security vulnerabilities.

The problem is not technical. It is philosophical. Implicit type promotion violates a principle we call the Minimal Runtime Axiom: every decision deferred to runtime that could have been resolved at compile time is a failure of static reasoning.

When you write x + y where x is Int and y is Float, the language silently converts x to a float. That is a decision made at runtime that the type system could have caught. In a language that claims to be explicit and safe, this is a contradiction.

In JAPL, we are redesigning numbers from first principles:

1. No implicit promotion. Int + Float is a compile error. You write to_float(x) + y. The conversion is visible, intentional, auditable.

2. Overflow is a failure, not a feature. Silent wraparound on i64 is how distributed systems silently corrupt data across nodes. JAPL panics on overflow in debug mode and uses checked arithmetic in release — consistent with our principle that failures are normal and typed.

3. Byte as a first-class type. A language designed for distributed systems needs u8 for binary protocols, network packets, and cryptographic operations. Not as a library. As a primitive.

4. Numeric traits for generic code. A Num trait lets you write fn sum(list: List(a)) -> a where Num(a) — one function that works for both Int and Float, resolved entirely at compile time via monomorphization.

5. Literal syntax that respects the developer. 1_000_000 for readability. 0xFF for hex. 0b1010 for binary. 1.5e10 for scientific notation. These are not luxuries. They are how systems programmers think.

The deeper insight: number representation is where a language's values become visible. If you claim to be explicit but silently promote types, if you claim to be safe but silently overflow, if you claim to be for distributed systems but lack a byte type — the numbers expose the gap between what a language says and what it does.

Every non-omega decision at runtime is a theorem waiting to be proven. Implicit numeric promotion is one of those theorems.

https://github.com/YonedaAI/japl
https://minimal-runtime-axiom.vercel.app

#ProgrammingLanguages #TypeTheory #LanguageDesign #Rust #Erlang #FunctionalProgramming #SoftwareEngineering #CompilerDesign
