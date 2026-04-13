# JAPL Distributed Runtime Decision

## Question

What is the right path if JAPL wants to claim “distributed by default”?

## Wrong Answer

“Use wasmCloud and the problem is solved.”

That gives JAPL a deployment substrate, not distributed semantics by default.

## Correct Answer

JAPL must own its distributed runtime semantics itself.

That means:

- process identity
- message passing
- mailbox behavior
- supervision
- failure and restart semantics
- distributed observability

must be JAPL concepts first.

## Practical Options

### Option A: JAPL-native distributed runtime, substrate optional

JAPL owns the runtime semantics.

Substrates such as:

- local embedded wasmtime
- NATS transport
- wasmCloud hosting
- containers
- VMs

are just execution environments.

This is the strongest and cleanest model.

### Option B: Hybrid model

JAPL owns semantics.
wasmCloud hosts deployment/orchestration.

This is the most realistic path if JAPL wants portability and platform leverage.

But the language truth must still live in JAPL, not wasmCloud.

### Option C: wasmCloud as the semantic engine

This is the wrong model.

It creates semantic drift because wasmCloud is not an Erlang-like VM for JAPL.

## Recommended Direction

The best direction is:

> Build JAPL’s own distributed runtime model. Use wasmCloud only as a substrate if it helps deployment.

In short:

- JAPL is the runtime model
- wasmCloud is infrastructure

If later JAPL apps truly run end to end through wasmCloud, that is a deployment success, not a reason to stop owning the semantics.
