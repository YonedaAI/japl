# JAPL Architecture Contract

## Core Rule

JAPL must own its distributed semantics.

That means the language/runtime defines:

- process identity
- spawn/send/receive
- mailbox semantics
- supervision semantics
- failure behavior
- distributed observability in JAPL terms

No deployment substrate gets to define those semantics for JAPL.

## Substrate Rule

wasmCloud may be used as a hosting and orchestration substrate.

It is not the source of truth for JAPL semantics.

If JAPL uses wasmCloud, the correct model is:

> JAPL runtime semantics first, wasmCloud substrate second.

This is the same distinction as:

- BEAM is the semantic runtime for Erlang
- infrastructure beneath BEAM is not what defines Erlang process semantics

For JAPL, the semantic truth must remain JAPL-owned even if execution is hosted on wasmCloud.

## Current Honest State

As of now:

- `japl run` is the local reference runtime
- `japl run --distributed` is the working distributed runtime path today
- `japl deploy` and wasmCloud integration are not yet the canonical proven distributed engine

The repo must not claim otherwise until deployed JAPL apps are actually proven running through wasmCloud end to end.

## What "Distributed by Default" Means

JAPL may claim “distributed by default” only when all of these are true:

1. The distributed semantics are defined by JAPL itself
2. Those semantics work in the canonical distributed execution path
3. That path is proven by black-box tests using real JAPL apps
4. The release gate fails if that path is broken

If any one of these is false, the claim is not closed.

## Canonical Execution Modes

### Local reference mode

- `japl run`
- `japl serve`

These prove:

- local semantics
- developer workflow
- local runtime behavior

They do not prove distributed-by-default semantics.

### Distributed runtime mode

- `japl run --distributed`

This currently proves:

- JAPL’s working distributed runtime semantics today
- NATS-backed distributed process messaging
- external client access through the HTTP gateway

This is the current canonical proof path for distributed behavior.

### Deployment substrate mode

- `japl deploy`
- wasmCloud

This may eventually become the canonical distributed engine path.

Until a deployed JAPL app is proven running end to end there, it remains:

- integration work
- deployment substrate work
- not the canonical proof of distributed-by-default semantics

## No-Shortcut Rule

No distributed claim may be closed using only:

- local `japl run`
- local `japl serve`
- direct NATS/provider probing
- `--dry-run` manifests
- component build only
- documentation-only reasoning

Every distributed claim must be backed by the execution mode it is claiming.

## Review Rule

Future reviews must distinguish these proof levels:

- `LOCAL`
- `DISTRIBUTED_RUNTIME`
- `DEPLOY_SUBSTRATE`
- `DEPLOYED_ENGINE`
- `LIMITED`
- `EXPERIMENTAL`
- `BLOCKED`

No result may be labeled `PROVEN` without naming the execution mode in which it is proven.

## Upgrade Rule For wasmCloud

wasmCloud may only be called the canonical distributed engine for JAPL when:

1. real JAPL apps run through `japl deploy`
2. the runtime/provider contract actually executes JAPL process semantics there
3. the release gate proves this automatically
4. docs, tests, and CLI behavior all agree

Until then, wasmCloud is a target substrate, not the semantic engine.
