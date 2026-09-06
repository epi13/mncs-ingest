# Schemas

This directory will hold portable schemas for versioned MNCS Ingest interchange and validation once the first executable semantic IR is selected.

The schema layer should describe the canonical fragment contract without becoming the source of semantic policy.

Expected concerns include:

- IR version identity;
- atom/type encoding;
- relation encoding;
- frame boundaries;
- provenance/source references;
- confidence and uncertainty representation;
- deterministic serialization requirements;
- forward/backward compatibility rules.

Do not treat JSON as the permanent canonical format merely because early fixtures may use it. The first schema should be chosen by executable pressure and should leave room for a compact machine-native encoding later.
