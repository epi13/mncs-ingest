# MNCS Ingest Roadmap

## Stage 0 — boundary definition

Status: initial repository scope.

- define ingest as a first-class MNCS subsystem;
- separate ingest from memory, model execution, and orchestration;
- define a small semantic-IR vocabulary;
- establish adapter and provenance requirements;
- avoid premature commitment to a universal ontology or final wire format.

## Stage 1 — deterministic convergence slice

Build the smallest executable path that proves semantic convergence.

Target observation class: a simple bounded event with entities, roles, object, and quantity.

Required inputs:

- at least two natural-language phrasings;
- one structured JSON/event representation;
- one native MNCS representation.

Required outputs:

- one versioned canonical semantic-fragment form;
- deterministic serialization/comparison;
- source/provenance preserved;
- explicit unresolved/unknown handling;
- tests showing equivalent meaning converges;
- tests showing meaningfully different observations do not collapse incorrectly.

## Stage 2 — authoritative MNCS-language policy

Move canonicalization and validation decisions that are genuinely semantic policy into executed MNCS Language modules.

Goals:

- host/runtime code handles transport and integration rather than duplicating semantic policy;
- language limitations become explicit issues/pressure on `mncs-language`;
- tests prove the MNCS modules are in the executed path;
- add an MNCS Actions conformance boundary only after there is executable behavior worth proving.

## Stage 3 — memory consumer integration

Integrate `mncs-memory` as the first serious downstream consumer.

Prove:

```text
source input
  -> mncs-ingest
  -> canonical semantic fragment
  -> mncs-memory
```

The integration must preserve the ownership boundary: ingest interprets the observation; memory decides retention, contradiction/supersession, reinforcement, retrieval, and lifecycle policy.

## Stage 4 — adapter pressure

Add heterogeneous adapters only as they create useful architectural pressure.

Candidate order:

1. text;
2. structured event/JSON;
3. native MNCS;
4. source-code/AST-derived observations;
5. machine/runtime telemetry;
6. model-produced vision/audio observations rather than raw multimodal modeling inside ingest.

Each adapter should prove convergence against shared semantic cases where practical.

## Stage 5 — routing and micro-model handoff

Explore semantic routing hints suitable for `mncs-memory`/MNEL micro-model systems.

Questions include:

- what routing information belongs in ingest versus downstream cognition;
- how explicit graph relations interact with learned latent relationships;
- whether canonical concept neighborhoods should use stable IDs;
- how local confidence/uncertainty affects model activation;
- how new concepts are represented before a specialist/micro-model exists.

## Stage 6 — portable semantic ABI

Once multiple consumers exist, evaluate whether the semantic IR should become a versioned MNCS cognitive/semantic ABI.

Possible pressure areas:

- compact binary encoding;
- WASM boundary transport;
- schema evolution;
- distributed semantic exchange;
- deterministic hashing/content addressing;
- rights/provenance integration;
- cross-runtime compatibility.

## Non-goals until pressure requires them

Do not prematurely build:

- a universal NLP stack;
- a giant hand-authored ontology;
- an embedding database;
- a vector search service;
- a general multimodal foundation model;
- memory lifecycle policy;
- online neural training;
- distributed storage;
- unsupported conformance claims.
