//! Semantic separation: meaningfully different observations must not
//! collapse. Every case asserts host-key inequality AND executed MNCS
//! inequality together, so the two decision paths can never silently
//! disagree.

mod common;

use common::rt;
use mncs_ingest::adapters::{Adapter, TextAdapter};
use mncs_ingest::canonical::mncs_equal;
use mncs_ingest::language::LanguageRuntime;

fn ingest(rt: &LanguageRuntime, text: &str) -> mncs_ingest::canonical::Ingested {
    TextAdapter
        .ingest(rt, text)
        .expect("separation input ingests")
}

fn assert_separated(rt: &LanguageRuntime, left: &str, right: &str) {
    let a = ingest(rt, left);
    let b = ingest(rt, right);
    assert!(
        !a.fragment.semantically_equal(&b.fragment),
        "host keys must differ: {left:?} vs {right:?}"
    );
    assert!(
        !mncs_equal(rt, &a.facts, &b.facts).expect("MNCS equality executes"),
        "MNCS equality must differ: {left:?} vs {right:?}"
    );
}

#[test]
fn quantity_change_separates() {
    let rt = rt();
    assert_separated(
        &rt,
        "Alexander gave Finn 3 apples.",
        "Alexander gave Finn 4 apples.",
    );
}

#[test]
fn role_reversal_separates() {
    let rt = rt();
    assert_separated(
        &rt,
        "Alexander gave Finn 3 apples.",
        "Finn gave Alexander 3 apples.",
    );
}

#[test]
fn object_change_separates() {
    let rt = rt();
    assert_separated(
        &rt,
        "Alexander gave Finn 3 apples.",
        "Alexander gave Finn 3 oranges.",
    );
}

#[test]
fn singular_plural_converge_but_count_differs() {
    let rt = rt();
    let one = ingest(&rt, "Alexander gave Finn 1 apple.");
    let many = ingest(&rt, "Alexander gave Finn 3 apples.");
    assert!(!one.fragment.semantically_equal(&many.fragment));
    assert!(!mncs_equal(&rt, &one.facts, &many.facts).unwrap());
}

#[test]
fn zero_quantity_separates_from_missing_quantity() {
    let rt = rt();
    let zero = ingest(&rt, "Alexander gave Finn 0 apples.");
    let missing = ingest(&rt, "Alexander gave Finn apples.");
    assert!(zero.facts.quantity == Some(0));
    assert!(missing.facts.quantity.is_none());
    assert!(!zero.fragment.semantically_equal(&missing.fragment));
    assert!(!mncs_equal(&rt, &zero.facts, &missing.facts).unwrap());
}

#[test]
fn unknown_object_separates_from_known_object() {
    let rt = rt();
    assert_separated(
        &rt,
        "Alexander gave Finn 3 apples.",
        "Alexander gave Finn 3 parsnips.",
    );
}

#[test]
fn distinct_unknown_objects_separate() {
    let rt = rt();
    assert_separated(
        &rt,
        "Alexander gave Finn 3 parsnips.",
        "Alexander gave Finn 3 turnips.",
    );
}

#[test]
fn entity_case_is_load_bearing() {
    // A lowercase "finn" is not an entity under the bounded grammar: it
    // reads as an unknown-object candidate, so no target entity remains
    // and the sentence fails closed instead of guessing.
    let rt = rt();
    let error = TextAdapter
        .ingest(&rt, "Alexander gave finn 3 apples.")
        .expect_err("lowercase entity must not ingest");
    assert!(matches!(
        error,
        mncs_ingest::IngestError::MissingField { .. }
    ));
}
