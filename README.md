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
├── schemas/                # portable interchange/validation schemas
└── tests/                  # semantic convergence and boundary proofs
```

The initial repository is architecture-first. Directories contain boundary documentation rather than fake implementations so the first executable slice can be driven by tests and real language pressure.

## Status: deterministic convergence slice (implemented)

The first vertical slice is executable. Four representations of one
bounded transfer observation — active text, passive text, structured
JSON, native MNCS codes — deterministically converge to one canonical
`mncs-ingest/fragment-v1` fragment (see
`examples/transfer/canonical.txt`), with provenance preserved and
semantic-vs-provenance equality separated.

### What works

- Executable semantic IR (`src/ir.rs`): entities, transfer event,
  objects, quantities, four roles, explicit unknowns, versioned schema.
- Executed MNCS authority (`language/mncs/ingest*.mncs`, Source Profile
  0.13): word classification, role assignment, quantity parsing,
  validation, spelling/codes equality, ordering primitives — all run
  through the pinned compiler, reference interpreter, and two real
  backends in tests.
- Three adapters behind one trait (`src/adapters/`): bounded text
  ([grammar](docs/GRAMMAR.md)), structured JSON events, native codes.
- Deterministic canonical bytes, sha256 signal ids, byte-span
  provenance, `Direct`/`Parsed`/`Uncertain` confidence.
- Consumer handoff (`src/consumer.rs`): one fragment in, one receipt
  out; `EchoConsumer` proves decoupling with no memory policy inside
  ingest.
- 45 tests pin convergence, separation, determinism, provenance,
  unknowns, malformed input, adapter equivalence, and MNCS authority
  (including a policy-mutation test that proves MNCS is in the path).

### What does not work (yet)

- One event kind (`transfer`), two known objects, ≤64-byte sentences.
- No articles, adverbs, tense beyond the listed verbs, number words
  past `ten`, or multi-frame observations.
- Reference-execution latency (~140 ms floor per verdict in release;
  see `docs/LANGUAGE_PRESSURE.md` P-PERF-01) — fine for tests, not for
  a high-frequency path; backend-execution integration is future work.
- JSON is the interchange/debug form; the machine-native encoding is
  undecided, as the architecture doc always intended.

### Build / run / test

```bash
cargo build --offline
cargo test --offline          # full suite (~5 min, MNCS elaboration dominates)
cargo run --offline --example gen   # regenerate examples/transfer fixtures
```

MNCS dependencies pin `mncs-language` rev
`d7cc9536f0507fc34086d8a5784c6177ce5b67b4` (see `Cargo.toml`); the
`.mncs` sources under `language/` declare Source Profile 0.13.

### Language pressure

Eleven evidence-backed pressures from this implementation live in
[docs/LANGUAGE_PRESSURE.md](docs/LANGUAGE_PRESSURE.md) — strings and
spans, maps, fold liveness, strict `select`, nested-module
diagnostics, stdlib distribution, and measured execution cost.

### Layout

```text
mncs-ingest/
├── Cargo.toml / Cargo.lock
├── src/                     # host carriers: IR, canonical layout, adapters, runtime, consumer
├── language/mncs/ingest*.mncs  # authoritative MNCS policy (executed, tested)
├── tests/                   # semantic guarantee suites
├── examples/transfer/       # convergence fixtures + generator
├── schemas/fragment-v1.schema.json
└── docs/ARCHITECTURE.md docs/SEMANTIC_IR.md docs/ROADMAP.md
    docs/GRAMMAR.md docs/LANGUAGE_PRESSURE.md
```

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
