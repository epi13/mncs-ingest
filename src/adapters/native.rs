//! Native MNCS adapter.
//!
//! Input that is already machine-native and typed needs relatively little
//! reinterpretation: validation, provenance attachment, and canonical
//! serialization. The input shape uses canonical MNCS codes directly, so a
//! wrong code or a code/spelling conflict fails here instead of being
//! reinterpreted into something convenient.
//!
//! [`NativeTransfer`] is deliberately boring: no transport decoding, no
//! spelling tables on the host. Meaning still executes through
//! `ingest_validate` like every other adapter.

use crate::adapters::Adapter;
use crate::canonical::{assemble, object_label, CanonicalFacts, Ingested};
use crate::error::IngestError;
use crate::ir::{AdapterKind, Span, MAX_SPELLING_BYTES};
use crate::language::LanguageRuntime;

/// Already-typed machine-native transfer input.
#[derive(Debug, Clone)]
pub struct NativeTransfer {
    /// Canonical event code (`1` = transfer).
    pub event_code: i64,
    pub source: String,
    pub target: String,
    /// Canonical object code (`1` = apple, `2` = orange, `0` = unknown).
    pub object_code: i64,
    /// Required when `object_code` is `0`; must be absent-or-agreeing
    /// otherwise, so a code can never silently override a spelling.
    pub object_spelling: Option<String>,
    /// `None` is an absent quantity; `Some(0)` is an explicit zero.
    pub quantity: Option<i64>,
}

pub struct NativeAdapter;

impl Adapter for NativeAdapter {
    type Input<'a> = NativeTransfer;

    fn kind(&self) -> AdapterKind {
        AdapterKind::Native
    }

    fn ingest(&self, rt: &LanguageRuntime, input: NativeTransfer) -> Result<Ingested, IngestError> {
        const ADAPTER: &str = "native";
        non_empty(&input.source, ADAPTER, "source")?;
        non_empty(&input.target, ADAPTER, "target")?;
        bound(&input.source, ADAPTER, "source")?;
        bound(&input.target, ADAPTER, "target")?;

        let object_spelling = match (input.object_code, input.object_spelling) {
            (1 | 2, Some(spelling)) => {
                let canonical = object_label(input.object_code).unwrap_or_default();
                if !spelling.eq_ignore_ascii_case(canonical)
                    && !spelling.eq_ignore_ascii_case(&format!("{canonical}s"))
                {
                    return Err(IngestError::ConflictingFields {
                        adapter: ADAPTER.to_owned(),
                        detail: format!(
                            "object code {} means {canonical:?} but spelling is {spelling:?}",
                            input.object_code
                        ),
                    });
                }
                spelling.as_bytes().to_vec()
            }
            (1 | 2, None) => object_label(input.object_code)
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
            (0, Some(spelling)) => {
                non_empty(&spelling, ADAPTER, "object_spelling")?;
                bound(&spelling, ADAPTER, "object_spelling")?;
                spelling.as_bytes().to_vec()
            }
            (0, None) => {
                return Err(IngestError::missing_field(
                    ADAPTER,
                    "object_spelling for unknown object",
                ));
            }
            (other, _) => {
                return Err(IngestError::unsupported(
                    ADAPTER,
                    format!("unknown native object code {other}"),
                ));
            }
        };

        match rt.validate(
            input.event_code,
            !input.source.is_empty(),
            !input.target.is_empty(),
            input.quantity.is_some(),
            input.quantity.unwrap_or(0),
        )? {
            0 => {}
            1 => {
                return Err(IngestError::unsupported(
                    ADAPTER,
                    format!("unknown native event code {}", input.event_code),
                ));
            }
            2 => {
                return Err(IngestError::missing_field(
                    ADAPTER,
                    "transfer participant (source, target, or object)",
                ));
            }
            3 => {
                return Err(IngestError::malformed(ADAPTER, "quantity must be 0..=999"));
            }
            other => {
                return Err(IngestError::Language(format!(
                    "unknown validate status {other}"
                )));
            }
        }

        let facts = CanonicalFacts {
            event: input.event_code,
            source: input.source.as_bytes().to_vec(),
            target: input.target.as_bytes().to_vec(),
            object_code: input.object_code,
            object_spelling,
            quantity: input.quantity,
        };
        let raw = format!(
            "native:{}:{}:{}:{}:{}",
            input.event_code,
            input.source,
            input.target,
            input.object_code,
            input.quantity.map(|q| q.to_string()).unwrap_or_default()
        );
        let spans = ["source", "target", "object", "quantity"]
            .into_iter()
            .map(|field| Span {
                field: format!("native:{field}"),
                start: None,
                end: None,
            })
            .collect();
        Ok(assemble(
            AdapterKind::Native,
            "mncs.ingest/ingest_validate (native)",
            raw.as_bytes(),
            facts,
            spans,
            input.object_code == crate::ir::OBJECT_UNKNOWN,
        ))
    }
}

fn non_empty(value: &str, adapter: &str, field: &str) -> Result<(), IngestError> {
    if value.is_empty() {
        return Err(IngestError::malformed(
            adapter,
            format!("field {field} must not be empty"),
        ));
    }
    Ok(())
}

fn bound(value: &str, adapter: &str, field: &str) -> Result<(), IngestError> {
    if value.as_bytes().len() > MAX_SPELLING_BYTES {
        return Err(IngestError::Overlong {
            adapter: adapter.to_owned(),
            detail: format!("field {field} exceeds {MAX_SPELLING_BYTES} bytes"),
        });
    }
    Ok(())
}
