//! Downstream consumer handoff.
//!
//! The handoff is one canonical fragment in, one receipt out. The consumer
//! sees versioned atoms, relations, unknowns, and provenance — never
//! adapters, MNCS internals, or ingest configuration. Retention,
//! contradiction, reinforcement, retrieval, and every other memory/model
//! policy live downstream; this module proves the boundary, not the
//! policy. [`EchoConsumer`] is the decoupling test double.

use sha2::{Digest, Sha256};

use crate::error::IngestError;
use crate::ir::Fragment;

/// What a downstream system must accept without importing ingest policy.
pub trait FragmentConsumer {
    fn accept(&mut self, fragment: &Fragment) -> Result<ConsumerReceipt, IngestError>;
}

/// Deterministic receipt for one accepted fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerReceipt {
    pub signal_id: String,
    pub semantic_hash: String,
    pub atom_count: usize,
    pub relation_count: usize,
    pub unknown_count: usize,
}

pub fn receipt_for(fragment: &Fragment) -> ConsumerReceipt {
    let digest = Sha256::digest(fragment.canonical_bytes());
    ConsumerReceipt {
        signal_id: fragment.provenance.signal_id.clone(),
        semantic_hash: digest.iter().map(|b| format!("{b:02x}")).collect(),
        atom_count: fragment.atoms.len(),
        relation_count: fragment.relations.len(),
        unknown_count: fragment.unknowns.len(),
    }
}

/// Test consumer: stores accepted fragments and returns receipts. It
/// performs no retention or reasoning policy of its own.
#[derive(Debug, Default)]
pub struct EchoConsumer {
    received: Vec<Fragment>,
}

impl EchoConsumer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn received(&self) -> &[Fragment] {
        &self.received
    }
}

impl FragmentConsumer for EchoConsumer {
    fn accept(&mut self, fragment: &Fragment) -> Result<ConsumerReceipt, IngestError> {
        if fragment.schema != crate::ir::FRAGMENT_SCHEMA_VERSION {
            return Err(IngestError::SchemaMismatch {
                expected: crate::ir::FRAGMENT_SCHEMA_VERSION.to_owned(),
                got: fragment.schema.clone(),
            });
        }
        let receipt = receipt_for(fragment);
        self.received.push(fragment.clone());
        Ok(receipt)
    }
}
