# LinkedIn: JAPL

We designed and built a programming language from first principles — and then we used it to compile itself.

JAPL (Just Another Programming Language) is a statically-typed, pure-by-default functional language with Erlang-style concurrency, Rust-inspired resource safety, and dual-backend compilation targeting both TypeScript and C. The project includes a complete compiler (lexer, parser, type checker, code generator), 251 tests, 7 research papers totaling over 160 pages, and a proof-of-concept distributed application called TimeTracker.

The self-hosting milestone is the one we are most proud of: 1,495 lines of JAPL compiling JAPL. A programming language that can build its own compiler validates that the design is expressive enough, the type system is sound enough, and the implementation is complete enough to handle real-world complexity — including its own.

The dual-backend architecture compiles JAPL source to both TypeScript (for rapid prototyping and web deployment) and C (for systems programming and performance). Same source code, two targets, one type system guaranteeing safety across both.

This is not a weekend project or a toy. It is a research platform demonstrating that functional programming, actor-model concurrency, and linear resource management can coexist in a single coherent language.

All papers, compiler source, and documentation: https://japl-nine.vercel.app
Source: https://github.com/YonedaAI/japl

#ProgrammingLanguages #Compiler #OpenSource #FunctionalProgramming #TypeScript #Rust #Erlang #TypeTheory #SoftwareEngineering
