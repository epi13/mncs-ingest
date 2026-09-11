//! Controlled ingest errors.
//!
//! Every failure mode is an explicit variant. Ingest never panics on input
//! data and never silently drops information: unsupported or malformed input
//! becomes a typed error, while merely incomplete input becomes an explicit
//! unknown inside an otherwise valid [`crate::ir::Fragment`].

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IngestError {
    #[error("malformed input from {adapter}: {detail}")]
    Malformed { adapter: String, detail: String },

    #[error("unsupported construct from {adapter}: {detail}")]
    Unsupported { adapter: String, detail: String },

    #[error("missing required field from {adapter}: {field}")]
    MissingField { adapter: String, field: String },

    #[error("unexpected field from {adapter}: {field}")]
    UnexpectedField { adapter: String, field: String },

    #[error("conflicting fields from {adapter}: {detail}")]
    ConflictingFields { adapter: String, detail: String },

    #[error("input exceeds bounded slice limits from {adapter}: {detail}")]
    Overlong { adapter: String, detail: String },

    #[error("schema version mismatch: expected {expected}, got {got}")]
    SchemaMismatch { expected: String, got: String },

    #[error("MNCS language runtime failure: {0}")]
    Language(String),

    #[error("consumer handoff failure: {0}")]
    Consumer(String),
}

impl IngestError {
    pub fn malformed(adapter: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Malformed {
            adapter: adapter.into(),
            detail: detail.into(),
        }
    }

    pub fn unsupported(adapter: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Unsupported {
            adapter: adapter.into(),
            detail: detail.into(),
        }
    }

    pub fn missing_field(adapter: impl Into<String>, field: impl Into<String>) -> Self {
        Self::MissingField {
            adapter: adapter.into(),
            field: field.into(),
        }
    }
}
