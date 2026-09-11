//! Consumer handoff: ingest hands a canonical fragment to a downstream
//! system without importing that system's policy.

mod common;

use common::rt;
use mncs_ingest::adapters::{Adapter, TextAdapter};
use mncs_ingest::consumer::{receipt_for, EchoConsumer, FragmentConsumer};
use mncs_ingest::ir::FRAGMENT_SCHEMA_VERSION;

#[test]
fn consumer_accepts_canonical_fragment() {
    let rt = rt();
    let ingested = TextAdapter
        .ingest(&rt, "Alexander gave Finn 3 apples.")
        .unwrap();
    let mut consumer = EchoConsumer::new();
    let receipt = consumer
        .accept(&ingested.fragment)
        .expect("consumer accepts");
    assert_eq!(receipt.signal_id, ingested.fragment.provenance.signal_id);
    assert_eq!(receipt.atom_count, 5);
    assert_eq!(receipt.relation_count, 4);
    assert_eq!(receipt.unknown_count, 0);
    assert_eq!(receipt.semantic_hash.len(), 64);
    // The consumer stores what it was given, byte for byte.
    assert_eq!(consumer.received().len(), 1);
    assert_eq!(
        consumer.received()[0].canonical_bytes(),
        ingested.fragment.canonical_bytes()
    );
}

#[test]
fn consumer_rejects_schema_mismatch() {
    let rt = rt();
    let ingested = TextAdapter
        .ingest(&rt, "Alexander gave Finn 3 apples.")
        .unwrap();
    let mut fragment = ingested.fragment.clone();
    fragment.schema = "mncs-ingest/fragment-v99".to_owned();
    let mut consumer = EchoConsumer::new();
    let error = consumer
        .accept(&fragment)
        .expect_err("schema mismatch must fail");
    assert!(matches!(
        error,
        mncs_ingest::IngestError::SchemaMismatch { .. }
    ));
    assert!(consumer.received().is_empty());
}

#[test]
fn receipts_distinguish_semantics_not_provenance() {
    let rt = rt();
    let a = TextAdapter
        .ingest(&rt, "Alexander gave Finn 3 apples.")
        .unwrap();
    let b = TextAdapter
        .ingest(&rt, "Finn received three apples from Alexander.")
        .unwrap();
    // Same meaning: same semantic hash. Different signals: different ids.
    assert_eq!(
        receipt_for(&a.fragment).semantic_hash,
        receipt_for(&b.fragment).semantic_hash
    );
    assert_ne!(
        receipt_for(&a.fragment).signal_id,
        receipt_for(&b.fragment).signal_id
    );
    let _ = FRAGMENT_SCHEMA_VERSION;
}
