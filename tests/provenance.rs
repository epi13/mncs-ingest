//! Provenance preservation: every assertion stays auditable.
//!
//! Provenance is structural: adapter, signal hash, interpreter identity,
//! source spans, and confidence travel inside the fragment. Spans resolve
//! against the original input bytes, and the signal id replays to the
//! exact input through sha256.

mod common;

use common::rt;
use mncs_ingest::adapters::{
    Adapter, NativeAdapter, NativeTransfer, StructuredAdapter, TextAdapter,
};
use mncs_ingest::canonical::signal_id;

#[test]
fn text_spans_resolve_to_source_substrings() {
    let rt = rt();
    let input = "Alexander gave Finn 3 apples.";
    let ingested = TextAdapter.ingest(&rt, input).unwrap();
    let provenance = &ingested.fragment.provenance;

    let mut seen = std::collections::HashMap::new();
    for span in &provenance.spans {
        if let (Some(start), Some(end)) = (span.start, span.end) {
            seen.insert(span.field.clone(), &input[start as usize..end as usize]);
        }
    }
    assert_eq!(seen["word:source"], "Alexander");
    assert_eq!(seen["word:target"], "Finn");
    assert_eq!(seen["word:object"], "apples");
    assert_eq!(seen["word:verb"], "gave");
    assert_eq!(seen["word:quantity"], "3");
    assert_eq!(provenance.signal_id, signal_id(input.as_bytes()));
    assert_eq!(provenance.interpreter, "mncs.ingest/ingest_parse");
}

#[test]
fn passive_spans_resolve_against_reordered_roles() {
    let rt = rt();
    let input = "Finn received three apples from Alexander.";
    let ingested = TextAdapter.ingest(&rt, input).unwrap();
    let mut seen = std::collections::HashMap::new();
    for span in &ingested.fragment.provenance.spans {
        if let (Some(start), Some(end)) = (span.start, span.end) {
            seen.insert(span.field.clone(), &input[start as usize..end as usize]);
        }
    }
    // Surface order is Finn-first, but roles resolve semantically.
    assert_eq!(seen["word:source"], "Alexander");
    assert_eq!(seen["word:target"], "Finn");
    assert_eq!(seen["word:object"], "apples");
    assert_eq!(seen["word:quantity"], "three");
}

#[test]
fn structured_and_native_carry_field_provenance() {
    let rt = rt();
    let structured = StructuredAdapter
        .ingest(
            &rt,
            br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"apple","quantity":3}"#,
        )
        .unwrap();
    let fields: Vec<&str> = structured
        .fragment
        .provenance
        .spans
        .iter()
        .map(|span| span.field.as_str())
        .collect();
    assert!(fields.contains(&"json:from"));
    assert!(fields.contains(&"json:to"));
    assert!(fields.contains(&"json:object"));
    assert!(fields.contains(&"json:quantity"));

    let native = NativeAdapter
        .ingest(
            &rt,
            NativeTransfer {
                event_code: 1,
                source: "Alexander".to_owned(),
                target: "Finn".to_owned(),
                object_code: 1,
                object_spelling: None,
                quantity: Some(3),
            },
        )
        .unwrap();
    let fields: Vec<&str> = native
        .fragment
        .provenance
        .spans
        .iter()
        .map(|span| span.field.as_str())
        .collect();
    assert!(fields.contains(&"native:source"));
    assert!(fields.contains(&"native:quantity"));
    assert_eq!(
        native.fragment.provenance.interpreter,
        "mncs.ingest/ingest_validate (native)"
    );
}

#[test]
fn signal_id_replays_to_exact_input() {
    use sha2::{Digest, Sha256};
    let rt = rt();
    let input = "Alexander gave Finn 3 apples.";
    let ingested = TextAdapter.ingest(&rt, input).unwrap();
    let recomputed: String = Sha256::digest(input.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(ingested.fragment.provenance.signal_id, recomputed);
    assert_eq!(ingested.fragment.provenance.signal_id.len(), 64);
}
