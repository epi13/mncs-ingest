# MNCS Ingest Semantic IR

## Goal

The semantic IR is a canonical, modality-neutral representation of an interpreted observation.

It should be compact enough to share across MNCS systems, explicit enough to preserve relationships that would otherwise need to be relearned downstream, and simple enough to evolve under test pressure.

## Initial primitives

The first design should center on a small set of compositional primitives:

### Signal
The original incoming representation plus source identity and acquisition context.

### Segment
A bounded portion of the signal used during interpretation. Segments are source-specific and are not necessarily linguistic tokens.

### Atom
A typed semantic unit such as an entity, value, event, state, property, quantity, location, or time reference.

### Type
A machine-readable semantic class or role attached to an atom or relation.

### Relation
A directed, typed connection between semantic units.

### Frame
A bounded observation/event/state structure that groups atoms and relations that belong together.

### Context
Information that qualifies interpretation, including provenance, source, time, scope, modality, confidence, and uncertainty.

## Conceptual shape

The following is illustrative, not yet a committed wire format:

```text
fragment {
  atoms: [
    { id: a1, type: entity, canonical: "Alexander" },
    { id: a2, type: entity, canonical: "Finn" },
    { id: a3, type: object, canonical: "apple" },
    { id: a4, type: quantity, value: 3 },
    { id: e1, type: event, canonical: transfer }
  ]

  relations: [
    source(e1, a1),
    target(e1, a2),
    object(e1, a3),
    quantity(e1, a4)
  ]

  context: {
    source: <source-ref>,
    provenance: <provenance-ref>,
    confidence: <bounded-value>,
    modality: text
  }
}
```

## Semantic convergence

The IR exists to make equivalence testable.

For a bounded meaning, these inputs:

```text
Alexander gave Finn 3 apples.
Finn received three apples from Alexander.
```

and a structured record such as:

```json
{
  "event": "transfer",
  "from": "Alexander",
  "to": "Finn",
  "object": "apple",
  "quantity": 3
}
```

should be capable of converging on equivalent canonical structure.

Equivalence does not require byte-for-byte identity in every intermediate stage. It requires deterministic canonical comparison rules for the semantic content the system claims to understand.

## Ambiguity

The IR must not manufacture certainty.

Where interpretation is ambiguous, implementations should be able to preserve:

- multiple candidate frames;
- confidence per candidate or relation;
- unresolved references;
- unknown types;
- source spans/segments supporting each interpretation.

## Provenance

Provenance is part of the structure, not an optional logging feature.

A downstream consumer should be able to determine where a semantic assertion came from without requiring access to an opaque model conversation or transient adapter state.

## Canonicalization rules

Canonicalization should normalize representation while preserving semantic distinctions. Likely early rules include:

- stable relation role names;
- normalized numeric values and units;
- canonical entity/reference identifiers when resolvable;
- deterministic ordering for serialized structures;
- explicit unknown/unresolved states;
- versioned IR schema identity.

## What is intentionally unresolved

This document does not yet commit to:

- a binary encoding;
- JSON as the authoritative representation;
- globally stable concept IDs;
- embedding/vector fields in the canonical core;
- ontology ownership;
- learned latent-edge storage;
- micro-model ownership;
- a final confidence calculus.

Those decisions should be made only when an executable vertical slice creates real pressure for them.
