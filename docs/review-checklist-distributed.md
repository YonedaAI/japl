# JAPL Distributed Feature Review Checklist

## Distributed Claim Checklist

For any feature or PR claiming distributed behavior:

- [ ] Does it work with `japl run --distributed`?
- [ ] Is process messaging through NATS?
- [ ] Can an external client observe the behavior?
- [ ] Is it tested in the verification suite?
- [ ] Would the release gate fail if this broke?

## Shortcut Rejection Checklist

These do NOT prove distributed semantics and must not be cited as proof:

- [ ] `japl run` without `--distributed` (local only, proves nothing about distribution)
- [ ] Direct NATS CLI probing without a JAPL app running
- [ ] `--dry-run` manifest generation (proves template generation, not execution)
- [ ] Component build alone (proves compilation, not runtime behavior)
- [ ] Unit tests that mock NATS (proves mock behavior, not real transport)

## Provider Checklist

Before shipping a provider change:

- [ ] Provider starts and passes self-test (`japl.runtime.health` responds)
- [ ] `spawn` creates a process reachable via NATS subject
- [ ] `send` delivers a message to the spawned process mailbox
- [ ] `receive` retrieves the message from the mailbox
- [ ] Process cleanup fires for stale processes
- [ ] Mailbox size limit (10K) is enforced
- [ ] Provider works with `japl run --distributed` end-to-end
- [ ] At least one real app (kvstore or msgqueue) runs successfully

## Documentation Checklist

When documenting distributed features:

- [ ] Proof level is stated (LOCAL, PROVIDER, DISTRIBUTED, DEPLOYED)
- [ ] Known blockers are listed (e.g., wash 2.0.1 config parsing)
- [ ] The distinction between sidecar and native provider is clear
- [ ] Claims match the actual verification suite results
- [ ] Feature matrix is updated with correct proof level
