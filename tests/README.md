# Tests

Tests should prove semantic behavior rather than only parser mechanics.

The first suite should establish:

- semantic convergence across multiple representations of the same bounded observation;
- semantic separation for observations that differ materially;
- deterministic canonical serialization/comparison;
- provenance and source context preservation;
- explicit unresolved/unknown/ambiguous handling;
- execution of authoritative MNCS Language policy where claimed;
- a clean consumer handoff that does not import downstream memory/model policy into ingest.

As the repository matures, any MNCS Actions badge should be bound to a named, bounded test surface from this suite rather than implying broader ecosystem conformance.
