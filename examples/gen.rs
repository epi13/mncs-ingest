//! Fixture generator: ingests the four `examples/transfer` inputs and
//! writes the canonical outputs they converge to. Run with
//! `cargo run --offline --example gen`; outputs are checked in so the
//! convergence claim is inspectable without running code.

use mncs_ingest::adapters::{
    Adapter, NativeAdapter, NativeTransfer, StructuredAdapter, TextAdapter,
};
use mncs_ingest::language::LanguageRuntime;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("examples/transfer");
    let rt = LanguageRuntime::from_crate_dir().expect("MNCS program loads");

    let active = std::fs::read_to_string(dir.join("text_active.txt")).unwrap();
    let passive = std::fs::read_to_string(dir.join("text_passive.txt")).unwrap();
    let event = std::fs::read(dir.join("event.json")).unwrap();

    let a = TextAdapter
        .ingest(&rt, active.trim())
        .expect("active ingests");
    let b = TextAdapter
        .ingest(&rt, passive.trim())
        .expect("passive ingests");
    let c = StructuredAdapter
        .ingest(&rt, &event)
        .expect("event ingests");
    let d = NativeAdapter
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
        .expect("native ingests");

    assert!(a.fragment.semantically_equal(&b.fragment));
    assert!(a.fragment.semantically_equal(&c.fragment));
    assert!(a.fragment.semantically_equal(&d.fragment));

    let canonical = String::from_utf8(a.fragment.canonical_bytes()).unwrap();
    std::fs::write(dir.join("canonical.txt"), format!("{canonical}\n")).unwrap();
    std::fs::write(
        dir.join("fragment.json"),
        format!("{}\n", serde_json::to_string_pretty(&a.fragment).unwrap()),
    )
    .unwrap();
    println!("wrote canonical.txt and fragment.json ({canonical})");
}
