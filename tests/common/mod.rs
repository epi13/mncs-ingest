//! Shared test harness: one runtime per suite, loaded once.
//!
//! The MNCS program compiles once per test binary; frontend elaboration
//! dominates suite time, so sharing one `LanguageRuntime` across the
//! tests in a binary keeps the suite practical.

use mncs_ingest::language::LanguageRuntime;
use std::sync::OnceLock;

static RUNTIME: OnceLock<LanguageRuntime> = OnceLock::new();

pub fn rt() -> &'static LanguageRuntime {
    RUNTIME.get_or_init(|| LanguageRuntime::from_crate_dir().expect("MNCS ingest program loads"))
}
