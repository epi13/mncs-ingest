# Adapters

Adapters convert source-specific representations into the shared MNCS Ingest semantic contract.

Examples may eventually include text, structured events/JSON, source-code structure, telemetry, model-derived multimodal observations, and native MNCS.

Adapter rules:

- source-specific parsing belongs here;
- canonical semantic meaning belongs in the shared IR, not in private adapter-only structures;
- adapters must preserve enough source reference/provenance for audit and replay;
- uncertainty discovered during extraction must remain explicit;
- adapters must not implement memory retention/retrieval policy;
- adapters should participate in convergence tests against equivalent observations from other modalities.

The first executable adapter set should stay small: text, structured event/JSON, and native MNCS are sufficient to pressure the core design.
