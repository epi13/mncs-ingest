//! Malformed input: controlled errors, never silent corruption.
//!
//! Each case pins one failure mode to one error variant. Anything the
//! bounded grammar cannot mean must come back typed — not guessed, not
//! truncated, not panicked.

mod common;

use common::rt;
use mncs_ingest::adapters::{
    Adapter, NativeAdapter, NativeTransfer, StructuredAdapter, TextAdapter,
};
use mncs_ingest::IngestError;

fn text_error(input: &str) -> IngestError {
    TextAdapter
        .ingest(&rt(), input)
        .expect_err("input must fail")
}

fn json_error(input: &[u8]) -> IngestError {
    StructuredAdapter
        .ingest(&rt(), input)
        .expect_err("input must fail")
}

#[test]
fn empty_and_blank_inputs_are_malformed() {
    assert!(matches!(text_error(""), IngestError::Malformed { .. }));
    assert!(matches!(text_error("   "), IngestError::Malformed { .. }));
    assert!(matches!(text_error("."), IngestError::Malformed { .. }));
}

#[test]
fn unsupported_verbs_are_unsupported() {
    assert!(matches!(
        text_error("Alexander hugged Finn 3 apples."),
        IngestError::Unsupported { .. }
    ));
    assert!(matches!(
        text_error("Alexander takes Finn 3 apples."),
        IngestError::Unsupported { .. }
    ));
}

#[test]
fn missing_participants_are_missing_fields() {
    assert!(matches!(
        text_error("Alexander gave Finn."),
        IngestError::MissingField { .. }
    ));
    assert!(matches!(
        text_error("Alexander gave 3 apples."),
        IngestError::MissingField { .. }
    ));
    assert!(matches!(
        text_error("Finn received three apples."),
        IngestError::MissingField { .. }
    ));
}

#[test]
fn malformed_numbers_fail_closed() {
    assert!(matches!(
        text_error("Alexander gave Finn 3x apples."),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        text_error("Alexander gave Finn 1234 apples."),
        IngestError::Malformed { .. }
    ));
}

#[test]
fn bad_shapes_are_malformed() {
    assert!(matches!(
        text_error("Alexander gave Finn Bob 3 apples."),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        text_error("Alexander gave Finn 3 apples yesterday."),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        text_error("Alexander gave Finn 3 4 apples."),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        text_error("Alexander gave Finn gave Finn 3 apples."),
        IngestError::Malformed { .. }
    ));
}

#[test]
fn overlong_input_is_rejected_before_mncs() {
    let input = "Alexander gave Finn 3 apples and then Alexander gave Finn 3 apples.";
    assert!(input.len() > 64);
    assert!(matches!(text_error(input), IngestError::Overlong { .. }));
}

#[test]
fn structured_json_failures_are_typed() {
    assert!(matches!(
        json_error(b"{not json"),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        json_error(b"[1,2]"),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        json_error(br#"{"event":"transfer","to":"Finn","object":"apple"}"#),
        IngestError::MissingField { .. }
    ));
    assert!(matches!(
        json_error(br#"{"event":"transfer","from":"","to":"Finn","object":"apple"}"#),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        json_error(br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"apple","quantity":"3"}"#),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        json_error(br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"apple","quantity":-1}"#),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        json_error(br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"apple","quantity":1.5}"#),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        json_error(br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"apple","quantity":1000}"#),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        json_error(br#"{"event":"hug","from":"Alexander","to":"Finn","object":"apple"}"#),
        IngestError::Unsupported { .. }
    ));
    let unexpected = json_error(
        br#"{"event":"transfer","from":"Alexander","to":"Finn","object":"apple","when":"yesterday"}"#,
    );
    assert!(
        matches!(unexpected, IngestError::UnexpectedField { .. }),
        "unexpected fields are rejected, not dropped: {unexpected:?}"
    );
}

#[test]
fn native_input_failures_are_typed() {
    let rt = rt();
    let bad_event = NativeTransfer {
        event_code: 7,
        source: "Alexander".to_owned(),
        target: "Finn".to_owned(),
        object_code: 1,
        object_spelling: None,
        quantity: Some(3),
    };
    assert!(matches!(
        NativeAdapter.ingest(&rt, bad_event).unwrap_err(),
        IngestError::Unsupported { .. }
    ));
    let bad_code = NativeTransfer {
        event_code: 1,
        source: "Alexander".to_owned(),
        target: "Finn".to_owned(),
        object_code: 9,
        object_spelling: None,
        quantity: Some(3),
    };
    assert!(matches!(
        NativeAdapter.ingest(&rt, bad_code).unwrap_err(),
        IngestError::Unsupported { .. }
    ));
    let conflict = NativeTransfer {
        event_code: 1,
        source: "Alexander".to_owned(),
        target: "Finn".to_owned(),
        object_code: 1,
        object_spelling: Some("orange".to_owned()),
        quantity: Some(3),
    };
    assert!(matches!(
        NativeAdapter.ingest(&rt, conflict).unwrap_err(),
        IngestError::ConflictingFields { .. }
    ));
    let missing_spelling = NativeTransfer {
        event_code: 1,
        source: "Alexander".to_owned(),
        target: "Finn".to_owned(),
        object_code: 0,
        object_spelling: None,
        quantity: Some(3),
    };
    assert!(matches!(
        NativeAdapter.ingest(&rt, missing_spelling).unwrap_err(),
        IngestError::MissingField { .. }
    ));
    let bad_quantity = NativeTransfer {
        event_code: 1,
        source: "Alexander".to_owned(),
        target: "Finn".to_owned(),
        object_code: 1,
        object_spelling: None,
        quantity: Some(-2),
    };
    assert!(matches!(
        NativeAdapter.ingest(&rt, bad_quantity).unwrap_err(),
        IngestError::Malformed { .. }
    ));
}
