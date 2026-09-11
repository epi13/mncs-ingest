//! Semantic convergence: distinct surface representations of one bounded
//! observation produce equivalent canonical fragments.
//!
//! The four-way family: active text, passive text, structured JSON, native
//! MNCS codes. Equivalence is asserted twice — the host semantic key and
//! executed MNCS equality must agree — while provenance must differ.

mod common;

use common::rt;
use mncs_ingest::adapters::{
    Adapter, NativeAdapter, NativeTransfer, StructuredAdapter, TextAdapter,
};
use mncs_ingest::canonical::mncs_equal;
use mncs_ingest::ir::{AdapterKind, Confidence};

fn active() -> &'static str {
    "Alexander gave Finn 3 apples."
}

fn passive() -> &'static str {
    "Finn received three apples from Alexander."
}

fn event_json() -> &'static [u8] {
    br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"apple","quantity":3}"#
}

fn native() -> NativeTransfer {
    NativeTransfer {
        event_code: 1,
        source: "Alexander".to_owned(),
        target: "Finn".to_owned(),
        object_code: 1,
        object_spelling: None,
        quantity: Some(3),
    }
}

#[test]
fn four_representations_converge_to_one_canonical_form() {
    let rt = rt();
    let text = TextAdapter
        .ingest(&rt, active())
        .expect("active text ingests");
    let text_passive = TextAdapter
        .ingest(&rt, passive())
        .expect("passive text ingests");
    let structured = StructuredAdapter
        .ingest(&rt, event_json())
        .expect("structured event ingests");
    let native = NativeAdapter
        .ingest(&rt, native())
        .expect("native transfer ingests");

    let fragments = [
        &text.fragment,
        &text_passive.fragment,
        &structured.fragment,
        &native.fragment,
    ];
    for pair in [
        (&text, &text_passive),
        (&text, &structured),
        (&text, &native),
        (&text_passive, &structured),
        (&text_passive, &native),
        (&structured, &native),
    ] {
        assert!(
            pair.0.fragment.semantically_equal(&pair.1.fragment),
            "host semantic keys must agree"
        );
        assert!(
            mncs_equal(&rt, &pair.0.facts, &pair.1.facts).expect("MNCS equality executes"),
            "MNCS equality must agree with the host key"
        );
        assert_eq!(
            pair.0.fragment.canonical_bytes(),
            pair.1.fragment.canonical_bytes(),
            "canonical bytes are identical across adapters"
        );
    }

    // Pin the exact canonical shape.
    let fragment = &text.fragment;
    assert_eq!(fragment.schema, "mncs-ingest/fragment-v1");
    let labels: Vec<&str> = fragment
        .atoms
        .iter()
        .map(|atom| atom.label.as_str())
        .collect();
    assert_eq!(labels, ["transfer", "Alexander", "Finn", "apple", "3"]);
    assert!(fragment.unknowns.is_empty());
    let roles: Vec<&str> = fragment
        .relations
        .iter()
        .map(|relation| relation.role.as_str())
        .collect();
    assert_eq!(roles, ["source", "target", "object", "quantity"]);
    assert_eq!(fragment.atoms.len(), 5);
    assert_eq!(fragments.len(), 4);
}

#[test]
fn convergence_keeps_distinct_provenance() {
    let rt = rt();
    let text = TextAdapter.ingest(&rt, active()).unwrap();
    let text_passive = TextAdapter.ingest(&rt, passive()).unwrap();
    let structured = StructuredAdapter.ingest(&rt, event_json()).unwrap();
    let native = NativeAdapter.ingest(&rt, native()).unwrap();

    let ids = [
        text.fragment.provenance.signal_id.clone(),
        text_passive.fragment.provenance.signal_id.clone(),
        structured.fragment.provenance.signal_id.clone(),
        native.fragment.provenance.signal_id.clone(),
    ];
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(ids[i], ids[j], "distinct signals keep distinct provenance");
        }
    }
    assert_eq!(text.fragment.provenance.adapter, AdapterKind::Text);
    assert_eq!(
        structured.fragment.provenance.adapter,
        AdapterKind::Structured
    );
    assert_eq!(native.fragment.provenance.adapter, AdapterKind::Native);
    assert_eq!(text.fragment.provenance.confidence, Confidence::Parsed);
    assert_eq!(
        structured.fragment.provenance.confidence,
        Confidence::Direct
    );
    assert_eq!(native.fragment.provenance.confidence, Confidence::Direct);
}

#[test]
fn second_family_converges_without_quantity() {
    let rt = rt();
    let text = TextAdapter
        .ingest(&rt, "Finn gave Alexander oranges.")
        .expect("active text without quantity ingests");
    let structured = StructuredAdapter
        .ingest(
            &rt,
            br#"{"event":"give","from":"Finn","to":"Alexander","object":"oranges"}"#,
        )
        .expect("structured event without quantity ingests");
    let native = NativeAdapter
        .ingest(
            &rt,
            NativeTransfer {
                event_code: 1,
                source: "Finn".to_owned(),
                target: "Alexander".to_owned(),
                object_code: 2,
                object_spelling: None,
                quantity: None,
            },
        )
        .expect("native transfer without quantity ingests");

    assert!(text.fragment.semantically_equal(&structured.fragment));
    assert!(text.fragment.semantically_equal(&native.fragment));
    assert!(mncs_equal(&rt, &text.facts, &structured.facts).unwrap());
    assert!(mncs_equal(&rt, &text.facts, &native.facts).unwrap());
    // No quantity atom was manufactured.
    assert_eq!(text.fragment.atoms.len(), 4);
    assert_eq!(text.fragment.relations.len(), 3);
    let labels: Vec<&str> = text
        .fragment
        .atoms
        .iter()
        .map(|atom| atom.label.as_str())
        .collect();
    assert_eq!(labels, ["transfer", "Finn", "Alexander", "orange"]);
}
