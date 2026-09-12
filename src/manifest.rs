//! Incremental manifest ingestion (Tranche C).
//!
//! Manifests are a bounded JSON subset: top-level objects with string
//! keys and string / integer / boolean / nested values, depth ≤ 4,
//! documents ≤ 512 bytes. Two phases: `ManifestScanner` validates
//! structure incrementally over chunk views (bounded owned state
//! crosses each call); extraction reads typed pairs from the staged
//! bytes of a complete document.
//!
//! Unicode escapes (`\uXXXX`) are rejected by design (INGEST-P-003);
//! integers are ±99999; only `\"`, `\\`, `\n`, `\t` escapes survive.

use crate::error::IngestError;
use crate::language::LanguageRuntime;

/// Scanner state: the exact lanes of `mncs.flow.nest.NestState`,
/// decomposed because records cannot cross into calls (INGEST-P-009).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NestState {
    pub depth: u64,
    pub stack: [u64; 4],
    pub expect: u64,
    pub in_string: bool,
    pub esc: bool,
    pub max_depth: u64,
    pub seen_value: bool,
    pub in_literal: bool,
    pub after_comma: bool,
}

/// One scan step verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanVerdict {
    /// 0 ok-so-far, 1 complete, 2 truncated, 3 malformed.
    pub status: i64,
    pub state: NestState,
    pub position: u64,
    pub consumed: u64,
}

/// Typed manifest value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestValue {
    Str(Vec<u8>),
    Int(i64),
    Bool(bool),
    /// Opaque span (start, len) into the document; re-stageable.
    Nested { start: u64, len: u64 },
}

/// One extracted pair with source spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestPair {
    pub key: Vec<u8>,
    pub key_span: (u64, u64),
    pub value: ManifestValue,
    pub value_span: (u64, u64),
}

pub struct ManifestScanner {
    state: NestState,
    base: u64,
}

impl ManifestScanner {
    pub fn new() -> Self {
        Self {
            state: NestState::default(),
            base: 0,
        }
    }

    /// Scan one chunk view; at end-of-input pass `eof` with an empty view
    /// to finalize. Returns the verdict; state advances internally.
    pub fn scan(
        &mut self,
        rt: &LanguageRuntime,
        view: &[u8],
        eof: bool,
    ) -> Result<ScanVerdict, IngestError> {
        let out = rt.scan_chunk(&self.state, view, self.base, eof)?;
        self.base += out.consumed;
        self.state = out.state.clone();
        Ok(out)
    }
}

impl Default for ManifestScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate a whole document through chunk views of `width`, then
/// extract all pairs. Used by tests to prove split invariance.
pub fn ingest_manifest(
    rt: &LanguageRuntime,
    doc: &[u8],
    width: usize,
) -> Result<Vec<ManifestPair>, IngestError> {
    let mut scanner = ManifestScanner::new();
    let width = width.max(1);
    for view in doc.chunks(width) {
        let verdict = scanner.scan(rt, view, false)?;
        if verdict.status == 3 {
            return Err(IngestError::malformed(
                "manifest",
                format!("malformed structure at byte {}", verdict.position),
            ));
        }
    }
    let verdict = scanner.scan(rt, &[], true)?;
    match verdict.status {
        1 => extract_all(rt, doc),
        2 => Err(IngestError::malformed(
            "manifest",
            format!("truncated document at byte {}", verdict.position),
        )),
        _ => Err(IngestError::malformed(
            "manifest",
            format!("malformed structure at byte {}", verdict.position),
        )),
    }
}

fn extract_all(rt: &LanguageRuntime, doc: &[u8]) -> Result<Vec<ManifestPair>, IngestError> {
    if doc.len() > 512 {
        return Err(IngestError::Overlong {
            adapter: "manifest".to_owned(),
            detail: "document exceeds 512 bytes".to_owned(),
        });
    }
    let count = rt.pair_count(doc)?;
    if count < 0 || count > 64 {
        return Err(IngestError::Language(format!(
            "pair_count out of range: {count}"
        )));
    }
    (0..count as u64).map(|index| rt.pair_at(doc, index)).collect()
}
