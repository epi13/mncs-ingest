# MNCS Ingest Architecture

## Purpose

MNCS Ingest is the boundary between external representation and machine-native MNCS semantics.

Its job is not to decide what should be remembered, believed, trained, or acted upon. Its job is to transform an incoming signal into a canonical semantic fragment with enough structure and provenance for downstream systems to make those decisions.

## Pipeline

The architectural data flow is:

```text
raw signal
  -> source adapter
  -> structural segmentation
  -> semantic extraction
  -> canonicalization
  -> typed atoms + relations
  -> frame construction
  -> context/provenance attachment
  -> semantic-IR validation
  -> consumer handoff
```

### 1. Signal

A signal is an input plus its source context. Examples include:

- UTF-8 text;
- JSON or another structured record;
- a system event;
- source code or AST-derived structure;
- sensor measurements;
- image/audio/model-derived observations;
- native MNCS values.

### 2. Adapter

Adapters interpret source-specific representation. They may use deterministic parsers, learned models, external model outputs, or native MNCS structures, but they must emit the common semantic contract.

Adapters should not own downstream memory or reasoning policy.

### 3. Segmentation

Segmentation identifies meaningful boundaries without assuming word/subword tokenization is the canonical unit. Depending on the source, a segment may be a phrase, field, event member, AST node, region, measurement, or already-typed native structure.

### 4. Semantic extraction

Extraction proposes atoms, relations, events, states, quantities, references, and other bounded semantic structure.

Extraction may be uncertain. Uncertainty must remain explicit rather than being flattened into apparently certain canonical data.

### 5. Canonicalization

Canonicalization reduces irrelevant surface differences while preserving meaningful distinctions.

Examples:

```text
Alexander gave Finn 3 apples.
Finn received three apples from Alexander.
```

For the bounded transfer meaning, both should be capable of producing equivalent event/relation structure.

Canonicalization is not license to erase ambiguity. If two interpretations remain plausible, the IR should represent that uncertainty or alternate structure.

### 6. Semantic fragment

The output is a graph-ready fragment composed from typed atoms, explicit relations, frames, and context.

A fragment is intentionally smaller than a knowledge graph or memory store. It represents the interpreted incoming observation before downstream persistence and reasoning policy.

## Consumer boundary

Expected consumers include:

```text
mncs-memory    <- retention and memory-state policy
MNEL/models    <- model routing, learning, inference
mncs-atlas     <- agent-facing interpretation
Fabric/runtime <- machine/system events
Commons        <- shared/distributed semantic exchange where appropriate
```

Consumers may enrich or transform the fragment, but ingest should not import their internal policy into its core representation.

## Native MNCS path

MNCS-native input is a special case because it may already encode typed semantics.

A target path is:

```text
native MNCS
  -> validate
  -> normalize
  -> attach source/provenance context
  -> canonical semantic fragment
```

This creates useful architectural pressure: representations closer to machine-native MNCS should require less interpretive work.

## Routing and micro-models

Ingest may attach routing hints based on semantic type or discovered relationships, but it should not own the lifecycle of a micro-model or specialist.

Conceptually:

```text
semantic fragment
   -> entity neighborhood
   -> quantity neighborhood
   -> event neighborhood
   -> temporal neighborhood
```

A downstream memory/model layer may use those hints to activate or update relevant micro-models.

## Stability target

Adapters should be easier to replace than the canonical IR. The IR should therefore evolve deliberately, with versioning and conformance tests once executable implementations begin.
