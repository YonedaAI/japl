# JAPL Distributed Feature Review Checklist

## First Principle

JAPL owns distributed semantics.

Reviewers must not accept evidence from a lower layer as proof of a higher-layer claim.

Examples:

- provider health is not proof of distributed JAPL execution
- component build is not proof of deployed process behavior
- wasmCloud host startup is not proof that JAPL runs there

## Claim Classification

Every distributed/runtime claim must be classified as one of:

- `LOCAL`
- `DISTRIBUTED_RUNTIME`
- `DEPLOY_SUBSTRATE`
- `DEPLOYED_ENGINE`
- `LIMITED`
- `EXPERIMENTAL`
- `BLOCKED`

If the claim is labeled `PROVEN`, it must also name the execution mode in which it is proven.

## Distributed Claim Checklist

For any feature or PR claiming distributed behavior:

1. What exact execution mode is being claimed?
2. Does the proof use a real JAPL app?
3. Does the proof exercise JAPL process behavior?
4. Can an external caller observe the behavior?
5. Is the proof automated?
6. Would the release gate fail if it broke?

If any answer is missing or weak, the claim is not closed.

## Shortcut Rejection Checklist

These do NOT prove distributed semantics and must not be accepted as closure evidence:

- `japl run` without `--distributed`
- direct NATS CLI probing without a JAPL app
- `--dry-run` manifest generation
- component build alone
- unit tests that mock transport/provider behavior
- docs that describe intended architecture without matching runtime proof

These can support development confidence. They do not prove distributed-by-default semantics.

## Current Canonical Distributed Proof

Today, the canonical proof path for distributed JAPL behavior is:

- `japl run --distributed`
- real JAPL apps
- NATS-backed process messaging
- observable behavior through external clients where applicable

Until wasmCloud runs JAPL apps end to end, reviewers must not treat it as the canonical proof path.

## wasmCloud Upgrade Checklist

Reviewers may treat wasmCloud as the canonical distributed engine only if all are true:

1. `japl deploy` runs a real JAPL app through wasmCloud
2. the app actually executes JAPL process semantics there
3. the proof is automated and release-blocking
4. provider/runtime identity is owned by deployed runtime state
5. docs match that shipped behavior exactly

If any item is false, wasmCloud remains `DEPLOY_SUBSTRATE` or `BLOCKED`, not `DEPLOYED_ENGINE`.

## Provider Checklist

Before shipping a provider change:

- does it improve JAPL semantics, not just provider internals?
- does it preserve or improve real app execution?
- does it improve distributed proof in the canonical path?
- does it change identity, mailbox, or failure semantics?
- if so, are those semantics documented and tested?

## Documentation Checklist

When documenting distributed features:

- proof level is stated explicitly
- current blockers are listed
- sidecar vs native provider distinction is explicit
- local runtime proof is never presented as deployed proof
- README, feature matrix, release report, and public site say the same thing
