//! Unknown and partial input: never manufacture certainty.
//!
//! Unknown objects stay explicit (`Unknown` atoms plus `unknowns` tags),
//! missing quantities stay absent (no zero is invented), and partially
//! valid structured input still ingests with `Uncertain` confidence.

mod common;

use common::rt;
use mncs_ingest::adapters::{Adapter, StructuredAdapter, TextAdapter};
use mncs_ingest::ir::{AtomKind, Confidence};

#[test]
fn unknown_object_word_stays_explicit() {
    let rt = rt();
    let ingested = TextAdapter
        .ingest(&rt, "Alexander gave Finn 3 parsnips.")
        .expect("unknown object still ingests");
    let object = &ingested.fragment.atoms[3];
    assert_eq!(object.kind, AtomKind::Unknown);
    assert_eq!(object.label, "parsnips");
    assert_eq!(
        ingested.fragment.unknowns,
        ["object:unknown:parsnips".to_owned()]
    );
    assert_eq!(
        ingested.fragment.provenance.confidence,
        Confidence::Uncertain
    );
    assert_eq!(ingested.facts.object_code, 0);
}

#[test]
fn missing_quantity_invents_no_zero() {
    let rt = rt();
    let ingested = TextAdapter
        .ingest(&rt, "Alexander gave Finn apples.")
        .expect("missing quantity still ingests");
    assert!(ingested.facts.quantity.is_none());
    assert_eq!(ingested.fragment.atoms.len(), 4);
    assert!(ingested
        .fragment
        .atoms
        .iter()
        .all(|atom| atom.kind != AtomKind::Quantity));
    assert_eq!(ingested.fragment.provenance.confidence, Confidence::Parsed);
}

#[test]
fn structured_partial_input_ingests_without_quantity() {
    let rt = rt();
    let ingested = StructuredAdapter
        .ingest(
            &rt,
            br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"apple"}"#,
        )
        .expect("partial structured input ingests");
    assert!(ingested.facts.quantity.is_none());
    assert_eq!(ingested.fragment.atoms.len(), 4);
}

#[test]
fn structured_unknown_object_is_uncertain() {
    let rt = rt();
    let ingested = StructuredAdapter
        .ingest(
            &rt,
            br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"parsnip","quantity":3}"#,
        )
        .expect("unknown structured object ingests");
    assert_eq!(
        ingested.fragment.provenance.confidence,
        Confidence::Uncertain
    );
    assert_eq!(
        ingested.fragment.unknowns,
        ["object:unknown:parsnip".to_owned()]
    );
}

#[test]
fn duplicate_relations_are_impossible_by_construction() {
    // The builder emits exactly one relation per role; a second object
    // word fails closed instead of doubling the relation.
    let rt = rt();
    let error = TextAdapter
        .ingest(&rt, "Alexander gave Finn 3 apples oranges.")
        .expect_err("doubled object must fail");
    assert!(matches!(error, mncs_ingest::IngestError::Malformed { .. }));
}
