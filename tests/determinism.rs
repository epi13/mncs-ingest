//! Determinism: repeated ingestion is byte-identical, across reloads.

mod common;

use common::rt;
use mncs_ingest::adapters::{Adapter, StructuredAdapter, TextAdapter};

#[test]
fn repeated_ingestion_is_byte_identical() {
    let rt = rt();
    let first = TextAdapter
        .ingest(&rt, "Alexander gave Finn 3 apples.")
        .unwrap();
    for _ in 0..3 {
        let again = TextAdapter
            .ingest(&rt, "Alexander gave Finn 3 apples.")
            .unwrap();
        assert_eq!(
            first.fragment.canonical_bytes(),
            again.fragment.canonical_bytes()
        );
        assert_eq!(
            first.fragment.provenance.signal_id,
            again.fragment.provenance.signal_id
        );
    }
}

#[test]
fn reload_preserves_canonical_output() {
    let before = TextAdapter
        .ingest(&rt(), "Finn received three apples from Alexander.")
        .unwrap();
    let after = TextAdapter
        .ingest(&rt(), "Finn received three apples from Alexander.")
        .unwrap();
    assert_eq!(
        before.fragment.canonical_bytes(),
        after.fragment.canonical_bytes()
    );
    let structured = StructuredAdapter
        .ingest(
            &rt(),
            br#"{"event":"transfer","from":"Finn","to":"Alexander","object":"apple","quantity":3}"#,
        )
        .unwrap();
    assert_ne!(
        before.fragment.canonical_bytes(),
        structured.fragment.canonical_bytes(),
        "different observations still differ after reload"
    );
}
