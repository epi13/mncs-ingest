//! Ingest envelopes (Tranche D): content identity + provenance.
//!
//! An envelope binds three things that must never collapse into each
//! other:
//!
//! - `source_id`: host sha256 over the exact raw source bytes (replay key);
//! - `normalized_digest`: pure-MNCS sha256 over the canonical record
//!   bytes (meaning identity — chunking-independent by construction);
//! - `provenance`: adapter, interpreter, spans, confidence (audit trail).
//!
//! Identical normalized values from different sources share the digest
//! and keep distinct source ids and provenance (Tranche I property,
//! tested). Raw source is never part of canonical identity.

use sha2::{Digest, Sha256};

use crate::error::IngestError;
use crate::ir::Provenance;
use crate::language::LanguageRuntime;

pub const ENVELOPE_SCHEMA: &str = "mncs-ingest/envelope-v1";

/// Canonical record bytes: fields joined with UNIT SEPARATOR (0x1F).
/// The separator cannot appear in field values (controls are rejected
/// upstream), so joining is injective.
pub fn canonical_record_bytes(fields: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(0x1f);
        }
        out.extend_from_slice(field);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestEnvelope {
    pub schema: String,
    pub source_id: String,
    pub normalized_digest: [u8; 32],
    pub provenance: Provenance,
}

impl IngestEnvelope {
    pub fn digest_hex(&self) -> String {
        self.normalized_digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

pub fn source_id_of(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Seal canonical bytes into an envelope: MNCS digests meaning, the
/// host identifies raw source, provenance travels alongside.
pub fn seal(
    rt: &LanguageRuntime,
    canonical: &[u8],
    raw: &[u8],
    provenance: Provenance,
) -> Result<IngestEnvelope, IngestError> {
    if canonical.len() > 256 {
        return Err(IngestError::Overlong {
            adapter: "envelope".to_owned(),
            detail: "canonical record exceeds 256 bytes".to_owned(),
        });
    }
    Ok(IngestEnvelope {
        schema: ENVELOPE_SCHEMA.to_owned(),
        source_id: source_id_of(raw),
        normalized_digest: rt.digest_record(canonical)?,
        provenance,
    })
}
