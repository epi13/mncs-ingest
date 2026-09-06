# AGENTS.md

## Mission

`mncs-ingest` is the first-class MNCS boundary that converts heterogeneous external representations into canonical machine-native semantic fragments.

## Architectural constraints

Agents working in this repository must preserve these boundaries:

- ingest interprets and normalizes incoming observations;
- `mncs-memory` owns retention, contradiction history, reinforcement, decay, retrieval, and forgetting;
- model systems/MNEL own learned model execution and weight lifecycle;
- Atlas/Fabric and other orchestrators own system-level agent/process orchestration;
- source adapters may be modality-specific, but the canonical semantic IR must remain modality-neutral;
- provenance, confidence, source, uncertainty, and relevant temporal/scope context are first-class structural data, not optional prose metadata.

## Language pressure

When an executable semantic decision becomes authoritative, prefer implementing that decision in MNCS Language and calling it from host/runtime code rather than reproducing policy in the host language.

Do not add decorative `.mncs` examples solely to claim MNCS usage. MNCS source must execute in tested paths before it can support a conformance claim.

If language limitations prevent a clean implementation, capture the limitation as explicit pressure on `mncs-language` rather than silently moving the semantic policy into another language.

## Development discipline

- Keep the primitive semantic vocabulary small and compositional.
- Avoid building a universal hand-authored ontology.
- Prefer deterministic canonicalization where the input semantics permit it.
- Preserve raw source references/provenance sufficiently for audit and replay.
- Separate extraction confidence from downstream truth or memory confidence.
- Test semantic convergence across different surface representations.
- Keep adapters replaceable and canonical IR stable enough to be shared across consumers.
- Make conformance claims narrow and evidence-backed.

## Initial definition of done

The first implementation slice is complete only when multiple distinct representations of the same bounded observation deterministically converge to equivalent canonical semantic fragments, provenance survives the conversion, native MNCS participates in an executed path, and at least one downstream consumer can accept the fragment without taking an ingest dependency on that consumer's internal policy.
