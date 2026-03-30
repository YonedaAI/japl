# JAPL Language Specification

**Version:** 1.0-draft
**Date:** 2026-03-26
**Status:** Working Draft

> "Pure by default, concurrent by design, resource-safe by construction."

JAPL (Just Another Programming Language) is a strict, typed, effect-aware functional programming language. It combines Rust's ownership and resource safety, Go's simplicity and tooling, Erlang/OTP's lightweight processes and supervision trees, and the FP tradition's immutable values, algebraic data types, pattern matching, and effect tracking.

---

## Table of Contents

1. [Lexical Structure](#1-lexical-structure)
2. [Syntax (EBNF Grammar)](#2-syntax-ebnf-grammar)
3. [Type System](#3-type-system)
4. [Ownership and Linearity](#4-ownership-and-linearity)
5. [Expression Semantics](#5-expression-semantics)
6. [Process Semantics](#6-process-semantics)
7. [Supervision](#7-supervision)
8. [Error Handling](#8-error-handling)
9. [Distribution](#9-distribution)
10. [Module System](#10-module-system)
11. [Standard Library (Core Types)](#11-standard-library-core-types)
12. [FFI](#12-ffi)
13. [Built-in Test Framework](#13-built-in-test-framework)

---

## 1. Lexical Structure

### 1.1 Source Encoding

All JAPL source files are UTF-8 encoded. The file extension is `.japl`.

### 1.2 Comments

```
-- This is a line comment (extends to end of line)
```

Line comments begin with `--` and extend to the end of the line. There are no block comments.

### 1.3 Whitespace and Layout

JAPL is indentation-aware. Blocks are delimited by indentation rather than braces. Semicolons are not used. Newlines are significant as statement terminators within blocks. Blank lines are ignored.

### 1.4 Keywords

The following identifiers are reserved keywords:

```
assert    bench     continue  deriving  do        else
fn        forall    foreign   if        impl      import
let       loop      match     module    opaque    own
packed    property  ref       receive   send      signature
spawn     strategy  supervisor test     then      trait
type      unsafe    use       where     while     with
```

### 1.5 Identifiers

```
identifier      ::= lower_start (alpha | digit | '_')*
type_identifier ::= upper_start (alpha | digit | '_')*
lower_start     ::= 'a' .. 'z' | '_'
upper_start     ::= 'A' .. 'Z'
alpha           ::= 'a' .. 'z' | 'A' .. 'Z'
digit           ::= '0' .. '9'
```

Value-level identifiers (variables, function names, field names) begin with a lowercase letter or underscore. Type-level identifiers (type names, constructors, module names) begin with an uppercase letter.

### 1.6 Literals

#### Integer Literals

```
int_literal     ::= decimal | hexadecimal | octal | binary
decimal         ::= digit (digit | '_')*
hexadecimal     ::= '0x' hex_digit (hex_digit | '_')*
octal           ::= '0o' oct_digit (oct_digit | '_')*
binary          ::= '0b' bin_digit (bin_digit | '_')*
hex_digit       ::= digit | 'a'..'f' | 'A'..'F'
oct_digit       ::= '0'..'7'
bin_digit       ::= '0' | '1'
```

Integer literals are 64-bit signed integers with checked overflow. The compiler rejects operations that would overflow at compile time when detectable; runtime overflow traps. Underscores may be used as visual separators.

#### Float Literals

```
float_literal   ::= digit+ '.' digit+ exponent?
                   | digit+ exponent
exponent        ::= ('e' | 'E') ('+' | '-')? digit+
```

Float literals denote 64-bit IEEE 754 double-precision values by default. A `Float32` suffix may be used for 32-bit floats.

#### String Literals

```
string_literal  ::= '"' string_char* '"'
string_char     ::= <any UTF-8 character except '"' and '\'>
                   | escape_sequence
escape_sequence ::= '\\' | '\"' | '\n' | '\t' | '\r' | '\0'
                   | '\u{' hex_digit+ '}'
```

Strings are UTF-8 encoded and immutable.

#### Boolean Literals

```
True | False
```

#### Unit Literal

```
()
```

#### List Literals

```
[1, 2, 3]
[1, ..rest]        -- cons pattern / spread
[]                  -- empty list
```

### 1.7 Operators

#### Arithmetic Operators
| Operator | Meaning | Precedence |
|----------|---------|------------|
| `+`      | Addition | 6 |
| `-`      | Subtraction | 6 |
| `*`      | Multiplication | 7 |
| `/`      | Division | 7 |
| `%`      | Modulo | 7 |

#### Comparison Operators
| Operator | Meaning | Precedence |
|----------|---------|------------|
| `==`     | Structural equality | 4 |
| `!=`     | Structural inequality | 4 |
| `<`      | Less than | 5 |
| `>`      | Greater than | 5 |
| `<=`     | Less than or equal | 5 |
| `>=`     | Greater than or equal | 5 |

#### Logical Operators
| Operator | Meaning | Precedence |
|----------|---------|------------|
| `&&`     | Logical AND (short-circuit) | 3 |
| `\|\|`   | Logical OR (short-circuit) | 2 |
| `!`      | Logical NOT (prefix) | 9 |

#### Composition and Pipeline Operators
| Operator | Meaning | Precedence |
|----------|---------|------------|
| `\|>`    | Pipe (left-to-right application) | 1 |
| `>>`     | Forward function composition | 1 |
| `++`     | String/list concatenation | 6 |
| `<>`     | Semigroup append | 6 |

#### Special Operators
| Operator | Meaning |
|----------|---------|
| `?`      | Error propagation (postfix) |
| `\|`     | Record update separator / sum type variant separator |
| `->`     | Function arrow / match arm arrow |
| `=`      | Binding / definition |
| `:`      | Type annotation |
| `.`      | Field access / module path separator |
| `..`     | Spread operator (lists, records) |

### 1.8 Operator Precedence (Highest to Lowest)

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 10 | `.` (field access) | Left |
| 9 | `!` (prefix NOT), `-` (prefix negation) | Prefix |
| 8 | `?` (error propagation) | Postfix |
| 7 | `*`, `/`, `%` | Left |
| 6 | `+`, `-`, `++`, `<>` | Left |
| 5 | `<`, `>`, `<=`, `>=` | Non-associative |
| 4 | `==`, `!=` | Non-associative |
| 3 | `&&` | Left |
| 2 | `\|\|` | Left |
| 1 | `\|>`, `>>` | Left |

---

## 2. Syntax (EBNF Grammar)

The grammar uses the following EBNF conventions: `{ X }` means zero or more `X`; `[ X ]` means optional `X`; `( X | Y )` means alternative; and literal tokens are in double quotes.

### 2.1 Top-Level Program

```ebnf
program         ::= { top_decl }

top_decl        ::= module_decl
                   | import_decl
                   | type_decl
                   | fn_decl
                   | trait_decl
                   | impl_decl
                   | signature_decl
                   | supervisor_decl
                   | test_decl
                   | property_decl
                   | bench_decl
                   | foreign_decl
```

### 2.2 Module Declarations

```ebnf
module_decl     ::= "module" module_path [ ":" signature_ref ] "=" module_body
                   | "module" module_path

module_path     ::= TYPE_ID { "." TYPE_ID }

module_body     ::= INDENT { module_item } DEDENT

module_item     ::= type_decl | fn_decl | trait_decl | impl_decl
                   | module_decl | signature_decl
```

### 2.3 Import Declarations

```ebnf
import_decl     ::= "import" module_path [ import_spec ]

import_spec     ::= "." "{" import_item { "," import_item } "}"

import_item     ::= IDENT | TYPE_ID
```

### 2.4 Type Declarations

```ebnf
type_decl       ::= "type" TYPE_ID [ type_params ] [ deriving_clause ] "=" type_body
                   | "type" "alias" TYPE_ID [ type_params ] "=" type_expr
                   | "opaque" "type" TYPE_ID [ type_params ]

type_params     ::= "[" type_var { "," type_var } "]"

type_var        ::= IDENT

type_body       ::= sum_type | record_type

sum_type        ::= "|" constructor { "|" constructor }

constructor     ::= TYPE_ID [ "(" type_expr { "," type_expr } ")" ]

record_type     ::= "{" field_decl { "," field_decl } "}"

field_decl      ::= IDENT ":" type_expr

deriving_clause ::= "deriving" "(" TYPE_ID { "," TYPE_ID } ")"
```

### 2.5 Function Declarations

```ebnf
fn_decl         ::= "fn" IDENT [ type_params ] "(" [ param_list ] ")" "->" type_expr [ effect_clause ] [ where_clause ] "=" expr

param_list      ::= param { "," param }

param           ::= [ ownership_qual ] IDENT [ ":" type_expr ]

ownership_qual  ::= "own" | "ref"

effect_clause   ::= "with" effect { "," effect }

effect          ::= "Pure" | "Io" | "Async" | "Net"
                   | "State" "[" type_expr "]"
                   | "Process" [ "[" type_expr "]" ]
                   | "Fail" "[" type_expr "]"

where_clause    ::= "where" constraint { "," constraint }

constraint      ::= TYPE_ID "[" type_expr { "," type_expr } "]"
```

### 2.6 Expressions

```ebnf
expr            ::= let_expr
                   | use_expr
                   | match_expr
                   | if_expr
                   | fn_expr
                   | loop_expr
                   | pipe_expr

let_expr        ::= "let" pattern "=" expr NEWLINE expr

use_expr        ::= "use" IDENT "=" expr NEWLINE expr

match_expr      ::= "match" expr "with" NEWLINE { match_arm }

match_arm       ::= "|" pattern [ guard ] "->" expr NEWLINE

guard           ::= "if" expr

if_expr         ::= "if" expr "then" expr "else" expr

fn_expr         ::= "fn" [ IDENT ] [ "(" [ param_list ] ")" ] "->" expr
                   | "fn" IDENT "->" expr

loop_expr       ::= "loop" loop_bindings "while" expr "do" expr

loop_bindings   ::= loop_binding { "," loop_binding }

loop_binding    ::= IDENT "=" expr

pipe_expr       ::= unary_expr { "|>" unary_expr }

unary_expr      ::= compose_expr { ">>" compose_expr }

compose_expr    ::= or_expr

or_expr         ::= and_expr { "||" and_expr }

and_expr        ::= cmp_expr { "&&" cmp_expr }

cmp_expr        ::= add_expr [ cmp_op add_expr ]

cmp_op          ::= "==" | "!=" | "<" | ">" | "<=" | ">="

add_expr        ::= mul_expr { ("+" | "-" | "++" | "<>") mul_expr }

mul_expr        ::= postfix_expr { ("*" | "/" | "%") postfix_expr }

postfix_expr    ::= primary_expr { "?" | "." IDENT | "(" [ arg_list ] ")" }

arg_list        ::= expr { "," expr }

primary_expr    ::= IDENT
                   | TYPE_ID [ "(" [ arg_list ] ")" ]
                   | int_literal | float_literal | string_literal
                   | "True" | "False"
                   | "()"
                   | "(" expr ")"
                   | list_expr
                   | record_expr
                   | record_update

list_expr       ::= "[" [ expr { "," expr } [ "," ".." expr ] ] "]"

record_expr     ::= "{" field_assign { "," field_assign } "}"

field_assign    ::= IDENT "=" expr

record_update   ::= "{" expr "|" field_assign { "," field_assign } "}"
```

### 2.7 Patterns

```ebnf
pattern         ::= constructor_pat
                   | record_pat
                   | literal_pat
                   | wildcard_pat
                   | var_pat
                   | list_pat
                   | pinned_pat

constructor_pat ::= TYPE_ID [ "(" pattern { "," pattern } ")" ]

record_pat      ::= "{" field_pat { "," field_pat } "}"

field_pat       ::= IDENT [ "=" pattern ]

literal_pat     ::= int_literal | float_literal | string_literal
                   | "True" | "False" | "()"

wildcard_pat    ::= "_"

var_pat         ::= IDENT

list_pat        ::= "[" [ pattern { "," pattern } [ "," ".." pattern ] ] "]"

pinned_pat      ::= "^" IDENT
```

### 2.8 Type Expressions

```ebnf
type_expr       ::= fn_type
                   | simple_type

fn_type         ::= "fn" "(" [ type_expr { "," type_expr } ] ")" "->" type_expr [ effect_clause ]

simple_type     ::= named_type
                   | record_type_expr
                   | type_var
                   | "(" type_expr ")"

named_type      ::= TYPE_ID [ "[" type_expr { "," type_expr } "]" ]

record_type_expr ::= "{" field_type { "," field_type } [ "|" type_var ] "}"

field_type      ::= IDENT ":" type_expr
```

### 2.9 Trait Declarations

```ebnf
trait_decl      ::= "trait" TYPE_ID "[" type_var { "," type_var } "]" [ "where" constraint { "," constraint } ] "=" INDENT { trait_member } DEDENT

trait_member    ::= fn_signature

fn_signature    ::= "fn" IDENT "(" [ param_list ] ")" "->" type_expr [ effect_clause ]
```

### 2.10 Impl Declarations

```ebnf
impl_decl       ::= "impl" TYPE_ID "[" type_expr { "," type_expr } "]" "=" INDENT { fn_decl } DEDENT
```

### 2.11 Signature Declarations

```ebnf
signature_decl  ::= "signature" TYPE_ID [ type_params ] "=" INDENT { sig_member } DEDENT

sig_member      ::= "type" TYPE_ID
                   | fn_signature
```

### 2.12 Supervisor Declarations

```ebnf
supervisor_decl ::= "supervisor" IDENT "=" INDENT supervisor_body DEDENT

supervisor_body ::= "strategy" "=" strategy_expr NEWLINE
                     "max_restarts" "=" int_literal NEWLINE
                     "max_seconds" "=" int_literal NEWLINE
                     "children" "=" "[" { child_spec "," } "]"

strategy_expr   ::= "OneForOne" | "AllForOne" | "RestForOne"

child_spec      ::= "{" "id" "=" string_literal ","
                         "start" "=" expr ","
                         "restart" "=" restart_type ","
                         "shutdown" "=" shutdown_expr "}"

restart_type    ::= "Permanent" | "Transient" | "Temporary"

shutdown_expr   ::= "Timeout" "(" int_literal ")" | "Brutal"
```

### 2.13 Test Declarations

```ebnf
test_decl       ::= "test" string_literal "=" expr

property_decl   ::= "property" string_literal "=" "forall" "(" param_list ")" "->" expr

bench_decl      ::= "bench" string_literal "=" expr
```

### 2.14 Foreign Declarations

```ebnf
foreign_decl    ::= "foreign" string_literal "fn" IDENT "(" [ param_list ] ")" "->" type_expr
                   | "foreign" string_literal "module" string_literal "=" INDENT { foreign_fn } DEDENT

foreign_fn      ::= "fn" IDENT "(" [ param_list ] ")" "->" type_expr
```

---

## 3. Type System

JAPL's type system is a static, strong, parametrically polymorphic type system with bidirectional local type inference, algebraic data types, row polymorphism, traits, effect types, and linear resource types.

### 3.1 Primitive Types

| Type | Description | Representation |
|------|-------------|----------------|
| `Int` | Arbitrary-precision integer | Bignum (small ints unboxed) |
| `Float` | 64-bit IEEE 754 double precision | 8 bytes |
| `Float32` | 32-bit IEEE 754 single precision | 4 bytes |
| `Bool` | Boolean: `True` or `False` | 1 byte (tagged) |
| `Char` | Unicode scalar value | 4 bytes |
| `String` | Immutable UTF-8 encoded string | Pointer + length |
| `Bytes` | Immutable raw byte sequence | Pointer + length |
| `Unit` | The unit type; sole value `()` | Zero-size |
| `Never` | The bottom type; no values | Uninhabited |

All primitive types are immutable and freely copyable.

### 3.2 Algebraic Data Types

#### 3.2.1 Sum Types (Tagged Unions)

Sum types define a closed set of variants. Each variant is a constructor that may carry typed payloads.

```japl
type Shape =
  | Circle(Float)
  | Rectangle(Float, Float)
  | Triangle(Float, Float, Float)

type Option[a] =
  | Some(a)
  | None

type Result[a, e] =
  | Ok(a)
  | Err(e)
```

**Typing rule (Constructor Introduction):**

If type `T` has constructor `C(T1, ..., Tn)`, and `ei : Ti` for each `i`, then `C(e1, ..., en) : T`.

Sum types are closed: the set of variants is fixed at the definition site. The compiler enforces exhaustive pattern matching on sum types.

#### 3.2.2 Product Types (Records)

Records are structurally typed, labeled product types.

```japl
type User = {
  id: UserId,
  name: String,
  email: String,
  created_at: Timestamp,
}
```

Records are created with `=` syntax and accessed with `.` notation:

```japl
let alice = { id = UserId(1), name = "Alice", email = "a@b.com" }
let name = alice.name
```

**Record update syntax** creates a new record with modified fields:

```japl
let updated = { alice | name = "Alice Smith" }
```

The original record is unchanged.

#### 3.2.3 Packed Types

The `packed` qualifier requests a contiguous memory layout with no pointer indirection:

```japl
type Vec3 = packed { x: Float32, y: Float32, z: Float32 }
```

Packed types must contain only fixed-size types (no polymorphic fields, no heap-allocated types).

### 3.3 Parametric Polymorphism

JAPL supports parametric polymorphism (generics) using type variables denoted by lowercase identifiers.

```japl
fn map[a, b](list: List[a], f: fn(a) -> b) -> List[b] =
  match list with
  | [] -> []
  | [x, ..rest] -> [f(x), ..map(rest, f)]

fn identity[a](x: a) -> a = x
```

Type variables are implicitly universally quantified at the function level. Explicit quantification is available when needed:

```japl
fn const[a, b](x: a, _y: b) -> a = x
```

**Parametricity guarantee (free theorem):** A polymorphic function cannot inspect or branch on the runtime representation of a type variable. This guarantees that `identity : a -> a` can only be the identity function.

### 3.4 Type Inference

JAPL uses bidirectional type checking with local inference. The rules are:

1. **Top-level function signatures are required** at module boundaries. This ensures separate compilation and provides documentation.
2. **Within function bodies, types are inferred.** Local let-bindings, lambda parameters, and intermediate expressions have their types inferred.
3. **Type annotations may appear anywhere** for documentation or disambiguation: `let x: Int = 42`.

**Bidirectional rules:**

- **Checking mode:** Given an expected type, verify that an expression has that type.
- **Synthesis mode:** Given an expression, compute its type.

The inference algorithm is based on Hindley-Milner with extensions for row polymorphism and effect types. Type inference is decidable and runs in linear time for practical programs.

```japl
-- Signature required at module boundary
fn process(items: List[Item]) -> Summary with Io =
  -- Types inferred within the body
  let totals = List.map(items, fn item -> item.price * item.quantity)
  let sum = List.fold(totals, 0, fn acc, t -> acc + t)
  { item_count = List.length(items), total = sum }
```

### 3.5 Row Polymorphism

Records are structurally typed with row polymorphism. A row variable represents "the rest of the fields."

```japl
fn get_name(r: { name: String | rest }) -> String =
  r.name
```

The type `{ name: String | rest }` matches any record that has at least a `name: String` field. The row variable `rest` captures the remaining fields.

```japl
-- All of these calls are valid:
get_name({ name = "Alice" })
get_name({ name = "Bob", age = 30 })
get_name({ name = "Charlie", role = Admin, email = "c@b.com" })
```

**Typing rule (Field Access):**

If `e : { l: T | r }`, then `e.l : T`.

**Typing rule (Record Update):**

If `e : { l: T1, ... | r }`, then `{ e | l = e2 }` has type `{ l: T2, ... | r }` where `e2 : T2`.

### 3.6 Type Aliases and Opaque Types

**Type aliases** create synonyms with no abstraction boundary:

```japl
type alias Headers = Map[String, String]
type alias UserId = Int
```

**Opaque types** hide their representation outside the defining module:

```japl
module Map =
  opaque type Map[k, v]
  -- Implementation details hidden
```

Outside the `Map` module, code cannot inspect or construct `Map[k, v]` values except through the module's public API. This provides encapsulation equivalent to private fields in OOP.

### 3.7 Traits (Type Classes)

Traits define a set of functions that a type must implement. Traits support superclass constraints via `where`.

```japl
trait Eq[a] =
  fn eq(x: a, y: a) -> Bool

trait Ord[a] where Eq[a] =
  fn compare(x: a, y: a) -> Ordering

trait Show[a] =
  fn show(value: a) -> String

trait Functor[f] =
  fn map[a, b](fa: f[a], func: fn(a) -> b) -> f[b]

trait Serialize[a] =
  fn serialize(value: a) -> Bytes
  fn deserialize(data: Bytes) -> Result[a, SerializeError]
```

Implementations are provided with `impl`:

```japl
impl Show[Shape] =
  fn show(shape) =
    match shape with
    | Circle(r) -> "Circle(" ++ Float.to_string(r) ++ ")"
    | Rectangle(w, h) -> "Rectangle(" ++ Float.to_string(w) ++ ", " ++ Float.to_string(h) ++ ")"
    | Triangle(a, b, c) -> "Triangle(...)"
```

**Deriving** auto-generates trait implementations from the structure of a type:

```japl
type Point deriving(Eq, Ord, Show, Serialize) =
  { x: Float, y: Float }
```

Derivable traits include: `Eq`, `Ord`, `Show`, `Serialize`, `Deserialize`.

**Trait resolution:** At each call site requiring a trait, the compiler resolves the implementation by:
1. Looking for a direct `impl` for the concrete type.
2. Looking for a parametric `impl` matching the type structure.
3. If a `where` clause introduces the constraint, using the dictionary passed by the caller.

Resolution is deterministic: overlapping implementations are a compile error.

### 3.8 Effect Types

Effects track computational side effects in function signatures. Effects are declared after the `with` keyword.

#### 3.8.1 Effect Types

| Effect | Meaning |
|--------|---------|
| `Pure` | No effects (the default; not written) |
| `Io` | File system, console, clock, random |
| `Async` | Asynchronous operations |
| `Net` | Network access |
| `State[s]` | Local mutable state of type `s` |
| `Process[m]` | Process operations with mailbox type `m` |
| `Fail[e]` | May fail with error type `e` |

#### 3.8.2 Effect Algebra

Effects form a commutative, idempotent monoid:

- **Identity:** `Pure` (the empty effect set)
- **Composition:** `with E1, E2` is the union of `E1` and `E2`
- **Commutativity:** `with Io, Net` = `with Net, Io`
- **Idempotency:** `with Io, Io` = `with Io`

#### 3.8.3 Effect Hierarchy

```
Pure < State[s]
Pure < Fail[e]
Pure < Process
State[s] < Io
Net < Io
Process < Async
```

A function with effect set `E1` may call a function with effect set `E2` if and only if `E2` is a subset of `E1`.

#### 3.8.4 Effect Inference

Within a function body, effects are inferred from the operations performed. At module boundaries, effect signatures are checked against the inferred effects.

```japl
-- Pure function: no annotation needed
fn add(a: Int, b: Int) -> Int =
  a + b

-- Effectful function: effects listed after `with`
fn read_config(path: String) -> Config with Io, Fail[ConfigError] =
  let text = File.read_to_string(path)?   -- infers Io
  parse_config(text)?                       -- infers Fail[ConfigError]
```

#### 3.8.5 Effect Handlers

Effect handlers interpret effectful computations, discharging their effects:

```japl
-- Running a State effect
let result: Int = State.run(0, fn ->
  stateful_computation()
)

-- Catching a Fail effect
let result: Result[a, e] = Fail.catch(fn ->
  fallible_computation()
)
```

An effect handler `State.run(init, f)` takes a computation `f` with effect `State[s]` and produces a pure value. Similarly, `Fail.catch(f)` converts a computation with `Fail[e]` into a `Result[a, e]` value.

### 3.9 Resource Types

Resource types track mutable, externally-managed resources. See [Section 4](#4-ownership-and-linearity) for details.

The `Owned<T>` type (written `own T` in parameter position) represents exclusive ownership of a resource of type `T`. Resources are not part of the pure type hierarchy; they live in the resource layer and are governed by linear typing rules.

### 3.10 Capability Types

Capabilities are types representing unforgeable permissions to perform actions:

```japl
type FsCapability = capability {
  root: Path,
  permissions: FsPermissions,
}

fn read_file(cap: FsCapability, path: Path) -> Result[String, IoError] with Io =
  if Path.is_within(path, cap.root) && cap.permissions.read then
    File.read_to_string(path)
  else
    Err(PermissionDenied)
```

Capabilities cannot be forged; they must be granted by the runtime or a parent process. The capability system integrates with the effect system to control what operations are available in a given context.

---

## 4. Ownership and Linearity

JAPL employs a **dual-layer** memory model: a pure layer for immutable values and a resource layer for mutable resources. Each layer has distinct typing rules and memory management.

### 4.1 Pure Layer

**Semantics:** All values in the pure layer are immutable. Once constructed, a value cannot be observably modified. Values can be freely shared, duplicated, and discarded.

**Memory management:** Bump allocator in WASM linear memory. Because values are immutable, allocation is append-only within the linear memory region. A per-process generational collector is planned but not yet implemented.

**Typing rules:** Standard structural rules (weakening, contraction, exchange) apply. A pure value `x : T` may be used zero or more times.

```japl
let data = [1, 2, 3, 4, 5]
let copy = data  -- sharing is fine; data is immutable
```

### 4.2 Resource Layer

**Semantics:** Resources are mutable external handles (file handles, network sockets, GPU buffers, FFI pointers). Each resource has exactly one owner at any time. Resources must be consumed exactly once.

**Memory management:** Ownership-tracked via compile-time linearity checking (`--strict` mode). Resources must be consumed exactly once. Deterministic release at scope exit is planned but currently requires explicit consumption.

**Typing rules:** Linear typing rules (no weakening, no contraction) apply. A resource `x : own T` must be used exactly once.

#### 4.2.1 Typing Judgments

The dual-layer type system uses a mixed context `Gamma; Delta` where:
- `Gamma` is the unrestricted (pure) context: variables may be used any number of times.
- `Delta` is the linear (resource) context: variables must each be used exactly once.

```
Gamma; Delta |- e : T
```

#### 4.2.2 Core Typing Rules

**Variable (Unrestricted):**
```
    x : T in Gamma
    ────────────────
    Gamma; . |- x : T
```

**Variable (Linear):**
```
    ──────────────────────
    Gamma; x : T |- x : T
```

**Linear Abstraction:**
```
    Gamma; Delta, x : A |- e : B
    ─────────────────────────────
    Gamma; Delta |- fn(x) -> e : A -o B
```

**Linear Application:**
```
    Gamma; Delta1 |- e1 : A -o B     Gamma; Delta2 |- e2 : A
    ──────────────────────────────────────────────────────────
    Gamma; Delta1, Delta2 |- e1(e2) : B
```

**Promotion (Pure to Linear):**
```
    Gamma; . |- e : A
    ─────────────────
    Gamma; . |- e : !A
```

This rule embeds pure values into the linear context. Pure values implicitly carry the exponential modality `!`, meaning they can be freely duplicated and discarded.

### 4.3 Resource Lifecycle

Resources follow a linear lifecycle: **acquire**, **use**, **release**.

```japl
fn process_file(path: String) -> Result[String, IoError] with Io =
  -- Acquire: `use` binds a linear resource
  use file = File.open(path, Read)?
  -- Use: borrow for reading
  let contents = File.read_all(file)?
  -- Release: ownership consumed
  File.close(file)
  Ok(contents)
```

The `use` keyword introduces a linear binding. Failing to consume a `use`-bound resource is a compile error.

### 4.4 Ownership Transfer

Ownership can be transferred via function parameters qualified with `own`:

```japl
fn send_to_worker(buf: own Buffer, pid: Pid[WorkerMsg]) -> Unit =
  Process.send(pid, ProcessBuffer(buf))
  -- `buf` is moved; using it here is a compile error
```

After transfer, the original binding is consumed and cannot be referenced.

### 4.5 Borrowing

The `ref` qualifier allows temporary, read-only access to a resource without consuming it:

```japl
fn peek(buf: ref Buffer) -> Byte =
  Buffer.get(buf, 0)
```

Borrowing rules:
1. A `ref` borrow does not consume the resource.
2. The resource cannot be consumed (moved or closed) while any `ref` borrow is live.
3. Multiple `ref` borrows may coexist.
4. There are no mutable borrows; mutation of a resource requires exclusive ownership (`own`).

### 4.6 Resource Release [PLANNED]

**Note:** Region-based inference for automatic resource release is planned but not yet implemented. Currently, the compiler performs compile-time linearity checking (`--strict` mode) to verify that resources are consumed exactly once. Automatic insertion of release operations at scope boundaries is a future goal.

### 4.7 Ownership Summary

| Layer | Mutability | Memory | Sharing | Typing |
|-------|-----------|--------|---------|--------|
| Pure | Immutable | Bump allocator (WASM linear memory) | Free | Unrestricted |
| Resource | Mutable | Ownership-tracked, deterministic | Single owner | Linear |

---

## 5. Expression Semantics

### 5.1 Evaluation Strategy

JAPL uses **strict (eager) evaluation** with **left-to-right** evaluation order. All arguments to a function are fully evaluated before the function body executes.

**Rationale:** Strict evaluation provides predictable memory usage, predictable performance, and stack traces that correspond to source code order. Lazy evaluation is available explicitly via thunks (`fn() -> expr`).

### 5.2 Let Binding

```japl
let x = expr1
expr2
```

Evaluates `expr1`, binds the result to `x`, then evaluates `expr2` with `x` in scope. The binding is irrevocable: `x` cannot be reassigned.

**Typing rule:**
```
    Gamma |- e1 : T1     Gamma, x : T1 |- e2 : T2
    ────────────────────────────────────────────────
    Gamma |- let x = e1; e2 : T2
```

### 5.3 Function Application

```japl
f(a, b, c)
```

Evaluates `f`, then `a`, `b`, `c` left-to-right, then applies `f` to the arguments.

**Typing rule:**
```
    Gamma |- f : (T1, T2, ..., Tn) -> R     Gamma |- ei : Ti  for each i
    ──────────────────────────────────────────────────────────────────────
    Gamma |- f(e1, ..., en) : R
```

### 5.4 Pattern Matching

```japl
match expr with
| Pattern1 -> body1
| Pattern2 -> body2
| _ -> default_body
```

Evaluates `expr`, then tests each pattern in order from top to bottom. The first matching pattern binds its variables and the corresponding body is evaluated.

**Exhaustiveness:** The compiler requires that pattern match expressions cover all possible values of the scrutinee's type. A non-exhaustive match is a compile error.

**Pattern forms:**

| Pattern | Matches |
|---------|---------|
| `x` | Any value, binds to `x` |
| `_` | Any value, discards |
| `Constructor(p1, ..., pn)` | Value built with `Constructor` whose fields match `p1`...`pn` |
| `{ f1 = p1, f2 = p2 }` | Record with fields matching the given patterns |
| `[p1, p2, ..rest]` | List with head matching `p1`, `p2`, tail matching `rest` |
| `[]` | Empty list |
| `42`, `"hello"`, `True` | Literal values |
| `^x` | Pinned variable: matches the current value of `x` |

**Guards** add boolean conditions:

```japl
match value with
| x if x > 0 -> "positive"
| x if x < 0 -> "negative"
| _ -> "zero"
```

When guards are used, the compiler may require a catch-all pattern to ensure exhaustiveness.

### 5.5 If-Then-Else

```japl
if condition then true_branch else false_branch
```

Evaluates `condition`. If `True`, evaluates `true_branch`; if `False`, evaluates `false_branch`. Both branches must have the same type.

**Typing rule:**
```
    Gamma |- c : Bool     Gamma |- e1 : T     Gamma |- e2 : T
    ──────────────────────────────────────────────────────────
    Gamma |- if c then e1 else e2 : T
```

### 5.6 Pipe Operator

```japl
x |> f
```

Equivalent to `f(x)`. The pipe operator passes the left-hand expression as the first argument to the right-hand function.

```japl
raw_data
  |> parse_csv
  |> List.filter(fn row -> row.amount > 0)
  |> List.map(to_transaction)
  |> List.sort_by(fn t -> t.date)
```

The pipe operator is left-associative: `a |> f |> g` means `g(f(a))`.

**Typing rule:**
```
    Gamma |- e : A     Gamma |- f : A -> B
    ───────────────────────────────────────
    Gamma |- e |> f : B
```

### 5.7 Forward Composition Operator

```japl
let process = validate >> transform >> store
```

`f >> g` produces a new function equivalent to `fn x -> g(f(x))`. It is left-associative and requires that the output type of `f` matches the input type of `g`.

**Typing rule:**
```
    Gamma |- f : A -> B     Gamma |- g : B -> C
    ─────────────────────────────────────────────
    Gamma |- f >> g : A -> C
```

### 5.8 Record Operations

**Creation:**
```japl
let p = { x = 1.0, y = 2.0 }
```

**Field access:**
```japl
let val = p.x
```

**Update (functional):**
```japl
let p2 = { p | x = 3.0 }
```

This creates a new record identical to `p` except for the `x` field. The original `p` is unchanged.

### 5.9 Binary Operations

All arithmetic and comparison operators are desugared to function calls on the appropriate types. For example, `a + b` is `Int.add(a, b)` when both operands are `Int`.

Type-directed overloading: the compiler selects the implementation based on the operand types. Only built-in numeric types (`Int`, `Float`, `Float32`) support arithmetic operators. User-defined types do not support operator overloading.

### 5.10 Loop Expressions

```japl
loop i = 0, acc = 0 while i < n do
  continue(i + 1, acc + i)
```

A `loop` expression introduces named bindings and a `while` guard. The body must invoke `continue(...)` to advance to the next iteration with new binding values, or evaluate to a final expression to exit the loop.

Loops are syntactic sugar for tail-recursive functions. The compiler guarantees tail-call optimization for loop constructs.

---

## 6. Process Semantics

JAPL uses Erlang-style lightweight processes as the sole concurrency primitive. Processes are isolated: they share no mutable memory and communicate only through message passing.

### 6.1 Process Properties

1. **Isolated:** Each process runs as a separate WASM instance on an OS thread managed by the japl-runtime scheduler. Process count is bounded by available OS threads, not Erlang-style green threads. Lightweight green-thread scheduling is planned.
2. **Isolated:** Each process has its own heap partition. No shared mutable state between processes.
3. **OS-scheduled:** Processes are currently scheduled by the OS via the japl-runtime thread pool. Preemptive scheduling with reduction counting is planned.
4. **Independent memory:** Each process has its own WASM linear memory with bump allocation. Per-process generational garbage collection is planned.

### 6.2 Process Creation

```japl
let pid: Pid[Msg] = Process.spawn(fn -> process_body())
```

`Process.spawn(f)` creates a new process that executes `f`. It returns a `Pid[Msg]` where `Msg` is the message type that the process accepts.

**Remote spawn:**

```japl
let pid = Process.spawn_on(remote_node, fn -> process_body())
```

Creates a process on a remote node. The returned `Pid[Msg]` is location-transparent.

**Typing rule (Spawn):**
```
    Gamma |- f : () -> Never with Process[A]
    ──────────────────────────────────────────
    Gamma |- Process.spawn(f) : Pid[A] with Process
```

### 6.3 Message Passing

**Send (asynchronous, non-blocking):**

```japl
Process.send(pid, message)
```

Places `message` in the mailbox of the process identified by `pid`. Send always succeeds for local processes (the message is buffered). For remote processes, send is best-effort.

**Typing rule (Send):**
```
    Gamma |- pid : Pid[A]     Gamma |- msg : A
    ───────────────────────────────────────────
    Gamma |- Process.send(pid, msg) : Unit with Process
```

**Receive (blocking):**

```japl
let msg = Process.receive()
```

Blocks until a message is available in the current process's mailbox. Returns a value of the process's declared message type.

**Receive with timeout:**

```japl
let msg = Process.receive_with_timeout(5000)
```

Returns `Option[Msg]`: `Some(msg)` if a message arrives within the timeout (milliseconds), `None` if the timeout expires.

**Selective receive:**

```japl
Process.receive_matching(fn msg ->
  match msg with
  | Priority(High, _) -> True
  | _ -> False
)
```

Scans the mailbox for the first message matching the predicate, leaving non-matching messages in place.

### 6.4 Typed Mailboxes

Each process has a single mailbox typed by its message type. The type system prevents sending messages of the wrong type.

```japl
type CounterMsg =
  | Increment
  | Decrement
  | GetCount(Reply[Int])

fn counter(count: Int) -> Never with Process[CounterMsg] =
  match Process.receive() with
  | Increment -> counter(count + 1)
  | Decrement -> counter(count - 1)
  | GetCount(reply) ->
      Reply.send(reply, count)
      counter(count)
```

The `Reply[T]` type represents a one-shot reply channel. It is used for synchronous request-response patterns. A `Reply[T]` value is linear: it must be used exactly once.

### 6.5 Process State Pattern

Processes manage state through recursive function calls. The process loop is a tail-recursive function that receives a message, computes new state, and calls itself with the updated state.

```japl
fn server_loop(state: ServerState) -> Never with Process[ServerMsg] =
  let msg = Process.receive()
  let new_state = handle_message(state, msg)
  server_loop(new_state)
```

This pattern avoids mutable state entirely: each "iteration" creates a new state value.

### 6.6 Process Lifecycle

```
Spawned --> Running --> (Waiting <--> Running) --> Exited(reason)
```

| State | Description |
|-------|-------------|
| `Spawned` | Process created but not yet scheduled |
| `Running` | Process is executing code |
| `Waiting` | Process is blocked on `receive` |
| `Exited(reason)` | Process has terminated |

Exit reasons:
- `Normal` -- process completed normally
- `CrashReason` -- process terminated due to a crash (see Section 8)

### 6.7 Links and Monitors

**Links** are bidirectional: if either linked process crashes, the other receives an exit signal.

```japl
Process.link(pid)
```

**Monitors** are unidirectional: the monitoring process receives a `ProcessDown` message when the monitored process exits.

```japl
let ref = Process.monitor(pid)
-- Later, receive:
match Process.receive() with
| ProcessDown(^ref, ^pid, reason) -> handle_failure(reason)
```

The `^` pin operator in the pattern ensures that the `ref` and `pid` match the specific values bound earlier.

### 6.8 Process Introspection

```japl
let info = Process.info(pid)
-- Returns: { status: ProcessStatus, message_queue_len: Int, memory: Int, ... }
```

The runtime provides built-in observability for processes: status, mailbox depth, memory usage, and current function.

---

## 7. Supervision

Supervision trees are built into the language and runtime. A supervisor is a process that monitors child processes and restarts them according to a declared strategy when they fail.

### 7.1 Supervisor Declaration

```japl
fn start_app() -> Pid[SupervisorMsg] with Process =
  Supervisor.start(
    strategy = OneForOne,
    max_restarts = 5,
    max_seconds = 60,
    children = [
      { id = "db_pool"
      , start = fn -> DbPool.start(config.database)
      , restart = Permanent
      , shutdown = Timeout(5000)
      },
      { id = "http_server"
      , start = fn -> HttpServer.start(config.http)
      , restart = Permanent
      , shutdown = Timeout(10000)
      },
      { id = "background_jobs"
      , start = fn -> JobRunner.start(config.jobs)
      , restart = Transient
      , shutdown = Timeout(30000)
      },
    ]
  )
```

### 7.2 Restart Strategies

| Strategy | Behavior |
|----------|----------|
| `OneForOne` | Only the crashed child is restarted |
| `AllForOne` | All children are restarted when one crashes |
| `RestForOne` | The crashed child and all children started after it are restarted |

### 7.3 Restart Policies

| Policy | Behavior |
|--------|----------|
| `Permanent` | Always restart the child when it exits |
| `Transient` | Restart only if the child exits abnormally |
| `Temporary` | Never restart the child |

### 7.4 Restart Intensity

- `max_restarts`: Maximum number of restarts allowed within `max_seconds` seconds.
- If the limit is exceeded, the supervisor itself crashes, propagating the failure up the supervision tree.

### 7.5 Shutdown Policies

| Policy | Behavior |
|--------|----------|
| `Timeout(ms)` | Send shutdown signal; wait up to `ms` milliseconds for graceful termination; force-kill if timeout expires |
| `Brutal` | Immediately terminate the child |

### 7.6 Child Specification Type

```japl
type ChildSpec = {
  id: String,
  start: fn() -> Never,
  restart: RestartPolicy,
  shutdown: ShutdownPolicy,
}

type RestartPolicy = Permanent | Transient | Temporary

type ShutdownPolicy =
  | Timeout(Int)
  | Brutal
```

### 7.7 Typed Crash Reasons

Unlike Erlang's untyped crash reasons, JAPL provides structured crash reasons:

```japl
type CrashReason =
  | Normal
  | AssertionFailed(String, Location)
  | ResourceExhausted(String)
  | InvariantViolation(String)
  | Timeout
  | Custom(String)
```

Supervisors can pattern-match on crash reasons to make informed restart decisions.

### 7.8 Supervision Tree Structure

```
            Application Supervisor
           /          |           \
     DB Pool      HTTP Server    Job Runner
     /    \        /    \            |
  Conn1  Conn2  Acc1   Acc2     Worker Pool
                                /    |    \
                             W1     W2     W3
```

Supervisors are hierarchical. When a supervisor cannot contain a failure (restart intensity exceeded), it crashes, and its own supervisor handles the escalation.

### 7.9 Formal Properties

**Crash containment:** A process failure cannot corrupt the state of any other process. This is guaranteed by process isolation: no shared mutable memory.

**Supervision liveness:** For any child with restart policy `Permanent`, if the child crashes and the restart intensity has not been exceeded, the child will eventually be restarted. Formally:

```
If P_i crashes and restarts(P_i) < max_restarts within max_seconds,
then eventually P_i is restarted with fresh initial state.
```

---

## 8. Error Handling

JAPL provides a **dual error model** that cleanly separates two kinds of failure.

### 8.1 Domain Errors: Result Types and the ? Operator

Domain errors represent expected, recoverable failures (parse errors, validation failures, not-found conditions). They are values tracked by the type system.

#### 8.1.1 The Result Type

```japl
type Result[a, e] =
  | Ok(a)
  | Err(e)
```

#### 8.1.2 The ? Operator

The `?` postfix operator propagates errors through the call chain:

```japl
fn get_user(id: UserId) -> Result[User, AppError] with Io =
  let row = Db.query_one(sql, [id])?
  validate_user(row)?
  Ok(to_user(row))
```

Desugaring of `expr?`:
```japl
match expr with
| Ok(val) -> val
| Err(e) -> return Err(e)
```

The `?` operator is valid only in functions whose return type is `Result[_, E]` or which have the `Fail[E]` effect.

#### 8.1.3 The Fail Effect

The `Fail[E]` effect is an algebraic effect for error signaling:

```japl
fn read_config(path: String) -> Config with Io, Fail[ConfigError] =
  let text = File.read_to_string(path)?
  parse_config(text)?
```

The `Fail[E]` effect can be handled with `Fail.catch`:

```japl
let result: Result[Config, ConfigError] = Fail.catch(fn ->
  read_config("/etc/app.conf")
)
```

#### 8.1.4 Error Type Composition

When composing functions with different error types, explicit conversion is required:

```japl
fn get_profile(id: UserId) -> Profile with Io, Fail[AppError] =
  let user = get_user(id)?
  let prefs = get_preferences(id) |> map_err(fn e -> DbError(show(e)))?
  { user, preferences = prefs }
```

### 8.2 Process Failures: Crash and Restart

Process failures represent unexpected conditions (invariant violations, corrupted state, unrecoverable resource loss). The process terminates and a supervisor restarts it.

```japl
fn critical_worker(state: State) -> Never with Process[WorkerMsg] =
  match Process.receive() with
  | ProcessTask(task) ->
      assert valid_invariant(state)
      let new_state = handle_task(state, task)
      critical_worker(new_state)
  | Shutdown ->
      cleanup(state)
      Process.exit(Normal)
```

If `assert` fails or any unhandled exception occurs, the process crashes. Its supervisor detects the crash via the process monitoring mechanism and applies the configured restart strategy.

### 8.3 Error/Crash Boundary

The two error modes are complementary:

| Aspect | Domain Errors | Process Failures |
|--------|--------------|------------------|
| **Nature** | Expected | Unexpected |
| **Mechanism** | `Result` + `Fail` effect | `crash` + supervision |
| **Scope** | Function / call chain | Process |
| **Recovery** | Caller handles the error | Supervisor restarts the process |
| **State** | Preserved | Discarded (fresh state on restart) |
| **Typing** | Algebraic error types | Typed crash reasons |

**Guideline:** If you know what went wrong and can describe it as a type, use `Result`/`Fail`. If state may be corrupted or the failure is unexpected, let the process crash.

### 8.4 Panic

`panic(message)` immediately terminates the current process with crash reason `AssertionFailed(message, location)`. It is intended for programming errors (violated invariants), not for expected failures.

### 8.5 Assert

`assert condition` evaluates `condition`. If `False`, the current process panics.

```japl
assert List.length(items) > 0
```

---

## 9. Distribution

JAPL treats distribution as a first-class language concern rather than a library afterthought. **[PROTOTYPE]** Distribution currently works between local processes with serialized tagged values over TCP. Cross-machine distribution with typed ADT messages has not been verified end-to-end.

### 9.1 Node Addressing

A node is a running instance of the JAPL runtime. Nodes are identified by addresses:

```japl
let node = Node.start(
  name = "web-1",
  cookie = Env.get("CLUSTER_COOKIE"),
  listen = "0.0.0.0:9000",
)
```

Nodes connect to each other via TCP:

```japl
let remote = Node.connect("worker-1.internal:9000")
```

### 9.2 Location-Transparent PIDs

Process identifiers (`Pid[Msg]`) are location-transparent. A PID may refer to a process on the local node or a remote node. The `Process.send` and `Process.receive` operations work identically regardless of location.

```japl
-- Same syntax, regardless of process location
Process.send(pid, message)
```

The `Net` effect marks functions that perform network operations, making distribution boundaries visible in types when needed.

### 9.3 Remote Process Spawning

```japl
let pid = Process.spawn_on(remote_node, fn -> image_processor())
```

The function body is serialized and sent to the remote node for execution. The returned PID is usable from the local node.

**Constraint:** The function passed to `spawn_on` must not close over non-serializable values (e.g., function closures, local resources). The compiler enforces this: all captured values must satisfy the `Serialize` constraint.

### 9.4 Type-Derived Serialization

JAPL derives serialization from algebraic data type definitions. Types that derive `Serialize` automatically generate efficient wire format encoders and decoders.

```japl
type JobRequest deriving(Serialize, Deserialize) = {
  id: JobId,
  payload: Bytes,
  priority: Priority,
}
```

**Serialization rules (inductively defined):**

1. All primitive types (`Int`, `Float`, `Bool`, `String`, `Bytes`, `Unit`) are serializable.
2. Products and sums of serializable types are serializable.
3. Container types (`List[a]`, `Map[k, v]`, `Option[a]`) preserve serializability if their element types are serializable.
4. `Pid[a]` is always serializable (PIDs are network addresses).
5. **Function types are NOT serializable.** Closures cannot cross node boundaries.
6. Named types with `deriving(Serialize)` are serializable if all field types are serializable.

**Soundness:** If `Serialize(T)` holds and `v : T`, then `deserialize(serialize(v)) = v`. Serialization is a faithful round-trip for all serializable types.

### 9.5 Protocol Versioning

When a type's definition changes between deployments, JAPL provides type compatibility rules for rolling upgrades:

**Compatible changes (no coordination required):**
- Adding a new variant to a sum type (existing variants unchanged)
- Adding an optional field to a record (with a default value)

**Incompatible changes (require coordination):**
- Removing a variant
- Changing a field's type
- Reordering constructors (binary format depends on tag order)

The compiler can check compatibility between two versions of a type definition and report whether a rolling upgrade is safe.

### 9.6 Service Discovery

```japl
let registry = Registry.connect("service-registry.local")
let workers = Registry.lookup(registry, "image-processor")
```

The runtime provides primitives for service registration and lookup. Processes can register themselves under a name, and other processes (on any node) can look them up.

### 9.7 Process Monitoring Across Nodes

```japl
Process.monitor(remote_pid)

match Process.receive() with
| ProcessDown(ref, pid, reason) -> handle_failure(reason)
```

Monitoring works across node boundaries. If the network connection to the remote node is lost, the monitor triggers with a `NodeDown` reason.

---

## 10. Module System

### 10.1 Module Declarations

A module is a named collection of types, functions, and sub-modules. Modules serve as namespaces, compilation units, and encapsulation boundaries.

```japl
module Http.Server

import Http.{Request, Response, Status}
import Json

fn handle_request(req: Request) -> Response with Io, Net =
  let body = Json.parse(req.body)
  match body with
  | Ok(data) -> Response.json(Status.Ok, data)
  | Err(_) -> Response.text(Status.BadRequest, "invalid JSON")
```

### 10.2 Visibility

By default, all declarations in a module are **public** (accessible from other modules).

**Opaque types** hide their representation:

```japl
module Map =
  opaque type Map[k, v]

  fn empty() -> Map[k, v] = ...
  fn insert(map: Map[k, v], key: k, value: v) -> Map[k, v] where Ord[k] = ...
```

Outside the `Map` module, code cannot construct or destructure `Map[k, v]` values directly. It can only use the module's public functions.

### 10.3 Imports

```japl
-- Import a module (access via qualified names)
import Http.Server

-- Import specific items
import Http.{Request, Response}

-- Module path separator is "."
Http.Server.handle_request(req)
```

### 10.4 Signatures (Module Types)

Signatures define the interface a module must satisfy:

```japl
signature KeyValueStore[k, v] =
  type Store
  fn create() -> Store with Io
  fn get(store: Store, key: k) -> Option[v] with Io
  fn set(store: Store, key: k, value: v) -> Unit with Io
  fn delete(store: Store, key: k) -> Unit with Io
```

A module satisfies a signature if it provides all required types and functions with compatible types and effects:

```japl
module RedisStore : KeyValueStore[String, String] =
  type Store = RedisConnection
  fn create() -> Store with Io = Redis.connect(default_config)
  fn get(store, key) with Io = Redis.get(store, key)
  fn set(store, key, value) with Io = Redis.set(store, key, value)
  fn delete(store, key) with Io = Redis.del(store, key)
```

### 10.5 Trait Implementations

Trait implementations are scoped to modules and are automatically imported when either the trait or the implementing type is in scope (orphan rule: an `impl` must be in the same module as either the trait or the type).

```japl
impl Show[Point] =
  fn show(p) = "(" ++ Float.to_string(p.x) ++ ", " ++ Float.to_string(p.y) ++ ")"
```

### 10.6 Module Compilation

Modules are compiled independently. When a module's implementation changes but its signature (exported types and function signatures) does not, downstream modules do not need recompilation. This property is critical for fast incremental builds.

---

## 11. Standard Library (Core Types)

### 11.1 Core Data Types

| Type | Description |
|------|-------------|
| `List[a]` | Singly-linked immutable list |
| `Map[k, v]` | Immutable hash-array mapped trie (requires `Ord[k]`) |
| `Set[a]` | Immutable set (requires `Ord[a]`) |
| `Option[a]` | `Some(a)` or `None` |
| `Result[a, e]` | `Ok(a)` or `Err(e)` |

### 11.2 Numeric Types

| Type | Description |
|------|-------------|
| `Int` | Arbitrary-precision integer |
| `Float` | 64-bit IEEE 754 |
| `Float32` | 32-bit IEEE 754 |

Core functions: `Int.to_string`, `Int.parse`, `Float.to_string`, `Float.round`, `Float.ceil`, `Float.floor`, arithmetic operators.

### 11.3 Text and Bytes

| Type | Description |
|------|-------------|
| `String` | Immutable UTF-8 string |
| `Bytes` | Immutable byte sequence |
| `Char` | Unicode scalar value |

Core functions: `String.length`, `String.concat`, `String.split`, `String.contains`, `String.trim`, `String.to_bytes`, `Bytes.length`, `Bytes.slice`, `Bytes.to_string`.

### 11.4 Process Types

| Type | Description |
|------|-------------|
| `Pid[msg]` | Process identifier with typed message protocol (compiler-checked; runtime uses serialized tagged values) |
| `Reply[a]` | One-shot reply channel (linear) |
| `Ref` | Unique monitor reference |

Core functions: `Process.spawn`, `Process.spawn_on`, `Process.send`, `Process.receive`, `Process.receive_with_timeout`, `Process.receive_matching`, `Process.link`, `Process.monitor`, `Process.exit`, `Process.info`, `Reply.send`.

### 11.5 IO Types

| Type / Module | Description |
|---------------|-------------|
| `File` | File operations |
| `Socket` | TCP/UDP socket operations |
| `Path` | File system path manipulation |
| `Io` | Console, clock, random |

Core functions: `File.open`, `File.read_all`, `File.read_to_string`, `File.write`, `File.close`, `Io.println`, `Io.read_line`, `Io.clock`.

### 11.6 Time

| Type | Description |
|------|-------------|
| `Time` | Absolute timestamp |
| `Duration` | Time interval |

Core functions: `Time.now`, `Time.diff`, `Duration.from_millis`, `Duration.from_seconds`.

### 11.7 Node and Distribution

| Type | Description |
|------|-------------|
| `Node` | A JAPL runtime instance |
| `Registry` | Service registry |

Core functions: `Node.start`, `Node.connect`, `Node.self`, `Registry.register`, `Registry.lookup`.

### 11.8 Standard Library Modules

```
Std.Http      -- HTTP client and server
Std.Json      -- JSON encoding/decoding
Std.Crypto    -- Cryptographic primitives
Std.Fs        -- File system operations
Std.Net       -- TCP/UDP networking
Std.Test      -- Testing framework
Std.Time      -- Time and duration
Std.Log       -- Structured logging
Std.Trace     -- Distributed tracing
```

---

## 12. FFI

JAPL provides a Foreign Function Interface to C, Rust, and WASM. All FFI calls are capability-controlled.

### 12.1 Foreign Function Declarations

```japl
foreign "C" fn sqlite3_open(filename: CString, db: Ptr[Ptr[Sqlite3]]) -> CInt
```

This declares an external C function. The compiler generates the necessary calling convention glue.

### 12.2 WASM Foreign Modules

```japl
foreign "wasm" module "image_codec" =
  fn encode_png(data: Bytes, width: Int, height: Int) -> Bytes
  fn decode_png(data: Bytes) -> Result[Image, CodecError]
```

### 12.3 Unsafe Blocks

Foreign functions are called within `unsafe` blocks:

```japl
fn open_database(path: String) -> Result[Database, DbError] with Io =
  use filename = CString.from(path)
  use db_ptr = Ptr.alloc[Ptr[Sqlite3]]()
  let rc = unsafe sqlite3_open(filename, db_ptr)
  if rc == 0 then
    Ok(Database.from_raw(Ptr.read(db_ptr)))
  else
    Err(DbError.from_code(rc))
```

The `unsafe` keyword marks code where the compiler cannot verify safety. Unsafe blocks:
- May call foreign functions
- May perform raw pointer operations
- May violate linearity constraints

Unsafe blocks are tracked by the type system: a function containing `unsafe` must have the `Io` effect.

### 12.4 Capability Wrapping

Foreign resources should be wrapped in safe JAPL interfaces that restore the safety guarantees:

```japl
-- Raw FFI: unsafe, untracked
foreign "C" fn fopen(path: CString, mode: CString) -> Ptr[CFile]
foreign "C" fn fread(buf: Ptr[U8], size: Int, count: Int, file: Ptr[CFile]) -> Int
foreign "C" fn fclose(file: Ptr[CFile]) -> CInt

-- Safe wrapper: tracked by ownership and effects
module SafeFile =
  opaque type FileHandle

  fn open(path: String, mode: FileMode) -> Result[own FileHandle, IoError] with Io =
    use cpath = CString.from(path)
    use cmode = CString.from(mode_string(mode))
    let ptr = unsafe fopen(cpath, cmode)
    if Ptr.is_null(ptr) then Err(IoError.last())
    else Ok(FileHandle.from_raw(ptr))

  fn close(handle: own FileHandle) -> Unit with Io =
    let _ = unsafe fclose(FileHandle.to_raw(handle))
    ()
```

---

## 13. Built-in Test Framework

Testing is a first-class language feature, not a library.

### 13.1 Test Blocks

```japl
test "parsing a valid integer" =
  assert parse_int("42") == Ok(42)

test "parsing rejects non-numeric input" =
  assert parse_int("abc") == Err(InvalidInt("abc"))

test "user creation validates email" =
  let result = create_user("bad-email")
  assert result == Err(InvalidEmail("bad-email"))
```

Test blocks are top-level declarations. The compiler discovers all tests and the test runner executes them.

### 13.2 Assert Expressions

`assert expr` evaluates `expr`. If it is `True`, execution continues. If `False`, the test fails with a diagnostic message showing the expression, expected value, and actual value.

`assert expr1 == expr2` provides rich failure diagnostics:

```
FAILED: test "order total includes tax"
  assert total == 33.0
  left:  33.1
  right: 33.0
  at src/order_test.japl:15:3
```

### 13.3 Property-Based Testing

```japl
property "reversing a list twice is identity" =
  forall (xs: List[Int]) ->
    List.reverse(List.reverse(xs)) == xs

property "sort produces ordered output" =
  forall (xs: List[Int]) ->
    let sorted = List.sort(xs)
    is_sorted(sorted) && List.length(sorted) == List.length(xs)
```

The `forall` keyword introduces universally quantified test variables. The test runner generates random values and checks the property. If a counterexample is found, it is shrunk to a minimal failing case.

Types used in `forall` must implement the `Arbitrary` trait (auto-derivable for most types).

### 13.4 Benchmark Blocks

```japl
bench "fibonacci 30" =
  fibonacci(30)
```

Benchmarks measure execution time of the given expression. The test runner reports mean time, standard deviation, and throughput.

### 13.5 Running Tests

```
$ japl test                 -- run all tests
$ japl test --filter user   -- run tests matching "user"
$ japl test --parallel 8    -- parallel execution
$ japl test --coverage      -- with coverage report
$ japl test --property-seed 42  -- deterministic property tests
```

---

## Appendix A: Complete Keyword Table

| Keyword | Category | Description |
|---------|----------|-------------|
| `assert` | Expression | Runtime assertion |
| `bench` | Declaration | Benchmark block |
| `continue` | Expression | Loop continuation |
| `deriving` | Declaration | Auto-derive trait implementations |
| `do` | Expression | Loop body delimiter |
| `else` | Expression | Alternative branch |
| `fn` | Declaration/Expression | Function definition or lambda |
| `forall` | Expression | Universal quantification (tests) |
| `foreign` | Declaration | FFI binding |
| `if` | Expression | Conditional |
| `impl` | Declaration | Trait implementation |
| `import` | Declaration | Module import |
| `let` | Expression | Value binding |
| `loop` | Expression | Iterative loop |
| `match` | Expression | Pattern matching |
| `module` | Declaration | Module definition |
| `opaque` | Modifier | Hide type representation |
| `own` | Modifier | Ownership qualifier |
| `packed` | Modifier | Packed memory layout |
| `property` | Declaration | Property-based test |
| `ref` | Modifier | Borrow qualifier |
| `receive` | Expression | Process message reception |
| `send` | Expression | Process message send |
| `signature` | Declaration | Module type / interface |
| `spawn` | Expression | Process creation |
| `strategy` | Supervisor | Supervision strategy |
| `supervisor` | Declaration | Supervisor definition |
| `test` | Declaration | Unit test |
| `then` | Expression | Consequent branch |
| `trait` | Declaration | Trait (type class) definition |
| `type` | Declaration | Type definition |
| `unsafe` | Expression | Escape safety guarantees |
| `use` | Expression | Linear resource binding |
| `where` | Clause | Trait constraints |
| `while` | Expression | Loop guard |
| `with` | Clause | Effect annotation |

## Appendix B: Built-in Type Constructors

| Constructor | Kind | Description |
|-------------|------|-------------|
| `List` | `Type -> Type` | Singly-linked list |
| `Option` | `Type -> Type` | Optional value |
| `Result` | `(Type, Type) -> Type` | Success or failure |
| `Map` | `(Type, Type) -> Type` | Key-value map |
| `Set` | `Type -> Type` | Unique element set |
| `Pid` | `Type -> Type` | Process identifier |
| `Reply` | `Type -> Type` | One-shot reply channel |

## Appendix C: Toolchain Commands

| Command | Description |
|---------|-------------|
| `japl build` | Compile project (Cranelift backend, fast) |
| `japl build --release` | Compile optimized (LLVM backend) |
| `japl build --target <target>` | Cross-compile |
| `japl build --static` | Produce static binary |
| `japl run` | Compile and execute |
| `japl test` | Run all tests |
| `japl fmt` | Format source (opinionated, no config) |
| `japl doc` | Generate documentation |
| `japl deps` | Dependency management |
| `japl release` | Cross-compilation and packaging |

Supported targets: `linux-amd64`, `linux-arm64`, `darwin-arm64`, `darwin-amd64`, `windows-amd64`, `wasm32`.

## Appendix D: Effect Compatibility Table

A function with effects in column can call functions with effects in row:

| Callee \ Caller | Pure | Fail[e] | State[s] | Process | Net | Io | Async |
|-----------------|------|---------|----------|---------|-----|------|-------|
| Pure | Y | Y | Y | Y | Y | Y | Y |
| Fail[e] | N | Y | N* | N* | N* | N* | N* |
| State[s] | N | N | Y | N | N | Y | Y |
| Process | N | N | N | Y | N | N | Y |
| Net | N | N | N | N | Y | Y | Y |
| Io | N | N | N | N | N | Y | Y |
| Async | N | N | N | N | N | N | Y |

*N*: Allowed only if the caller also has `Fail[e]` in its effect set.

The general rule: a callee's effects must be a subset of the caller's effects.
