//! Executed MNCS authority: the `.mncs` sources are not decoration.
//!
//! These tests prove the language runtime loads the real program, executes
//! real verdicts with exact spans, changes behavior when the policy
//! changes, and runs through real backend pipelines — not just the
//! reference interpreter.

mod common;

use common::rt;
use mncs_ingest::language::LanguageRuntime;

fn copy_language_tree() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mncs-ingest-authority-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("language");
    copy_dir(&source, &root);
    root
}

// Minimal recursive copy without new dependencies.
fn copy_dir(source: &std::path::Path, dest: &std::path::Path) {
    std::fs::create_dir_all(dest).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = dest.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn program_loads_with_stable_identity() {
    let first = rt();
    let second = rt();
    assert!(!first.semantic_identity().is_empty());
    assert_eq!(first.semantic_identity(), second.semantic_identity());
}

#[test]
fn parse_executes_with_exact_spans() {
    let rt = rt();
    let parse = rt.parse_transfer(b"Alexander gave Finn 3 apples.").unwrap();
    assert_eq!(parse.status, 0);
    assert_eq!(parse.verb, 1);
    assert_eq!(parse.source, Some((0, 9)));
    assert_eq!(parse.target, Some((15, 4)));
    assert_eq!(parse.object_code, 1);
    assert_eq!(parse.object, Some((22, 6)));
    assert_eq!(parse.quantity_present, true);
    assert_eq!(parse.quantity, 3);
    assert_eq!(parse.quantity_span, Some((20, 1)));

    let passive = rt
        .parse_transfer(b"Finn received three apples from Alexander.")
        .unwrap();
    assert_eq!(passive.status, 0);
    assert_eq!(passive.verb, 2);
    assert_eq!(passive.source, Some((32, 9)));
    assert_eq!(passive.target, Some((0, 4)));
    assert_eq!(passive.object_code, 1);
    assert_eq!(passive.quantity, 3);
}

#[test]
fn statuses_execute_for_each_failure_mode() {
    let rt = rt();
    assert_eq!(
        rt.parse_transfer(b"Alexander hugged Finn 3 apples.")
            .unwrap()
            .status,
        1
    );
    assert_eq!(
        rt.parse_transfer(b"Alexander gave Finn.").unwrap().status,
        2
    );
    assert_eq!(
        rt.parse_transfer(b"Alexander gave Finn 3x apples.")
            .unwrap()
            .status,
        3
    );
    assert_eq!(
        rt.parse_transfer(b"Alexander gave Finn Bob 3 apples.")
            .unwrap()
            .status,
        4
    );
    assert_eq!(rt.parse_transfer(b"").unwrap().status, 5);
}

#[test]
fn changing_mncs_policy_changes_the_verdict() {
    // The pressure-test heart: mutate one vocabulary byte in a copied
    // tree ("gave" no longer matches) and the same host call must fail.
    let root = copy_language_tree();
    let vocab_path = root.join("mncs/ingest/vocab.mncs");
    let vocab = std::fs::read_to_string(&vocab_path).unwrap();
    assert!(vocab.contains("token_gave"));
    // Break the "gave" table: first byte g(103) -> x(120).
    let mutated = vocab.replacen(
        "(103) as byte, (97) as byte, (118) as byte, (101) as byte",
        "(120) as byte, (97) as byte, (118) as byte, (101) as byte",
        1,
    );
    assert_ne!(vocab, mutated);
    std::fs::write(&vocab_path, mutated).unwrap();

    let altered = LanguageRuntime::new(&root).expect("mutated program still loads");
    assert_ne!(altered.semantic_identity(), rt().semantic_identity());
    let parse = altered
        .parse_transfer(b"Alexander gave Finn 3 apples.")
        .unwrap();
    assert_eq!(
        parse.status, 1,
        "without the gave-table, the verb is unsupported"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn ordering_primitives_execute() {
    let rt = rt();
    assert_eq!(rt.sort_codes(&[3, 1, 2]).unwrap(), [3 - 2, 3 - 1, 3]);
    assert_eq!(rt.sort_codes(&[]).unwrap(), Vec::<u64>::new());
    assert_eq!(rt.sort_codes(&[2, 2, 1]).unwrap(), [1, 2, 2]);
    assert!(rt.is_sorted(&[1, 2, 3]).unwrap());
    assert!(!rt.is_sorted(&[2, 1, 3]).unwrap());
}

#[test]
fn backend_matrix_executes_on_real_backends() {
    let rt = rt();
    let observations = rt.backend_matrix().expect("backend matrix runs");
    assert_eq!(observations.len(), 2);
    for observation in &observations {
        assert_eq!(
            observation.execution, "Returned",
            "backend {} must execute, got {:?}",
            observation.backend, observation
        );
    }
}
