# JAPL Truth Rules

These rules exist to stop architectural drift and dishonest closure.

## Rule 1: Verified behavior beats intended architecture

If the runtime path does not execute, the feature is not working, even if:

- the docs are written
- the manifest is generated
- the component builds
- the provider starts

## Rule 2: A lower layer cannot prove a higher-layer claim

Examples:

- NATS request/reply cannot prove deployed JAPL execution
- provider self-test cannot prove wasmCloud execution
- local runtime tests cannot prove distributed-by-default semantics

## Rule 3: “Done” requires black-box proof

A feature is not done until:

- a real JAPL app uses it
- the intended execution path is used
- the result is externally observable or otherwise black-box verifiable
- the proof is automated

## Rule 4: Proof must name the execution mode

All claims must specify whether they are proven in:

- local runtime
- distributed runtime
- deploy substrate
- deployed engine

## Rule 5: Docs do not get ahead of runtime truth

The docs may describe:

- shipped behavior
- blocked work
- experimental work

They may not describe intended architecture as current reality.

## Rule 6: Release gates are stronger than development checks

Development checks may skip infrastructure.

Release checks may not.

## Rule 7: JAPL owns semantics

Neither NATS nor wasmCloud defines JAPL’s process semantics.

They may host or transport them.

JAPL must define them.
