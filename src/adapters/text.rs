//! Bounded text adapter.
//!
//! Supported grammar — the complete contract (see `docs/GRAMMAR.md`):
//!
//! ```text
//! active:  Entity VERB Entity (Number)? Object "."
//! passive: Entity RECEIVE-VERB (Number)? Object FROM Entity "."
//! ```
//!
//! The host only transports bytes: UTF-8 validity (guaranteed by `&str`),
//! the 64-byte bound, and span slicing. Word classification, role
//! assignment, quantity parsing, and every verdict execute in
//! `mncs.ingest.parse` through [`LanguageRuntime::parse_transfer`].

use crate::adapters::Adapter;
use crate::canonical::{assemble, CanonicalFacts, Ingested};
use crate::error::IngestError;
use crate::ir::{AdapterKind, Span, MAX_SPELLING_BYTES, MAX_TEXT_BYTES};
use crate::language::{status, LanguageRuntime};

pub struct TextAdapter;

impl Adapter for TextAdapter {
    type Input<'a> = &'a str;

    fn kind(&self) -> AdapterKind {
        AdapterKind::Text
    }

    fn ingest(&self, rt: &LanguageRuntime, input: &str) -> Result<Ingested, IngestError> {
        const ADAPTER: &str = "text";
        let bytes = input.as_bytes();
        if bytes.len() > MAX_TEXT_BYTES {
            return Err(IngestError::Overlong {
                adapter: ADAPTER.to_owned(),
                detail: format!("input is {} bytes, bound is {MAX_TEXT_BYTES}", bytes.len()),
            });
        }
        let parse = rt.parse_transfer(bytes)?;
        match parse.status {
            status::OK => {}
            status::UNSUPPORTED_VERB => {
                return Err(IngestError::unsupported(
                    ADAPTER,
                    "no supported transfer verb (gave/give/gives/received/receive/receives)",
                ));
            }
            status::MISSING_FIELD => {
                return Err(IngestError::missing_field(
                    ADAPTER,
                    "transfer participant (source, target, or object)",
                ));
            }
            status::MALFORMED_QUANTITY => {
                return Err(IngestError::malformed(
                    ADAPTER,
                    "quantity is not zero..ten or 0..999",
                ));
            }
            status::EMPTY => {
                return Err(IngestError::malformed(ADAPTER, "empty input"));
            }
            status::BAD_SHAPE => {
                return Err(IngestError::malformed(
                    ADAPTER,
                    "input is outside the bounded transfer grammar",
                ));
            }
            other => {
                return Err(IngestError::Language(format!(
                    "unknown parse status {other}"
                )));
            }
        }
        let source = slice(bytes, parse.source, "source")?;
        let target = slice(bytes, parse.target, "target")?;
        let object_span = slice(bytes, parse.object, "object")?;
        let quantity = parse.quantity_present.then_some(parse.quantity);
        let mut spans = vec![
            span("word:verb", parse.verb_span),
            span("word:source", parse.source),
            span("word:target", parse.target),
            span("word:object", parse.object),
        ];
        if parse.quantity_present {
            spans.push(span("word:quantity", parse.quantity_span));
        }
        let facts = CanonicalFacts {
            event: crate::ir::EVENT_TRANSFER,
            source: source.to_vec(),
            target: target.to_vec(),
            object_code: parse.object_code,
            object_spelling: object_span.to_vec(),
            quantity,
        };
        Ok(assemble(
            AdapterKind::Text,
            "mncs.ingest/ingest_parse",
            bytes,
            facts,
            spans,
            parse.object_code == crate::ir::OBJECT_UNKNOWN,
        ))
    }
}

fn slice<'a>(
    bytes: &'a [u8],
    span: Option<(u64, u64)>,
    role: &str,
) -> Result<&'a [u8], IngestError> {
    let (start, length) = span.ok_or_else(|| {
        IngestError::Language(format!("MNCS parse returned no {role} span on success"))
    })?;
    if length as usize > MAX_SPELLING_BYTES {
        return Err(IngestError::Language(format!(
            "{role} span length {length} exceeds bound {MAX_SPELLING_BYTES}"
        )));
    }
    let (start, length) = (start as usize, length as usize);
    bytes.get(start..start + length).ok_or_else(|| {
        IngestError::Language(format!(
            "{role} span [{start}..{}] is outside the input",
            start + length
        ))
    })
}

fn span(field: &str, span: Option<(u64, u64)>) -> Span {
    let (start, end) = span
        .map(|(start, length)| (Some(start), Some(start + length)))
        .unwrap_or((None, None));
    Span {
        field: field.to_owned(),
        start,
        end,
    }
}
