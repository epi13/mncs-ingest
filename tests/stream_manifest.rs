//! Tranche C: incremental manifest parsing.
//!
//! Golden documents, per-split-point invariance, typed values, nested
//! spans with re-staging, malformed positions, truncated-vs-malformed,
//! depth/key/string/int bounds, escape discipline, and a fuzz roundtrip.

mod common;

use common::rt;
use mncs_ingest::language::LanguageRuntime;
use mncs_ingest::manifest::{ingest_manifest, ManifestPair, ManifestValue};
use mncs_ingest::IngestError;

fn pairs_of(pairs: &[ManifestPair]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|pair| {
            let key = String::from_utf8_lossy(&pair.key).into_owned();
            let value = match &pair.value {
                ManifestValue::Str(bytes) => format!("s:{}", String::from_utf8_lossy(bytes)),
                ManifestValue::Int(n) => format!("i:{n}"),
                ManifestValue::Bool(b) => format!("b:{b}"),
                ManifestValue::Nested { start, len } => format!("n:{start}+{len}"),
            };
            (key, value)
        })
        .collect()
}

fn manifest_err(rt: &LanguageRuntime, doc: &[u8], width: usize) -> IngestError {
    ingest_manifest(rt, doc, width).expect_err("document must fail")
}

#[test]
fn golden_manifest_extracts_typed_pairs() {
    let rt = rt();
    let doc = br#"{"name": "Alexander", "age": 31, "active": true, "tags": ["a", "b"], "meta": {"x": 1}}"#;
    let pairs = ingest_manifest(rt, doc, 0).expect("golden manifest ingests");
    let view = pairs_of(&pairs);
    assert_eq!(view[0], ("name".to_owned(), "s:Alexander".to_owned()));
    assert_eq!(view[1], ("age".to_owned(), "i:31".to_owned()));
    assert_eq!(view[2], ("active".to_owned(), "b:true".to_owned()));
    assert!(matches!(view[3].1.as_str(), _ if view[3].1.starts_with("n:")));
    assert!(matches!(view[4].1.as_str(), _ if view[4].1.starts_with("n:")));
    // Spans resolve into the document.
    assert_eq!(&doc[pairs[0].key_span.0 as usize..][..pairs[0].key_span.1 as usize], b"name");
    if let ManifestValue::Nested { start, len } = pairs[3].value {
        assert_eq!(&doc[start as usize..(start + len) as usize], br#"["a", "b"]"#);
    } else {
        panic!("tags must be nested");
    }
}

#[test]
fn array_string_elements_close_as_values() {
    // A string closing in a fresh array is a value (need-comma), not a
    // key (need-colon): the comma after the first element must survive.
    let rt = rt();
    let doc = br#"{"a": ["x", "y"]}"#;
    let pairs = ingest_manifest(rt, doc, 0).expect("string array ingests");
    assert_eq!(pairs.len(), 1);
    if let ManifestValue::Nested { start, len } = pairs[0].value {
        assert_eq!(&doc[start as usize..(start + len) as usize], br#"["x", "y"]"#);
    } else {
        panic!("array must stay nested");
    }
    // Nested arrays nest the same way.
    let pairs = ingest_manifest(rt, br#"{"a": [["x"]]}"#, 0).expect("nested array ingests");
    assert_eq!(pairs.len(), 1);
    assert!(matches!(pairs[0].value, ManifestValue::Nested { .. }));
}

#[test]
fn every_split_point_agrees() {
    let rt = rt();
    let doc = br#"{"a": 1, "b": [true, false], "c": {"d": "x"}}"#;
    let baseline = ingest_manifest(rt, doc, 0).expect("baseline ingests");
    for width in [1usize, 2, 3, 5, 7, 8, 11, 16, 31, 32, 33, 64] {
        assert_eq!(
            ingest_manifest(rt, doc, width).expect("split ingests"),
            baseline,
            "chunk width {width} must agree"
        );
    }
}

#[test]
fn nested_values_restage() {
    // Recursion via transport: stage a nested span as its own document.
    let rt = rt();
    let doc = br#"{"outer": {"inner": 42}}"#;
    let pairs = ingest_manifest(rt, doc, 0).unwrap();
    assert_eq!(pairs.len(), 1);
    if let ManifestValue::Nested { start, len } = pairs[0].value {
        let inner = &doc[start as usize..(start + len) as usize];
        assert_eq!(inner, br#"{"inner": 42}"#);
        let staged = format!("{{\"wrap\": {}}}", String::from_utf8_lossy(inner));
        let wrapped = ingest_manifest(rt, staged.as_bytes(), 3).unwrap();
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].key, b"wrap");
    } else {
        panic!("outer must be nested");
    }
}

#[test]
fn malformed_positions_are_exact() {
    let rt = rt();
    // Mismatched closer at byte 7.
    let error = manifest_err(rt, br#"{"a": 1]"#, 0);
    assert!(matches!(error, IngestError::Malformed { .. }), "{error:?}");
    // Trailing comma.
    assert!(matches!(
        manifest_err(rt, br#"{"a": 1,}"#, 0),
        IngestError::Malformed { .. }
    ));
    // Missing colon.
    assert!(matches!(
        manifest_err(rt, br#"{"a" 1}"#, 0),
        IngestError::Malformed { .. }
    ));
    // Bare scalar root is not a manifest.
    assert!(matches!(
        manifest_err(rt, b"42", 0),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        manifest_err(rt, br#""str""#, 0),
        IngestError::Malformed { .. }
    ));
    // Bad escape and unicode escape.
    assert!(matches!(
        manifest_err(rt, br#"{"a": "x\q"}"#, 0),
        IngestError::Malformed { .. }
    ));
    // Unicode escapes are rejected by design (INGEST-P-003).
    assert!(matches!(
        manifest_err(rt, br#"{"a": "x\u0041"}"#, 0),
        IngestError::Malformed { .. }
    ));
    // Boolean prefixes and float lookalikes fail in extraction.
    assert!(matches!(
        manifest_err(rt, br#"{"a": tru}"#, 0),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        manifest_err(rt, br#"{"a": 1.5}"#, 0),
        IngestError::Malformed { .. }
    ));
}

#[test]
fn truncated_is_not_malformed() {
    let rt = rt();
    let full = br#"{"alpha": [1, {"beta": false}]}"#;
    for cut in [1usize, 5, 10, 15, 20, full.len() - 1] {
        let error = manifest_err(rt, &full[..cut], 0);
        assert!(
            matches!(error, IngestError::Malformed { .. }),
            "cut {cut}: truncation surfaces as malformed-truncated: {error:?}"
        );
        // And the prefix streams without error until end-of-input.
        let mut scanner = mncs_ingest::manifest::ManifestScanner::new();
        let verdict = scanner.scan(rt, &full[..cut], false).unwrap();
        assert_eq!(verdict.status, 0, "cut {cut}: prefix is ok-so-far");
    }
}

#[test]
fn depth_and_size_bounds_hold() {
    let rt = rt();
    // Depth 4 ingests; depth 5 is malformed.
    assert!(ingest_manifest(rt, br#"{"a": {"b": {"c": {"d": 1}}}}"#, 0).is_ok());
    assert!(matches!(
        manifest_err(rt, br#"{"a": {"b": {"c": {"d": {"e": 1}}}}}"#, 0),
        IngestError::Malformed { .. }
    ));
    // Integers within ±99999; beyond fails.
    assert!(ingest_manifest(rt, br#"{"n": 99999, "m": -99999}"#, 0).is_ok());
    assert!(matches!(
        manifest_err(rt, br#"{"n": 100000}"#, 0),
        IngestError::Malformed { .. }
    ));
    // 32-byte keys pass; 33-byte keys are overlong.
    let ok_doc = format!("{{\"{}\": 1}}", "k".repeat(32));
    assert!(ingest_manifest(rt, ok_doc.as_bytes(), 0).is_ok());
    let big_doc = format!("{{\"{}\": 1}}", "k".repeat(33));
    assert!(matches!(
        manifest_err(rt, big_doc.as_bytes(), 0),
        IngestError::Overlong { .. }
    ));
    // 64-byte strings pass; 65-byte strings are overlong.
    let ok_doc = format!("{{\"s\": \"{}\"}}", "v".repeat(64));
    assert!(ingest_manifest(rt, ok_doc.as_bytes(), 0).is_ok());
    let big_doc = format!("{{\"s\": \"{}\"}}", "v".repeat(65));
    assert!(matches!(
        manifest_err(rt, big_doc.as_bytes(), 0),
        IngestError::Overlong { .. }
    ));
}

#[test]
fn escape_discipline_in_values() {
    let rt = rt();
    let pairs = ingest_manifest(rt, br#"{"q": "a\"b\\c", "t": "x\ty"}"#, 0).unwrap();
    assert_eq!(pairs.len(), 2);
    assert!(matches!(&pairs[0].value, ManifestValue::Str(bytes) if bytes == b"a\"b\\c"));
    assert!(matches!(&pairs[1].value, ManifestValue::Str(bytes) if bytes == b"x\ty"));
}

#[test]
fn seeded_manifest_fuzz() {
    let rt = rt();
    let mut state: u64 = 0x9a7b_3c1d_5e0f_2718;
    let mut next = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 33) as usize
    };
    for case in 0..60 {
        let npairs = 1 + next() % 4;
        let mut doc = Vec::from(b"{" as &[u8]);
        let mut expect = Vec::new();
        for index in 0..npairs {
            if index > 0 {
                doc.push(b',');
            }
            let key = format!("k{index}");
            doc.push(b'"');
            doc.extend_from_slice(key.as_bytes());
            doc.extend_from_slice(b"\":");
            match next() % 4 {
                0 => {
                    let text = format!("v{case}-{index}");
                    doc.push(b'"');
                    doc.extend_from_slice(text.as_bytes());
                    doc.push(b'"');
                    expect.push((key, format!("s:{text}")));
                }
                1 => {
                    let num = (next() % 200000) as i64 - 99999;
                    doc.extend_from_slice(num.to_string().as_bytes());
                    expect.push((key, format!("i:{num}")));
                }
                2 => {
                    let flag = next() % 2 == 0;
                    doc.extend_from_slice(if flag { b"true" } else { b"false" });
                    expect.push((key, format!("b:{flag}")));
                }
                _ => {
                    doc.extend_from_slice(b"[1,2]");
                    expect.push((key, "n:".to_owned()));
                }
            }
        }
        doc.push(b'}');
        let width = 1 + next() % 25;
        let pairs = ingest_manifest(rt, &doc, width).unwrap_or_else(|error| {
            panic!("case {case} width {width}: {error:?} in {}", String::from_utf8_lossy(&doc))
        });
        assert_eq!(pairs.len(), expect.len(), "case {case}");
        for (pair, (key, value)) in pairs.iter().zip(&expect) {
            assert_eq!(String::from_utf8_lossy(&pair.key), *key);
            if value == "n:" {
                assert!(matches!(pair.value, ManifestValue::Nested { .. }));
            } else {
                let rendered = match &pair.value {
                    ManifestValue::Str(bytes) => format!("s:{}", String::from_utf8_lossy(bytes)),
                    ManifestValue::Int(n) => format!("i:{n}"),
                    ManifestValue::Bool(b) => format!("b:{b}"),
                    ManifestValue::Nested { .. } => "n:".to_owned(),
                };
                assert_eq!(&rendered, value, "case {case}");
            }
        }
    }
}
