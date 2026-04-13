# JAPL Done Criteria

## Distributed Feature Done Criteria

A distributed feature is done only if:

1. The semantics are defined in JAPL terms
2. A real JAPL app uses the feature
3. The intended execution mode is exercised
4. The behavior is automatically verified
5. The release gate would fail if it regressed
6. The docs describe exactly that shipped state

## wasmCloud Done Criteria

wasmCloud integration is done only if:

1. `japl deploy` runs a real JAPL app through wasmCloud
2. JAPL process behavior executes there, not just provider health checks
3. The release gate proves it without manual steps being ignored
4. Provider/runtime identity is runtime-derived
5. The provider architecture is documented honestly

Until then:

- wasmCloud is integration work
- not the canonical distributed engine

## Stdlib Done Criteria

A stdlib module is done only if:

1. It is runtime-backed or intentionally pure
2. Its core behavior is exercised by tests
3. It is not marked `STUB`, `SIMULATED`, or `LIMITED` unless that status is part of the release truth
4. The feature matrix reflects that exact status
