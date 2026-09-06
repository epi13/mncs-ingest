# MNCS Ingest Language Modules

This directory is reserved for authoritative MNCS Language modules used by executable ingest paths.

The intent is not to add decorative `.mncs` files. As implementation begins, semantic policy that belongs to ingest—such as canonicalization, validation, equivalence, or bounded routing decisions—should move here when MNCS Language can express it cleanly.

Host code may provide transport, parsing bridges, runtime integration, and foreign-function boundaries, but should not silently duplicate authoritative semantic decisions that are claimed to be MNCS-native.

Any missing language capability encountered while building the vertical slice should become explicit pressure on `mncs-language` and its stdlib/tooling.
