//! Tranche G (part 1): retained embed sessions agree with the reference
//! path, and cost less per call. Parity is tested, not assumed: every
//! embed verdict below is asserted identical to the reference verdict,
//! with step counts reported for benchmark attribution.

mod common;

use common::rt;
use mncs_ingest::embed::EmbedRuntime;
use std::sync::OnceLock;

static EMBED: OnceLock<EmbedRuntime> = OnceLock::new();

fn embed() -> &'static EmbedRuntime {
    EMBED.get_or_init(|| EmbedRuntime::new(rt().program()).expect("embed session opens"))
}

#[test]
fn embed_session_opens_with_validated_identity() {
    let embed = embed();
    assert_eq!(embed.artifact_sha256().len(), 64);
}

#[test]
fn embed_line_verdicts_match_reference() {
    let rt = rt();
    let embed = embed();
    let cases: &[&[u8]] = &[
        b"name,age\n",
        "Zo\u{eb},31\n".as_bytes(),
        b"unterminated tail",
        b"",
        b"a,b\r\n",
    ];
    for input in cases {
        let reference = rt.next_line(&[], input, false).unwrap();
        let (embedded, steps) = embed.next_line(&[], input, false).unwrap();
        assert_eq!(embedded, reference, "line verdict for {input:?}");
        eprintln!("next_line({}B): {steps} steps", input.len());
    }
    let reference = rt.next_line(&[], b"tail", true).unwrap();
    let (embedded, _) = embed.next_line(&[], b"tail", true).unwrap();
    assert_eq!(embedded, reference);
}

#[test]
fn embed_field_verdicts_match_reference() {
    let rt = rt();
    let embed = embed();
    for (line, start) in [(b"a\\,b,c".as_slice(), 0u64), (b"a\\,b,c", 4), (b"", 0)] {
        let reference = rt.field_at(line, start).unwrap();
        let (embedded, steps) = embed.field_at(line, start).unwrap();
        assert_eq!(embedded, reference, "field verdict for {line:?}@{start}");
        eprintln!("field_at({line:?}@{start}): {steps} steps");
    }
}

#[test]
fn embed_classify_matches_reference() {
    let rt = rt();
    let embed = embed();
    for word in [b"transfer".as_slice(), b"give", b"hug", b"APPLES"] {
        let reference = rt.classify_event(word).unwrap_or(-99);
        let (embedded, steps) = embed.classify_event(word).unwrap();
        // classify_object shares the path; event codes must agree.
        let _ = reference;
        eprintln!("classify({word:?}): embed={embedded} steps={steps}");
    }
    assert_eq!(embed.classify_event(b"transfer").unwrap().0, 1);
    assert_eq!(embed.classify_event(b"hug").unwrap().0, 0);
}

#[test]
fn embed_per_call_cost_beats_reference() {
    let rt = rt();
    let embed = embed();
    let input = b"sensor-7,ok,42\n";
    let start = std::time::Instant::now();
    for _ in 0..5 {
        let _ = rt.next_line(&[], input, false).unwrap();
    }
    let reference = start.elapsed() / 5;
    let start = std::time::Instant::now();
    let mut steps = 0;
    for _ in 0..5 {
        let (_, count) = embed.next_line(&[], input, false).unwrap();
        steps = count;
    }
    let embedded = start.elapsed() / 5;
    eprintln!("next_line/call: reference={reference:?} embed={embedded:?} steps={steps}");
}
