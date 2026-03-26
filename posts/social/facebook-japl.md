We built a programming language from scratch, and then we used it to compile itself.

JAPL stands for Just Another Programming Language, but there's nothing "just" about it. Here's what went into it:

7 research papers (over 160 pages of formal specification and proofs)
A complete compiler with 251 tests
Two compilation targets (TypeScript and C)
A self-hosting compiler (the language compiles its own compiler!)
A real distributed application called TimeTracker

The idea was to take the best features from four great languages:
- Erlang's fault-tolerant processes
- Rust's memory and resource safety
- Go's clean simplicity
- TypeScript's huge ecosystem

Then combine them into one language, guided by actual math (category theory and type theory), not just good vibes.

The moment it compiled itself for the first time was something else. 1,495 lines of JAPL, processing themselves into a working compiler. That's the kind of thing that makes you stare at your terminal for a while.

Everything is open source. Papers, compiler, spec, example app.

Check it out: https://japl-nine.vercel.app
Source code: https://github.com/YonedaAI/japl

#ProgrammingLanguages #Compiler #OpenSource #FunctionalProgramming #SoftwareEngineering #TypeScript #Rust #Erlang
