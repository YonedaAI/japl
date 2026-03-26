# Meetup & Hackerspace Talk Proposals

## Chicago Haskell/FP Meetup

**Talk Title:** JAPL: What Happens When You Design a Language from Category Theory Up

**Duration:** 30 minutes + Q&A

**Abstract:**

JAPL is a functional programming language designed from mathematical first principles. Rather than starting with syntax and adding types later, we started with category theory and type theory, then derived a language that satisfies formal properties: purity by default, resource safety by construction, typed concurrency by design.

This talk covers the design journey from axioms to a self-hosting compiler, the surprises along the way (linear types and processes interact in non-obvious ways), and the lessons for anyone building or extending a type system.

Live demo: compiling JAPL with JAPL.

---

## Papers We Love Chicago

**Talk Title:** The Yoneda Constraint: One Axiom, Four Impossibility Theorems

**Duration:** 45 minutes + discussion

**Abstract:**

We present a 23-page paper proving that Godel's incompleteness, the quantum measurement problem, the compiler bootstrap paradox, and AI alignment impossibility are all instances of a single categorical axiom. The proof uses the Yoneda Lemma — one of the most fundamental results in category theory — applied to self-referential systems.

This talk walks through the proof at a level accessible to anyone comfortable with basic category theory, then discusses the implications for AI safety and programming language design.

---

## Local Hackerspace Lightning Talk

**Talk Title:** Self-Hosting in 1,495 Lines: How a Language Compiles Itself

**Duration:** 10 minutes

**Abstract:**

Lightning talk on what it means for a programming language to compile itself, why it matters, and the specific bootstrap paradox you hit when you try. With a live demo of JAPL compiling JAPL.

---

## Recurse Center Community

**Post for RC Zulip or mailing list:**

Sharing three research projects that might interest Recursers:

1. **JAPL** — A self-hosting FP language compiling to TypeScript + C. If you're interested in compilers, type systems, or language design, the 7 papers and full source are open: https://japl-nine.vercel.app

2. **The Minimal Runtime Axiom** — A formal argument that runtime overhead = incomplete static reasoning. Every null check is a missed proof. https://minimal-runtime-axiom.vercel.app

3. **The Yoneda Constraint** — One categorical axiom unifying Godel, quantum measurement, bootstrap, and AI alignment. https://yoneda-constraint.vercel.app

All open source. Happy to pair on any of it or discuss the ideas.
