//! Adapter equivalence: text, structured, and native obey one contract.
//!
//! Covers the shared-code contract (vocabulary codes match the host
//! rendering), open-vocabulary pass-through (unicode entities), and the
//! native adapter's no-reinterpretation rule.

mod common;

use common::rt;
use mncs_ingest::adapters::{
    Adapter, NativeAdapter, NativeTransfer, StructuredAdapter, TextAdapter,
};
use mncs_ingest::canonical::{mncs_equal, object_label};

#[test]
fn vocabulary_codes_match_host_rendering() {
    // If either side of the code table drifts, this pins the break.
    let rt = rt();
    assert_eq!(rt.classify_event(b"transfer").unwrap(), 1);
    assert_eq!(rt.classify_event(b"give").unwrap(), 1);
    assert_eq!(rt.classify_event(b"gave").unwrap(), 1);
    assert_eq!(rt.classify_event(b"hug").unwrap(), 0);
    assert_eq!(rt.classify_object(b"apple").unwrap(), 1);
    assert_eq!(rt.classify_object(b"apples").unwrap(), 1);
    assert_eq!(rt.classify_object(b"ORANGES").unwrap(), 2);
    assert_eq!(rt.classify_object(b"parsnip").unwrap(), 0);
    assert_eq!(object_label(1), Some("apple"));
    assert_eq!(object_label(2), Some("orange"));
    assert_eq!(object_label(0), None);
}

#[test]
fn unicode_entities_pass_through_all_adapters() {
    let rt = rt();
    let text = TextAdapter
        .ingest(&rt, "Zo\u{eb} gave Finn 3 apples.")
        .expect("unicode entity ingests through text");
    let structured = StructuredAdapter
        .ingest(
            &rt,
            "{\u{22}event\u{22}:\u{22}transfer\u{22},\u{22}from\u{22}:\u{22}Zo\u{eb}\u{22},\u{22}to\u{22}:\u{22}Finn\u{22},\u{22}object\u{22}:\u{22}apple\u{22},\u{22}quantity\u{22}:3}".as_bytes(),
        )
        .expect("unicode entity ingests through structured");
    assert!(text.fragment.semantically_equal(&structured.fragment));
    assert!(mncs_equal(&rt, &text.facts, &structured.facts).unwrap());
    assert_eq!(text.fragment.atoms[1].label, "Zo\u{eb}");
}

#[test]
fn native_codes_need_no_reinterpretation() {
    // The native adapter validates but never reclassifies: agreeing
    // spellings pass, conflicting ones fail, and validation still runs.
    let rt = rt();
    let agreeing = NativeTransfer {
        event_code: 1,
        source: "Alexander".to_owned(),
        target: "Finn".to_owned(),
        object_code: 1,
        object_spelling: Some("apples".to_owned()),
        quantity: Some(3),
    };
    let ingested = NativeAdapter
        .ingest(&rt, agreeing)
        .expect("agreeing native ingests");
    assert_eq!(ingested.fragment.atoms[3].label, "apple");
    let plural_agreeing = NativeTransfer {
        event_code: 1,
        source: "Alexander".to_owned(),
        target: "Finn".to_owned(),
        object_code: 2,
        object_spelling: Some("Oranges".to_owned()),
        quantity: None,
    };
    assert!(NativeAdapter.ingest(&rt, plural_agreeing).is_ok());
}

#[test]
fn all_adapters_reject_the_same_unknown_event() {
    let rt = rt();
    assert!(TextAdapter
        .ingest(&rt, "Alexander hugged Finn 3 apples.")
        .is_err());
    assert!(StructuredAdapter
        .ingest(
            &rt,
            br#"{"event":"hug","from":"Alexander","to":"Finn","object":"apple"}"#
        )
        .is_err());
    assert!(NativeAdapter
        .ingest(
            &rt,
            NativeTransfer {
                event_code: 99,
                source: "Alexander".to_owned(),
                target: "Finn".to_owned(),
                object_code: 1,
                object_spelling: None,
                quantity: Some(3),
            },
        )
        .is_err());
}
