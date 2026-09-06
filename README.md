# MNCS Ingest

MNCS Ingest is the machine-native data ingestion and semantic-normalization boundary for the MNCS ecosystem. It converts external signals and representations into canonical typed structures, relations, context, and graph-ready semantic fragments for memory, models, agents, runtimes, and other MNCS systems.

It is intentionally **not** a conventional tokenizer. The goal is to reduce dependence on surface-form token sequences by compiling heterogeneous observations toward a shared machine-native semantic representation.

## Why this exists

A conventional model front end often looks like:

```text
text -> tokenizer -> token IDs -> embeddings -> model
```

MNCS Ingest explores a different boundary:

```text
external signal
    -> segmentation
    -> canonicalization
    -> typed atoms
    -> relations
    -> context / provenance / confidence
    -> semantic graph fragment
    -> MNCS consumer
```

Natural language is only one frontend. JSON, source code, structured events, sensor data, image-derived observations, agent output, and native MNCS representations should be able to converge toward the same semantic substrate when they express the same underlying observation.

## Core boundary

MNCS Ingest owns:

- intake contracts for heterogeneous external signals;
- structural segmentation and boundary discovery;
- canonicalization of equivalent surface representations;
- typed semantic atoms;
- explicit relations between atoms;
- observation/event frames;
- source, provenance, confidence, time, scope, modality, and uncertainty context;
- canonical semantic-IR serialization and validation;
- adapter contracts for converting source representations into canonical form;
- routing hints that downstream systems may use without making ingest responsible for storage or reasoning.

MNCS Ingest does **not** own:

- long-term retention, reinforcement, decay, contradiction history, or retrieval policy (`mncs-memory`);
- neural training, model execution, specialist inference, or learned-weight lifecycle (MNEL/model systems);
- general agent orchestration (`mncs-atlas`, Fabric, or other orchestration layers);
- source-specific application logic;
- a fixed global ontology of every possible concept.

The boundary is: **interpret and normalize an observation, then hand off a canonical semantic fragment.**

## Canonical concepts

The first semantic IR should stay deliberately small. Initial primitives are expected to center on:

```text
Signal
Segment
Atom
Type
Relation
Frame
Context
Source
Provenance
Confidence
Time
Scope
Uncertainty
```

Higher-level concepts should be composed from these primitives rather than hard-coded into a giant ontology.

Example input:

```text
Alexander gave Finn 3 apples.
```

Conceptual canonical form:

```text
entity Alexander
entity Finn
entity apple
value 3

event transfer
relation source(transfer, Alexander)
relation target(transfer, Finn)
relation object(transfer, apple)
relation quantity(transfer, 3)
```

Equivalent structured input should be able to converge toward materially the same representation.

## Relationship to MNCS Memory

`mncs-memory` should be an early serious consumer of this repository, but ingest is intentionally independent of memory.

```text
external representation
        |
        v
   mncs-ingest
        |
        v
canonical semantic fragment
        |
   +----+---------+-----------+
   |              |           |
   v              v           v
memory           models      agents/runtime
```

MNCS Ingest answers:

> What does this incoming signal structurally mean in canonical MNCS form?

MNCS Memory answers:

> How should this information be retained, related, reinforced, contradicted, retrieved, or forgotten?

## Adapter model

Source representations enter through adapters rather than changing the canonical IR for every modality.

```text
text adapter ---------+
json adapter ---------+
event adapter --------+--> canonical semantic IR
code adapter ----------+
vision adapter --------+
native MNCS -----------+
```

Native MNCS should require the least interpretation: where a source is already expressed in validated machine-native MNCS structures, ingest may only need validation, normalization, provenance attachment, and canonical serialization.

## Repository layout

```text
mncs-ingest/
├── README.md
├── AGENTS.md
├── adapters/              # source/modality adapter contracts
├── docs/
│   ├── ARCHITECTURE.md     # system boundary and data flow
│   ├── SEMANTIC_IR.md      # canonical representation design
│   └── ROADMAP.md          # staged implementation plan
├── examples/              # representation-convergence examples
├── language/
│   └── mncs/
│       └── ingest/         # authoritative MNCS-language policies/modules
└── schemas/                # portable interchange/validation schemas
```

The initial repository is architecture-first. Directories contain boundary documentation rather than fake implementations so the first executable slice can be driven by tests and real language pressure.

## First implementation target

The first vertical slice should prove one thing well:

> Multiple surface representations of the same simple observation converge to an equivalent canonical semantic fragment.

A useful first test set should include:

1. two or more natural-language phrasings;
2. one structured JSON/event representation;
3. one native MNCS representation;
4. deterministic canonical output;
5. provenance preserved across all inputs;
6. equivalence tests proving semantic convergence;
7. a consumer handoff into `mncs-memory` without putting memory policy inside ingest.

## Design principles

- **Machine-native first.** Human readability is useful, but not the canonical design constraint.
- **Meaning over surface form.** Equivalent observations should converge whenever practical.
- **Small primitives, compositional structure.** Avoid a brittle universal ontology.
- **Explicit relationships.** Preserve structure instead of forcing downstream models to rediscover it.
- **Provenance is structural.** Source and uncertainty travel with the observation.
- **Modality-neutral core.** Text is an adapter, not the architecture.
- **Executable MNCS authority.** Authoritative semantic decisions should move into real MNCS Language modules as implementation begins.
- **Bounded claims.** Tests and future conformance badges must prove only the behavior actually exercised.

## Status

This repository currently defines the first-class MNCS ingestion boundary and its initial layout. It does not yet claim a production tokenizer replacement, general semantic parser, multimodal model, universal ontology, or completed MNCS conformance boundary.

See [Architecture](docs/ARCHITECTURE.md), [Semantic IR](docs/SEMANTIC_IR.md), and [Roadmap](docs/ROADMAP.md).
