gemini:2: command not found: _zsh_nvm_load
Loaded cached credentials.
Registering notification handlers for server 'contextfs'. Capabilities: {
  experimental: {},
  prompts: { listChanged: false },
  resources: { subscribe: false, listChanged: false },
  tools: { listChanged: false }
}
Server 'contextfs' has tools but did not declare 'listChanged' capability. Listening anyway for robustness...
Server 'contextfs' has resources but did not declare 'listChanged' capability. Listening anyway for robustness...
Server 'contextfs' has prompts but did not declare 'listChanged' capability. Listening anyway for robustness...
Scheduling MCP context refresh...
Executing MCP context refresh...
MCP context refresh complete.
Error stating path " ++ domain
    validate_email(email) == Ok(email)
}
end{lstlisting}

Because tests are known to the compiler, it can:
begin{itemize}[leftmargin=*]
item Skip codegen for non-test code in test-only builds
item Provide test-specific error messages that reference the assertion expression
item Track test coverage at the expression level, not the line level
item Run property-based tests with integrated shrinking
end{itemize}

subsection{Built-in Package Manager}

texttt{japl deps} manages dependencies through a centralized registry with reproducible builds:

begin{lstlisting}[language=shell]
$ japl deps add http 0.5
$ japl deps update
$ japl deps audit    # security audit
$ japl deps tree     # dependency tree
end{lstlisting}

The lockfile format is deterministic and human-readable.
Dependency resolution uses a SAT solver to find compatible version sets, with clear error messages when resolution fails.

subsection{Language Server Protocol}

japl{} ships a Language Server Protocol (LSP) implementation that provides:
begin{itemize}[leftmargin=*]
item Hover information with types, effects, and documentation
item Go-to-definition (including into standard library and dependencies)
item Find-all-references
item Rename refactoring
item Inline diagnostics with fix suggestions
item Effect signature display for any expression
end{itemize}

The LSP server reuses the compiler's type-checking infrastructure, ensuring that IDE feedback is always consistent with compilation results.

subsection{REPL}

begin{lstlisting}[language=shell]
$ japl repl
japl> let xs = [1, 2, 3, 4, 5]
xs : List[Int] = [1, 2, 3, 4, 5]

japl> List.map(xs, fn x -> x * x)
[1, 4, 9, 16, 25] : List[Int]

japl> :type List.fold
List.fold : fn(List[a], b, fn(b, a) -> b) -> b

japl> :effects File.read_to_string
File.read_to_string : Io, Fail[IoError]
end{lstlisting}

The REPL supports incremental compilation: each expression is compiled and executed in the context of previous definitions, with the same type checking and effect tracking as regular code.

subsection{Built-in Profiler}

begin{lstlisting}[language=shell]
$ japl run --profile myapp.japl
# After execution:
Top functions by time:
  1. Json.parse         312ms  (42.1%)
  2. Http.handle_req    198ms  (26.7%)
  3. Db.query           156ms  (21.0%)
  4. List.map            42ms   (5.7%)

Per-process GC statistics:
  Process <0.12.0>: 3 minor GCs, 0 major
  Process <0.47.0>: 12 minor GCs, 1 major
  Process <0.48.0>: 8 minor GCs, 0 major

Message passing:
  Total messages: 14,235
  Avg queue depth: 2.3
  Max queue depth: 47 (Process <0.47.0>)
end{lstlisting}

Because the profiler is integrated with the runtime, it can provide process-aware, effect-aware profiling data that external profilers cannot: per-process allocation rates, message throughput between process pairs, supervision tree overhead, and effect handler costs.

% ============================================================
% 8. DEPLOYMENT
% ============================================================
section{Deployment}
label{sec:deployment}

Deployment is where operational simplicity pays its largest dividends.
A language can have the most beautiful type system in the world, but if deploying it requires a Ph.D. in systems administration, it will not be adopted for production use.

subsection{Static Binaries}

Every japl{} build produces a single statically linked binary.
The binary includes:
begin{itemize}[leftmargin=*]
item The application code (compiled to native machine code)
item The japl{} runtime (process scheduler, GC, network event loop)
item The standard library (only functions actually used, thanks to dead code elimination)
item A minimal C runtime (for system calls)
end{itemize}

No shared libraries, no virtual machine, no interpreter, no runtime downloads.

subsection{Cross-Compilation Matrix}

japl{} supports cross-compilation for all major targets from any development platform:

begin{table}[H]
centering
scriptsize
begin{tabular}{lccc}
toprule
textbf{Target} & textbf{From Linux} & textbf{From macOS} & textbf{From Windows} \
midrule
linux-amd64 & checkmark & checkmark & checkmark \
linux-arm64 & checkmark & checkmark & checkmark \
darwin-amd64 & checkmark & checkmark & checkmark \
darwin-arm64 & checkmark & checkmark & checkmark \
windows-amd64 & checkmark & checkmark & checkmark \
wasm32 & checkmark & checkmark & checkmark \
bottomrule
end{tabular}
caption{Cross-compilation support matrix.}
label{tab:cross-compilation}
end{table}

subsection{Container-Friendly Builds}

japl{}'s static binaries enable minimal container images:

begin{lstlisting}[language=shell,basicstyle=ttfamilytiny]
# Multi-stage build
FROM japl:latest AS builder
COPY . /app
RUN japl build --release --static /app/main.japl

# Final image: just the binary
FROM scratch
COPY --from=builder /app/main /main
ENTRYPOINT ["/main"]
end{lstlisting}

The resulting image contains only the application binary---no OS, no package manager, no shell.
This produces images in the 5--20 MB range (depending on application size), compared to 100+ MB for typical Go images, 200+ MB for Rust images with a Debian base, and 500+ MB for Haskell images.

subsection{Minimal Runtime Dependencies}

japl{} binaries have zero runtime dependencies beyond the operating system kernel.
Specifically, they do not require:
begin{itemize}[leftmargin=*]
item A C standard library (system calls are made directly)
item DNS resolver libraries (a pure-japl{} DNS client is included in the runtime)
item TLS libraries (a pure-japl{} TLS implementation is included)
item Any texttt{.so}/texttt{.dylib}/texttt{.dll} files
end{itemize}

This eliminates the ``works on my machine'' class of deployment failures that arise from shared library version mismatches.

subsection{Deployment Comparison}

begin{table}[H]
centering
scriptsize
begin{tabularx}{columnwidth}{lXXXXX}
toprule
& textbf{japl{}} & textbf{Go} & textbf{Rust} & textbf{Haskell} & textbf{Erlang} \
midrule
Artifact & Binary & Binary & Binary & Binary & Release \
Deps & None & libc & libc* & Many & BEAM \
Image & 5--20MB & 10--30MB & 5--30MB & 200+MB & 100+MB \
Cross & Trivial & Trivial & Moderate & Hard & N/A \
bottomrule
end{tabularx}
caption{Deployment characteristics. (*Rust can statically link musl libc.)}
label{tab:deployment}
end{table}

% ============================================================
% 9. COMPARISON
% ============================================================
section{Comparison with Existing Languages}
label{sec:comparison}

We now provide a detailed comparison of japl{} against five languages that each represent a different point in the design space.

subsection{Go: Gold Standard for Tooling, Weak Types}

Go~citep{pike2012go,donovan2015go} is the language japl{} most admires operationally and most disagrees with type-theoretically.

textbf{What Go gets right:}
begin{itemize}[leftmargin=*]
item Compilation speed (seconds for large projects)
item Single static binary deployment
item Canonical formatter (texttt{gofmt})
item Unified toolchain (texttt{go build/test/vet/doc})
item Cross-compilation via texttt{GOOS}/texttt{GOARCH}
item Excellent standard library for networking
end{itemize}

textbf{What Go gives up:}
begin{itemize}[leftmargin=*]
item No algebraic data types (sum types). Error handling is texttt{if err != nil} repeated emph{ad nauseam}.
item No exhaustive pattern matching. Missing cases are silent bugs.
item No generics until Go 1.18, and the resulting generics are limited.
item No effect tracking. Any function might perform I/O.
item Shared mutable memory with goroutines creates data race possibilities.
item No supervision trees. Goroutine failures are unobserved by default.
end{itemize}

japl{} aims to match Go's operational excellence while providing the type safety that Go lacks.
The key insight is that ADTs, traits, effect types, and pattern matching do not inherently require slow compilation---their type-checking algorithms are well within the decidable, practically-fast region of the complexity hierarchy.

subsection{Rust: Powerful but Slow Compilation}

Rust~citep{matsakis2014rust,klabnik2019rust} provides the strongest compile-time guarantees of any mainstream language, at significant cost.

textbf{What Rust gets right:}
begin{itemize}[leftmargin=*]
item Ownership and borrowing eliminate memory safety bugs
item Zero-cost abstractions
item Excellent pattern matching and enums
item Trait-based generics
item Growing ecosystem (crates.io)
end{itemize}

textbf{What Rust gives up:}
begin{itemize}[leftmargin=*]
item Compilation speed: 10+ minutes for medium projects is common
item Cognitive overhead: lifetime annotations pervade the codebase
item No lightweight processes (async/await is complex)
item No supervision or fault tolerance primitives
item No effect tracking (beyond texttt{unsafe})
item Steep learning curve, particularly for the borrow checker
end{itemize}

japl{} borrows Rust's ownership model for resources but applies it only where needed (external resources), keeping the common case (pure functional code) free of lifetime annotations.

subsection{Haskell: Powerful Types, Deployment Nightmare}

Haskell~citep{marlow2010haskell,jones2003haskell} is the purest realization of the ``types first'' philosophy, and its deployment story illustrates the cost of neglecting operational concerns.

textbf{What Haskell gets right:}
begin{itemize}[leftmargin=*]
item The most expressive type system in mainstream use
item Purity enforced by the type system
item Algebraic data types and pattern matching
item Type classes and higher-kinded types
item Lazy evaluation enables elegant abstractions
end{itemize}

textbf{What Haskell gives up:}
begin{itemize}[leftmargin=*]
item Compilation speed: GHC is slow, especially with extensions
item Deployment: dynamic linking, platform-specific builds, large executables
item Space leaks from lazy evaluation~citep{mitchell2013leaking}
item Unpredictable stack traces
item Multiple build systems (Cabal, Stack, Nix)
item IO monad creates a ``monad transformer stack'' complexity cliff
end{itemize}

japl{}'s effect system achieves Haskell-like purity tracking without monadic syntax overhead, and strict evaluation eliminates the space leak problem entirely.

subsection{Erlang: Great Runtime, Weak Tooling}

Erlang~citep{armstrong2003erlang,armstrong2007erlang} provides the runtime model that japl{} most closely follows, while addressing its shortcomings.

textbf{What Erlang gets right:}
begin{itemize}[leftmargin=*]
item Lightweight processes (millions per node)
item Supervision trees and fault tolerance
item Hot code loading
item Runtime observability (texttt{:observer}, tracing)
item Distribution built in
item Per-process GC
end{itemize}

textbf{What Erlang gives up:}
begin{itemize}[leftmargin=*]
item Dynamic typing: runtime type errors in production
item No algebraic data types or exhaustive pattern matching
item No resource safety (no ownership model)
item Requires BEAM VM installation on target systems
item Limited tooling (no canonical formatter until recently)
item Unusual syntax discourages adoption
end{itemize}

japl{} combines Erlang's runtime model with static typing, static binaries, and modern tooling.

subsection{OCaml: Good Balance, Ecosystem Gaps}

OCaml~citep{leroy2014ocaml} is perhaps the closest existing language to japl{}'s design philosophy, but with significant differences.

textbf{What OCaml gets right:}
begin{itemize}[leftmargin=*]
item Fast compilation
item Algebraic data types and pattern matching
item Powerful module system (functors)
item Good native code generation
item Hindley-Milner type inference
end{itemize}

textbf{What OCaml gives up:}
begin{itemize}[leftmargin=*]
item No lightweight processes (until OCaml 5.0 with effects)
item No supervision or distribution
item No effect tracking (unrestricted mutation)
item Smaller ecosystem than Go, Rust, or Haskell
item Historically fragmented tooling (improved with dune and opam)
item No cross-compilation story comparable to Go
end{itemize}

subsection{Summary}

begin{table*}[t]
centering
scriptsize
begin{tabular}{lccccccc}
toprule
textbf{Property} & textbf{japl{}} & textbf{Go} & textbf{Rust} & textbf{Haskell} & textbf{Erlang} & textbf{OCaml} & textbf{Gleam} \
midrule
ADTs + pattern matching & checkmark & & checkmark & checkmark & Partial & checkmark & checkmark \
Effect tracking & checkmark & & Partial & checkmark & & & \
Ownership for resources & checkmark & & checkmark & & & & \
Lightweight processes & checkmark & checkmark & & & checkmark & Partial & checkmark \
Supervision trees & checkmark & & & & checkmark & & checkmark \
Fast compilation & checkmark & checkmark & & & checkmark & checkmark & checkmark \
Static binaries & checkmark & checkmark & checkmark & Partial & & checkmark & \
Canonical formatter & checkmark & checkmark & checkmark & & & checkmark & checkmark \
Built-in test runner & checkmark & checkmark & checkmark & & & & \
Cross-compilation & checkmark & checkmark & checkmark & & & & \
Distribution & checkmark & & & & checkmark & & checkmark \
bottomrule
end{tabular}
caption{Feature comparison across languages. japl{} is the only language that achieves all properties simultaneously.}
label{tab:full-comparison}
end{table*}

% ============================================================
% 10. THE TYPE POWER BUDGET
% ============================================================
section{The Type Power Budget}
label{sec:type-power-budget}

Not all type system features are created equal.
Some provide enormous safety benefits at low complexity cost; others provide marginal benefits at high cost.
We formalize this observation as a emph{type power budget}: a framework for evaluating whether a type system feature's safety contribution justifies its complexity.

subsection{Formalization}

begin{definition}[Type Feature]
A type feature $phi$ is characterized by a tuple $(S_phi, C_phi, I_phi)$ where:
begin{itemize}
item $S_phi in [0, 1]$ is the emph{safety contribution}: the fraction of a representative bug taxonomy that $phi$ prevents.
item $C_phi in [0, 1]$ is the emph{complexity cost}: a normalized measure of the cognitive overhead, compilation cost, and tooling difficulty that $phi$ introduces.
item $I_phi subseteq Phi$ is the emph{interaction set}: the set of other features whose complexity is affected by $phi$'s presence.
end{itemize}
end{definition}

begin{definition}[Safety-per-Complexity Ratio]
The safety-per-complexity ratio of a feature $phi$ in the context of a feature set $F$ is:
[
rho(phi, F) = frac{S_phi}{displaystyle C_phi + sum_{psi in I_phi cap F} Delta C_{phi,psi}}
]
where $Delta C_{phi,psi}$ is the additional complexity from the interaction between $phi$ and $psi$.
end{definition}

begin{definition}[Type Power Budget]
A type power budget $B$ is a threshold on the minimum acceptable ratio:
[
F^* = {phi in Phi : rho(phi, F^*) geq B}
]
The budget $B$ partitions the space of type features into those that earn their keep and those that do not.
end{definition}

subsection{Feature Evaluation}

We evaluate concrete type system features against japl{}'s budget:

begin{table}[H]
centering
scriptsize
begin{tabularx}{columnwidth}{lcccc}
toprule
textbf{Feature} & $S_phi$ & $C_phi$ & $rho$ & textbf{Include?} \
midrule
ADTs (sum types) & 0.85 & 0.15 & 5.67 & Yes \
Pattern matching & 0.80 & 0.10 & 8.00 & Yes \
Parametric poly. & 0.70 & 0.15 & 4.67 & Yes \
Traits/type classes & 0.65 & 0.20 & 3.25 & Yes \
Row polymorphism & 0.45 & 0.20 & 2.25 & Yes \
Effect types & 0.60 & 0.25 & 2.40 & Yes \
Linear types (res.) & 0.55 & 0.20 & 2.75 & Yes \
midrule
GADTs & 0.25 & 0.40 & 0.63 & No \
Type families & 0.20 & 0.45 & 0.44 & No \
Dependent types & 0.30 & 0.70 & 0.43 & No \
HKTs (full) & 0.20 & 0.35 & 0.57 & No \
bottomrule
end{tabularx}
caption{Type feature evaluation. japl{} includes features with $rho geq 2.0$.}
label{tab:type-budget}
end{table}

subsection{What japl{} Includes}

begin{enumerate}[leftmargin=*]
item textbf{Algebraic data types.} Sum types and product types prevent null pointer errors, represent domain models precisely, and enable exhaustive pattern matching. The safety benefit is enormous; the complexity cost is low.

item textbf{Exhaustive pattern matching.} Catches missing cases at compile time. Negligible complexity cost for massive safety benefit.

item textbf{Parametric polymorphism.} Enables generic data structures and functions without sacrificing type safety. Well-understood, efficiently implementable.

item textbf{Traits (type classes).} Enable ad-hoc polymorphism (overloading) in a principled way. japl{} restricts to single-parameter type classes without functional dependencies, keeping resolution decidable and predictable.

item textbf{Row polymorphism.} Enables structural subtyping for records without full subtype polymorphism. Allows writing functions that work on ``any record with a texttt{name} field'' without inheritance.

item textbf{Effect types.} Track side effects in function signatures. Enable optimization (pure function elimination), documentation (what can this function do?), and safety (pure functions cannot perform I/O).

item textbf{Linear types for resources.} Ensure deterministic cleanup of external resources. Applied only to the resource layer, not to all values.
end{enumerate}

subsection{What japl{} Excludes}

begin{enumerate}[leftmargin=*]
item textbf{GADTs.} Generalized algebraic data types enable type-level programming but make type inference undecidable~citep{jones2006gadts}. The practical use cases (length-indexed vectors, well-typed interpreters) do not justify the complexity for a general-purpose language.

item textbf{Type families.} Type-level functions add significant complexity to the type checker and are a common source of confusing error messages in Haskell. Most practical uses can be achieved with traits and associated types.

item textbf{Dependent types.} Full dependent types make type checking undecidable. While dependently typed languages like Agda~citep{norell2007agda} and Idris~citep{brady2013idris} are fascinating research vehicles, the complexity cost is prohibitive for a language targeting production use.

item textbf{Higher-kinded types (full).} japl{} supports first-order type constructors (e.g., texttt{List[a]}, texttt{Option[a]}) but not higher-kinded types (e.g., a function parameterized over texttt{f} where texttt{f} is itself a type constructor). This limits some abstraction patterns (no generic ``Monad'' trait) but dramatically simplifies type inference and error messages.
The texttt{Functor} trait is provided as a special case known to the compiler, rather than as a consequence of full HKT support.
end{enumerate}

subsection{The Budget as Design Discipline}

The type power budget is not a mathematical formula applied mechanically; the numbers in Cref{tab:type-budget} are informed estimates.
The budget's value is as a emph{design discipline}: it forces the question ``what safety problem does this feature solve, and at what cost?'' for every proposed addition to the type system.

This discipline prevents feature creep---the gradual accumulation of type system features that individually seem justified but collectively produce an incomprehensible language.
Haskell's GHC has over 100 language extensions, many of which interact in surprising ways.
japl{}'s type power budget is explicitly designed to avoid this outcome.

% ============================================================
% 11. OBSERVABILITY
% ============================================================
section{Observability}
label{sec:observability}

Runtime observability is a first-class design concern in japl{}, not a third-party concern delegated to APM vendors.

subsection{Built-in Tracing}

japl{}'s runtime includes a distributed tracing system compatible with the OpenTelemetry standard~citep{opentelemetry2023}:

begin{lstlisting}
fn handle_request(req: Request)
    -> Response with Io, Net, Trace =
  Trace.span("handle_request", fn ->
    let user = Trace.span("auth", fn ->
      authenticate(req)?
    )
    let data = Trace.span("fetch_data", fn ->
      fetch_user_data(user.id)?
    )
    Response.json(200, data)
  )
end{lstlisting}

Traces propagate across process boundaries and across nodes in a distributed cluster.
The tracing system is built into the runtime, so it can capture process-level events (spawn, crash, restart) in addition to application-level spans.

subsection{Structured Logging}

begin{lstlisting}
fn process_order(order: Order)
    -> Result[Receipt, OrderError] with Io, Log =
  Log.info("Processing: ENAMETOOLONG: name too long, stat '/Users/mlong/Documents/Development/japl/" ++ domain
    validate_email(email) == Ok(email)
}
end{lstlisting}

Because tests are known to the compiler, it can:
begin{itemize}[leftmargin=*]
item Skip codegen for non-test code in test-only builds
item Provide test-specific error messages that reference the assertion expression
item Track test coverage at the expression level, not the line level
item Run property-based tests with integrated shrinking
end{itemize}

subsection{Built-in Package Manager}

texttt{japl deps} manages dependencies through a centralized registry with reproducible builds:

begin{lstlisting}[language=shell]
$ japl deps add http 0.5
$ japl deps update
$ japl deps audit    # security audit
$ japl deps tree     # dependency tree
end{lstlisting}

The lockfile format is deterministic and human-readable.
Dependency resolution uses a SAT solver to find compatible version sets, with clear error messages when resolution fails.

subsection{Language Server Protocol}

japl{} ships a Language Server Protocol (LSP) implementation that provides:
begin{itemize}[leftmargin=*]
item Hover information with types, effects, and documentation
item Go-to-definition (including into standard library and dependencies)
item Find-all-references
item Rename refactoring
item Inline diagnostics with fix suggestions
item Effect signature display for any expression
end{itemize}

The LSP server reuses the compiler's type-checking infrastructure, ensuring that IDE feedback is always consistent with compilation results.

subsection{REPL}

begin{lstlisting}[language=shell]
$ japl repl
japl> let xs = [1, 2, 3, 4, 5]
xs : List[Int] = [1, 2, 3, 4, 5]

japl> List.map(xs, fn x -> x * x)
[1, 4, 9, 16, 25] : List[Int]

japl> :type List.fold
List.fold : fn(List[a], b, fn(b, a) -> b) -> b

japl> :effects File.read_to_string
File.read_to_string : Io, Fail[IoError]
end{lstlisting}

The REPL supports incremental compilation: each expression is compiled and executed in the context of previous definitions, with the same type checking and effect tracking as regular code.

subsection{Built-in Profiler}

begin{lstlisting}[language=shell]
$ japl run --profile myapp.japl
# After execution:
Top functions by time:
  1. Json.parse         312ms  (42.1%)
  2. Http.handle_req    198ms  (26.7%)
  3. Db.query           156ms  (21.0%)
  4. List.map            42ms   (5.7%)

Per-process GC statistics:
  Process <0.12.0>: 3 minor GCs, 0 major
  Process <0.47.0>: 12 minor GCs, 1 major
  Process <0.48.0>: 8 minor GCs, 0 major

Message passing:
  Total messages: 14,235
  Avg queue depth: 2.3
  Max queue depth: 47 (Process <0.47.0>)
end{lstlisting}

Because the profiler is integrated with the runtime, it can provide process-aware, effect-aware profiling data that external profilers cannot: per-process allocation rates, message throughput between process pairs, supervision tree overhead, and effect handler costs.

% ============================================================
% 8. DEPLOYMENT
% ============================================================
section{Deployment}
label{sec:deployment}

Deployment is where operational simplicity pays its largest dividends.
A language can have the most beautiful type system in the world, but if deploying it requires a Ph.D. in systems administration, it will not be adopted for production use.

subsection{Static Binaries}

Every japl{} build produces a single statically linked binary.
The binary includes:
begin{itemize}[leftmargin=*]
item The application code (compiled to native machine code)
item The japl{} runtime (process scheduler, GC, network event loop)
item The standard library (only functions actually used, thanks to dead code elimination)
item A minimal C runtime (for system calls)
end{itemize}

No shared libraries, no virtual machine, no interpreter, no runtime downloads.

subsection{Cross-Compilation Matrix}

japl{} supports cross-compilation for all major targets from any development platform:

begin{table}[H]
centering
scriptsize
begin{tabular}{lccc}
toprule
textbf{Target} & textbf{From Linux} & textbf{From macOS} & textbf{From Windows} \
midrule
linux-amd64 & checkmark & checkmark & checkmark \
linux-arm64 & checkmark & checkmark & checkmark \
darwin-amd64 & checkmark & checkmark & checkmark \
darwin-arm64 & checkmark & checkmark & checkmark \
windows-amd64 & checkmark & checkmark & checkmark \
wasm32 & checkmark & checkmark & checkmark \
bottomrule
end{tabular}
caption{Cross-compilation support matrix.}
label{tab:cross-compilation}
end{table}

subsection{Container-Friendly Builds}

japl{}'s static binaries enable minimal container images:

begin{lstlisting}[language=shell,basicstyle=ttfamilytiny]
# Multi-stage build
FROM japl:latest AS builder
COPY . /app
RUN japl build --release --static /app/main.japl

# Final image: just the binary
FROM scratch
COPY --from=builder /app/main /main
ENTRYPOINT ["/main"]
end{lstlisting}

The resulting image contains only the application binary---no OS, no package manager, no shell.
This produces images in the 5--20 MB range (depending on application size), compared to 100+ MB for typical Go images, 200+ MB for Rust images with a Debian base, and 500+ MB for Haskell images.

subsection{Minimal Runtime Dependencies}

japl{} binaries have zero runtime dependencies beyond the operating system kernel.
Specifically, they do not require:
begin{itemize}[leftmargin=*]
item A C standard library (system calls are made directly)
item DNS resolver libraries (a pure-japl{} DNS client is included in the runtime)
item TLS libraries (a pure-japl{} TLS implementation is included)
item Any texttt{.so}/texttt{.dylib}/texttt{.dll} files
end{itemize}

This eliminates the ``works on my machine'' class of deployment failures that arise from shared library version mismatches.

subsection{Deployment Comparison}

begin{table}[H]
centering
scriptsize
begin{tabularx}{columnwidth}{lXXXXX}
toprule
& textbf{japl{}} & textbf{Go} & textbf{Rust} & textbf{Haskell} & textbf{Erlang} \
midrule
Artifact & Binary & Binary & Binary & Binary & Release \
Deps & None & libc & libc* & Many & BEAM \
Image & 5--20MB & 10--30MB & 5--30MB & 200+MB & 100+MB \
Cross & Trivial & Trivial & Moderate & Hard & N/A \
bottomrule
end{tabularx}
caption{Deployment characteristics. (*Rust can statically link musl libc.)}
label{tab:deployment}
end{table}

% ============================================================
% 9. COMPARISON
% ============================================================
section{Comparison with Existing Languages}
label{sec:comparison}

We now provide a detailed comparison of japl{} against five languages that each represent a different point in the design space.

subsection{Go: Gold Standard for Tooling, Weak Types}

Go~citep{pike2012go,donovan2015go} is the language japl{} most admires operationally and most disagrees with type-theoretically.

textbf{What Go gets right:}
begin{itemize}[leftmargin=*]
item Compilation speed (seconds for large projects)
item Single static binary deployment
item Canonical formatter (texttt{gofmt})
item Unified toolchain (texttt{go build/test/vet/doc})
item Cross-compilation via texttt{GOOS}/texttt{GOARCH}
item Excellent standard library for networking
end{itemize}

textbf{What Go gives up:}
begin{itemize}[leftmargin=*]
item No algebraic data types (sum types). Error handling is texttt{if err != nil} repeated emph{ad nauseam}.
item No exhaustive pattern matching. Missing cases are silent bugs.
item No generics until Go 1.18, and the resulting generics are limited.
item No effect tracking. Any function might perform I/O.
item Shared mutable memory with goroutines creates data race possibilities.
item No supervision trees. Goroutine failures are unobserved by default.
end{itemize}

japl{} aims to match Go's operational excellence while providing the type safety that Go lacks.
The key insight is that ADTs, traits, effect types, and pattern matching do not inherently require slow compilation---their type-checking algorithms are well within the decidable, practically-fast region of the complexity hierarchy.

subsection{Rust: Powerful but Slow Compilation}

Rust~citep{matsakis2014rust,klabnik2019rust} provides the strongest compile-time guarantees of any mainstream language, at significant cost.

textbf{What Rust gets right:}
begin{itemize}[leftmargin=*]
item Ownership and borrowing eliminate memory safety bugs
item Zero-cost abstractions
item Excellent pattern matching and enums
item Trait-based generics
item Growing ecosystem (crates.io)
end{itemize}

textbf{What Rust gives up:}
begin{itemize}[leftmargin=*]
item Compilation speed: 10+ minutes for medium projects is common
item Cognitive overhead: lifetime annotations pervade the codebase
item No lightweight processes (async/await is complex)
item No supervision or fault tolerance primitives
item No effect tracking (beyond texttt{unsafe})
item Steep learning curve, particularly for the borrow checker
end{itemize}

japl{} borrows Rust's ownership model for resources but applies it only where needed (external resources), keeping the common case (pure functional code) free of lifetime annotations.

subsection{Haskell: Powerful Types, Deployment Nightmare}

Haskell~citep{marlow2010haskell,jones2003haskell} is the purest realization of the ``types first'' philosophy, and its deployment story illustrates the cost of neglecting operational concerns.

textbf{What Haskell gets right:}
begin{itemize}[leftmargin=*]
item The most expressive type system in mainstream use
item Purity enforced by the type system
item Algebraic data types and pattern matching
item Type classes and higher-kinded types
item Lazy evaluation enables elegant abstractions
end{itemize}

textbf{What Haskell gives up:}
begin{itemize}[leftmargin=*]
item Compilation speed: GHC is slow, especially with extensions
item Deployment: dynamic linking, platform-specific builds, large executables
item Space leaks from lazy evaluation~citep{mitchell2013leaking}
item Unpredictable stack traces
item Multiple build systems (Cabal, Stack, Nix)
item IO monad creates a ``monad transformer stack'' complexity cliff
end{itemize}

japl{}'s effect system achieves Haskell-like purity tracking without monadic syntax overhead, and strict evaluation eliminates the space leak problem entirely.

subsection{Erlang: Great Runtime, Weak Tooling}

Erlang~citep{armstrong2003erlang,armstrong2007erlang} provides the runtime model that japl{} most closely follows, while addressing its shortcomings.

textbf{What Erlang gets right:}
begin{itemize}[leftmargin=*]
item Lightweight processes (millions per node)
item Supervision trees and fault tolerance
item Hot code loading
item Runtime observability (texttt{:observer}, tracing)
item Distribution built in
item Per-process GC
end{itemize}

textbf{What Erlang gives up:}
begin{itemize}[leftmargin=*]
item Dynamic typing: runtime type errors in production
item No algebraic data types or exhaustive pattern matching
item No resource safety (no ownership model)
item Requires BEAM VM installation on target systems
item Limited tooling (no canonical formatter until recently)
item Unusual syntax discourages adoption
end{itemize}

japl{} combines Erlang's runtime model with static typing, static binaries, and modern tooling.

subsection{OCaml: Good Balance, Ecosystem Gaps}

OCaml~citep{leroy2014ocaml} is perhaps the closest existing language to japl{}'s design philosophy, but with significant differences.

textbf{What OCaml gets right:}
begin{itemize}[leftmargin=*]
item Fast compilation
item Algebraic data types and pattern matching
item Powerful module system (functors)
item Good native code generation
item Hindley-Milner type inference
end{itemize}

textbf{What OCaml gives up:}
begin{itemize}[leftmargin=*]
item No lightweight processes (until OCaml 5.0 with effects)
item No supervision or distribution
item No effect tracking (unrestricted mutation)
item Smaller ecosystem than Go, Rust, or Haskell
item Historically fragmented tooling (improved with dune and opam)
item No cross-compilation story comparable to Go
end{itemize}

subsection{Summary}

begin{table*}[t]
centering
scriptsize
begin{tabular}{lccccccc}
toprule
textbf{Property} & textbf{japl{}} & textbf{Go} & textbf{Rust} & textbf{Haskell} & textbf{Erlang} & textbf{OCaml} & textbf{Gleam} \
midrule
ADTs + pattern matching & checkmark & & checkmark & checkmark & Partial & checkmark & checkmark \
Effect tracking & checkmark & & Partial & checkmark & & & \
Ownership for resources & checkmark & & checkmark & & & & \
Lightweight processes & checkmark & checkmark & & & checkmark & Partial & checkmark \
Supervision trees & checkmark & & & & checkmark & & checkmark \
Fast compilation & checkmark & checkmark & & & checkmark & checkmark & checkmark \
Static binaries & checkmark & checkmark & checkmark & Partial & & checkmark & \
Canonical formatter & checkmark & checkmark & checkmark & & & checkmark & checkmark \
Built-in test runner & checkmark & checkmark & checkmark & & & & \
Cross-compilation & checkmark & checkmark & checkmark & & & & \
Distribution & checkmark & & & & checkmark & & checkmark \
bottomrule
end{tabular}
caption{Feature comparison across languages. japl{} is the only language that achieves all properties simultaneously.}
label{tab:full-comparison}
end{table*}

% ============================================================
% 10. THE TYPE POWER BUDGET
% ============================================================
section{The Type Power Budget}
label{sec:type-power-budget}

Not all type system features are created equal.
Some provide enormous safety benefits at low complexity cost; others provide marginal benefits at high cost.
We formalize this observation as a emph{type power budget}: a framework for evaluating whether a type system feature's safety contribution justifies its complexity.

subsection{Formalization}

begin{definition}[Type Feature]
A type feature $phi$ is characterized by a tuple $(S_phi, C_phi, I_phi)$ where:
begin{itemize}
item $S_phi in [0, 1]$ is the emph{safety contribution}: the fraction of a representative bug taxonomy that $phi$ prevents.
item $C_phi in [0, 1]$ is the emph{complexity cost}: a normalized measure of the cognitive overhead, compilation cost, and tooling difficulty that $phi$ introduces.
item $I_phi subseteq Phi$ is the emph{interaction set}: the set of other features whose complexity is affected by $phi$'s presence.
end{itemize}
end{definition}

begin{definition}[Safety-per-Complexity Ratio]
The safety-per-complexity ratio of a feature $phi$ in the context of a feature set $F$ is:
[
rho(phi, F) = frac{S_phi}{displaystyle C_phi + sum_{psi in I_phi cap F} Delta C_{phi,psi}}
]
where $Delta C_{phi,psi}$ is the additional complexity from the interaction between $phi$ and $psi$.
end{definition}

begin{definition}[Type Power Budget]
A type power budget $B$ is a threshold on the minimum acceptable ratio:
[
F^* = {phi in Phi : rho(phi, F^*) geq B}
]
The budget $B$ partitions the space of type features into those that earn their keep and those that do not.
end{definition}

subsection{Feature Evaluation}

We evaluate concrete type system features against japl{}'s budget:

begin{table}[H]
centering
scriptsize
begin{tabularx}{columnwidth}{lcccc}
toprule
textbf{Feature} & $S_phi$ & $C_phi$ & $rho$ & textbf{Include?} \
midrule
ADTs (sum types) & 0.85 & 0.15 & 5.67 & Yes \
Pattern matching & 0.80 & 0.10 & 8.00 & Yes \
Parametric poly. & 0.70 & 0.15 & 4.67 & Yes \
Traits/type classes & 0.65 & 0.20 & 3.25 & Yes \
Row polymorphism & 0.45 & 0.20 & 2.25 & Yes \
Effect types & 0.60 & 0.25 & 2.40 & Yes \
Linear types (res.) & 0.55 & 0.20 & 2.75 & Yes \
midrule
GADTs & 0.25 & 0.40 & 0.63 & No \
Type families & 0.20 & 0.45 & 0.44 & No \
Dependent types & 0.30 & 0.70 & 0.43 & No \
HKTs (full) & 0.20 & 0.35 & 0.57 & No \
bottomrule
end{tabularx}
caption{Type feature evaluation. japl{} includes features with $rho geq 2.0$.}
label{tab:type-budget}
end{table}

subsection{What japl{} Includes}

begin{enumerate}[leftmargin=*]
item textbf{Algebraic data types.} Sum types and product types prevent null pointer errors, represent domain models precisely, and enable exhaustive pattern matching. The safety benefit is enormous; the complexity cost is low.

item textbf{Exhaustive pattern matching.} Catches missing cases at compile time. Negligible complexity cost for massive safety benefit.

item textbf{Parametric polymorphism.} Enables generic data structures and functions without sacrificing type safety. Well-understood, efficiently implementable.

item textbf{Traits (type classes).} Enable ad-hoc polymorphism (overloading) in a principled way. japl{} restricts to single-parameter type classes without functional dependencies, keeping resolution decidable and predictable.

item textbf{Row polymorphism.} Enables structural subtyping for records without full subtype polymorphism. Allows writing functions that work on ``any record with a texttt{name} field'' without inheritance.

item textbf{Effect types.} Track side effects in function signatures. Enable optimization (pure function elimination), documentation (what can this function do?), and safety (pure functions cannot perform I/O).

item textbf{Linear types for resources.} Ensure deterministic cleanup of external resources. Applied only to the resource layer, not to all values.
end{enumerate}

subsection{What japl{} Excludes}

begin{enumerate}[leftmargin=*]
item textbf{GADTs.} Generalized algebraic data types enable type-level programming but make type inference undecidable~citep{jones2006gadts}. The practical use cases (length-indexed vectors, well-typed interpreters) do not justify the complexity for a general-purpose language.

item textbf{Type families.} Type-level functions add significant complexity to the type checker and are a common source of confusing error messages in Haskell. Most practical uses can be achieved with traits and associated types.

item textbf{Dependent types.} Full dependent types make type checking undecidable. While dependently typed languages like Agda~citep{norell2007agda} and Idris~citep{brady2013idris} are fascinating research vehicles, the complexity cost is prohibitive for a language targeting production use.

item textbf{Higher-kinded types (full).} japl{} supports first-order type constructors (e.g., texttt{List[a]}, texttt{Option[a]}) but not higher-kinded types (e.g., a function parameterized over texttt{f} where texttt{f} is itself a type constructor). This limits some abstraction patterns (no generic ``Monad'' trait) but dramatically simplifies type inference and error messages.
The texttt{Functor} trait is provided as a special case known to the compiler, rather than as a consequence of full HKT support.
end{enumerate}

subsection{The Budget as Design Discipline}

The type power budget is not a mathematical formula applied mechanically; the numbers in Cref{tab:type-budget} are informed estimates.
The budget's value is as a emph{design discipline}: it forces the question ``what safety problem does this feature solve, and at what cost?'' for every proposed addition to the type system.

This discipline prevents feature creep---the gradual accumulation of type system features that individually seem justified but collectively produce an incomprehensible language.
Haskell's GHC has over 100 language extensions, many of which interact in surprising ways.
japl{}'s type power budget is explicitly designed to avoid this outcome.

% ============================================================
% 11. OBSERVABILITY
% ============================================================
section{Observability}
label{sec:observability}

Runtime observability is a first-class design concern in japl{}, not a third-party concern delegated to APM vendors.

subsection{Built-in Tracing}

japl{}'s runtime includes a distributed tracing system compatible with the OpenTelemetry standard~citep{opentelemetry2023}:

begin{lstlisting}
fn handle_request(req: Request)
    -> Response with Io, Net, Trace =
  Trace.span("handle_request", fn ->
    let user = Trace.span("auth", fn ->
      authenticate(req)?
    )
    let data = Trace.span("fetch_data", fn ->
      fetch_user_data(user.id)?
    )
    Response.json(200, data)
  )
end{lstlisting}

Traces propagate across process boundaries and across nodes in a distributed cluster.
The tracing system is built into the runtime, so it can capture process-level events (spawn, crash, restart) in addition to application-level spans.

subsection{Structured Logging}

begin{lstlisting}
fn process_order(order: Order)
    -> Result[Receipt, OrderError] with Io, Log =
  Log.info("Processing'
Attempt 1 failed: You have exhausted your capacity on this model. Your quota will reset after 1s.. Retrying after 5322ms...
Attempt 1 failed with status 429. Retrying with backoff... GaxiosError: [{
  "error": {
    "code": 429,
    "message": "No capacity available for model gemini-3-flash-preview on the server",
    "errors": [
      {
        "message": "No capacity available for model gemini-3-flash-preview on the server",
        "domain": "global",
        "reason": "rateLimitExceeded"
      }
    ],
    "status": "RESOURCE_EXHAUSTED",
    "details": [
      {
        "@type": "type.googleapis.com/google.rpc.ErrorInfo",
        "reason": "MODEL_CAPACITY_EXHAUSTED",
        "domain": "cloudcode-pa.googleapis.com",
        "metadata": {
          "model": "gemini-3-flash-preview"
        }
      }
    ]
  }
}
]
    at Gaxios._request (/Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/gaxios/build/src/gaxios.js:142:23)
    at process.processTicksAndRejections (node:internal/process/task_queues:95:5)
    at async OAuth2Client.requestAsync (/Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/google-auth-library/build/src/auth/oauth2client.js:429:18)
    at async CodeAssistServer.requestStreamingPost (file:///Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/code_assist/server.js:262:21)
    at async CodeAssistServer.generateContentStream (file:///Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/code_assist/server.js:54:27)
    at async file:///Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/core/loggingContentGenerator.js:285:26
    at async file:///Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/telemetry/trace.js:81:20
    at async retryWithBackoff (file:///Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/utils/retry.js:130:28)
    at async GeminiChat.makeApiCallAndProcessStream (file:///Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/core/geminiChat.js:440:32)
    at async GeminiChat.streamWithRetries (file:///Users/mlong/.local/share/fnm/node-versions/v20.20.0/installation/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/core/geminiChat.js:266:40) {
  config: {
    url: 'https://cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse',
    method: 'POST',
    params: { alt: 'sse' },
    headers: {
      'Content-Type': 'application/json',
      'User-Agent': 'GeminiCLI/0.34.0/gemini-3.1-pro-preview (darwin; arm64) google-api-nodejs-client/9.15.1',
      Authorization: '<<REDACTED> - See `errorRedactor` option in `gaxios` for configuration>.',
      'x-goog-api-client': 'gl-node/20.20.0'
    },
    responseType: 'stream',
    body: '<<REDACTED> - See `errorRedactor` option in `gaxios` for configuration>.',
    signal: AbortSignal { aborted: false },
    retry: false,
    paramsSerializer: [Function: paramsSerializer],
    validateStatus: [Function: validateStatus],
    errorRedactor: [Function: defaultErrorRedactor]
  },
  response: {
    config: {
      url: 'https://cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse',
      method: 'POST',
      params: [Object],
      headers: [Object],
      responseType: 'stream',
      body: '<<REDACTED> - See `errorRedactor` option in `gaxios` for configuration>.',
      signal: [AbortSignal],
      retry: false,
      paramsSerializer: [Function: paramsSerializer],
      validateStatus: [Function: validateStatus],
      errorRedactor: [Function: defaultErrorRedactor]
    },
    data: '[{\n' +
      '  "error": {\n' +
      '    "code": 429,\n' +
      '    "message": "No capacity available for model gemini-3-flash-preview on the server",\n' +
      '    "errors": [\n' +
      '      {\n' +
      '        "message": "No capacity available for model gemini-3-flash-preview on the server",\n' +
      '        "domain": "global",\n' +
      '        "reason": "rateLimitExceeded"\n' +
      '      }\n' +
      '    ],\n' +
      '    "status": "RESOURCE_EXHAUSTED",\n' +
      '    "details": [\n' +
      '      {\n' +
      '        "@type": "type.googleapis.com/google.rpc.ErrorInfo",\n' +
      '        "reason": "MODEL_CAPACITY_EXHAUSTED",\n' +
      '        "domain": "cloudcode-pa.googleapis.com",\n' +
      '        "metadata": {\n' +
      '          "model": "gemini-3-flash-preview"\n' +
      '        }\n' +
      '      }\n' +
      '    ]\n' +
      '  }\n' +
      '}\n' +
      ']',
    headers: {
      'alt-svc': 'h3=":443"; ma=2592000,h3-29=":443"; ma=2592000',
      'content-length': '630',
      'content-type': 'application/json; charset=UTF-8',
      date: 'Thu, 26 Mar 2026 16:58:26 GMT',
      server: 'ESF',
      'server-timing': 'gfet4t7; dur=5521',
      vary: 'Origin, X-Origin, Referer',
      'x-cloudaicompanion-trace-id': '7cb70f358cf39371',
      'x-content-type-options': 'nosniff',
      'x-frame-options': 'SAMEORIGIN',
      'x-xss-protection': '0'
    },
    status: 429,
    statusText: 'Too Many Requests',
    request: {
      responseURL: 'https://cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse'
    }
  },
  error: undefined,
  status: 429,
  [Symbol(gaxios-gaxios-error)]: '6.7.1'
}
This is a comprehensive and well-argued paper that strikes a rare balance between programming language theory and systems engineering pragmatism. The central thesis—that operational excellence (compilation speed, deployment, tooling) is a first-class design constraint on par with type system expressiveness—is timely and addresses a significant "usability crisis" in the functional programming community.

The following is a formal review of the manuscript.

---

### **Summary**
The paper introduces **Japl**, a functional language designed to inhabit the "Pareto frontier" of type safety and operational simplicity. It proposes two novel frameworks: a **Type Power Budget** for evaluating language features based on a safety-to-complexity ratio, and a **Hybrid Memory Model** that bifurcates memory management into a BEAM-inspired per-process GC for immutable data and a Rust-inspired ownership/linear type system for external resources. The paper concludes with a case study demonstrating Japl's advantages in build times and deployment footprint compared to Go, Rust, Haskell, and Erlang.

### **Strengths**

1.  **Philosophical Clarity:** The coining of the term **"Elegant but Operationally Miserable" (EBOM)** is a significant contribution to the discourse on PL design. It provides a useful label for the friction points that prevent academic languages from achieving industrial scale.
2.  **Hybrid Memory Model:** The distinction between "Immutable Data" and "Resources" in Section 5 is a profound architectural insight. By not forcing a single memory management paradigm on two fundamentally different categories of data, Japl avoids both the non-determinism of GC for resources and the cognitive overhead of lifetimes for pure data.
3.  **Tooling as a Language Feature:** The paper correctly identifies that Go's success was largely "social" and "operational." Integrating the formatter, test runner, and profiler into the compiler itself is a proven strategy for reducing ecosystem fragmentation.
4.  **Effect-Type Integration:** Section 3.4 and 6.4 provide a strong technical justification for effect types, demonstrating how they aren't just for safety but are active drivers for aggressive compiler optimizations (e.g., safe dead code elimination).

### **Weaknesses**

1.  **Interaction between GC and Ownership:** A significant gap exists in explaining the interface between the GC heap and the Resource Arena. If an immutable Record (GC-managed) contains an owned File Handle (Linear-managed), how is the resource's linearity enforced? If the Record is dropped/GC'd, the resource cleanup becomes non-deterministic, violating the paper’s goal. The paper implies these are separate, but practical programs frequently need to package resources within data structures.
2.  **Subjectivity of the Type Power Budget:** While the framework in Section 10 is formally described, the values assigned ($S_\phi, C_\phi$) are subjective. For instance, the paper claims GADTs have a safety contribution of only 0.25. Proponents of strongly-typed DSLs would argue this is much higher. The paper would benefit from acknowledging these values as "design-team estimates" rather than objective constants.
3.  **Formal Rigor in Theorem 5.1:** The "Proof Sketch" for Theorem 5.1 (Hybrid Memory Model Correctness) is more of a restatement of the design intent. A more rigorous approach would require a small-step operational semantics showing that a well-typed program cannot reach a state where a resource is accessed after its ownership has been transferred.
4.  **Lack of Detail on "Typed Mailboxes":** The paper mentions Erlang-style processes with "typed mailboxes" but doesn't explain how variance or protocol evolution is handled. Typed actors are notoriously difficult to get right (as seen in Akka Typed).

### **Specific Suggestions**

*   **Line 126 (Proposition 3.1):** The limit $\lim_{|P| \to \infty} \frac{T_{\text{type}}(P)}{T(P)} \to 1$ is a bold claim. While true for languages with heavy type-level computation (like C++ templates or Scala), it may not hold for Japl's "bounded" type system. I suggest adding a qualifier: "For languages with unrestricted type-level features..."
*   **Section 5.3 (Resource Arena):** Please clarify if resources are "First-Class" or "Second-Class." Can I put an `own File` into a `List`? If so, the `List` must likely become a "Linear List," which significantly increases complexity. If you disallow putting resources in common collections, the language's expressiveness might be too limited for complex systems.
*   **Section 10.2 (Table 10):** Add a column or footnote for "Interaction Costs" ($\Delta C_{\phi,\psi}$). For example, the interaction between "Traits" and "Row Polymorphism" is notoriously complex to implement in a compiler.
*   **Case Study (Section 12.3):** The table lists "Build time (clean)" for Japl as 3s. Since the language is new, the paper should clarify if this is a measurement of a prototype compiler or a theoretical target based on Cranelift benchmarks.

### **Line-Level Feedback**

*   **L54:** "EBOM" – Excellent terminology.
*   **L145:** "Separating interface declarations from implementations" – This is a return to the Modula-2/Ada style. It would be worth citing the Modula-3 design report here as a precursor to this philosophy.
*   **L285:** The `use` keyword in the code example (reminiscent of C# or Python's `with`) is a great syntactic choice for managing linear resources.
*   **L415:** "No shared mutable memory." – Does Japl provide a "backdoor" for high-performance shared buffers (like `Arc<UnsafeCell<T>>` in Rust)? For high-performance networking, zero-copy buffers are often shared between the kernel and multiple processes.

---

### **Overall Assessment: Minor Revision**

The paper is excellent and provides a compelling vision for the future of systems programming. It moves the conversation beyond "How much can my type system prove?" to "How effectively can I build and run this software?" 

To reach "Accept" status, the authors should primarily address the **interaction between the GC heap and the Resource Arena** (Weakness #1). If resources cannot be placed in GC-managed data structures, the paper should explicitly state this limitation. If they can, the mechanism for maintaining linearity across the GC boundary must be explained.

**Reviewer Decision:** Minor Revision.
