//! Structured event / JSON adapter.
//!
//! The simplest reliable representation and the canonical-contract anchor:
//!
//! ```json
//! {"event": "transfer", "from": "Alexander", "to": "Finn",
//!  "object": "apple", "quantity": 3}
//! ```
//!
//! `event`, `from`, and `to` are required; `object` and `quantity` are
//! optional but explicit when present (`quantity: 0` differs from a
//! missing quantity). Unknown fields are rejected: this is a strict
//! machine boundary, and silent field-dropping would launder meaning.
//!
//! The host only decodes JSON and extracts fields. Event support, object
//! codes, and quantity validation execute in MNCS (`ingest_classify_*`,
//! `ingest_validate`).

use crate::adapters::Adapter;
use crate::canonical::{assemble, CanonicalFacts, Ingested};
use crate::error::IngestError;
use crate::ir::{AdapterKind, Span, MAX_SPELLING_BYTES};
use crate::language::LanguageRuntime;

pub struct StructuredAdapter;

const ALLOWED: [&str; 5] = ["event", "from", "to", "object", "quantity"];

impl Adapter for StructuredAdapter {
    /// Raw JSON document bytes.
    type Input<'a> = &'a [u8];

    fn kind(&self) -> AdapterKind {
        AdapterKind::Structured
    }

    fn ingest(&self, rt: &LanguageRuntime, input: &[u8]) -> Result<Ingested, IngestError> {
        const ADAPTER: &str = "structured";
        let value: serde_json::Value = serde_json::from_slice(input)
            .map_err(|e| IngestError::malformed(ADAPTER, format!("invalid JSON: {e}")))?;
        let object = value.as_object().ok_or_else(|| {
            IngestError::malformed(ADAPTER, "top-level JSON value must be an object")
        })?;
        let mut names: Vec<&str> = object.keys().map(String::as_str).collect();
        names.sort_unstable();
        for name in names {
            if !ALLOWED.contains(&name) {
                return Err(IngestError::UnexpectedField {
                    adapter: ADAPTER.to_owned(),
                    field: name.to_owned(),
                });
            }
        }

        let event_word = required_string(object, ADAPTER, "event")?;
        let event = rt.classify_event(event_word.as_bytes())?;
        if event == 0 {
            return Err(IngestError::unsupported(
                ADAPTER,
                format!("unsupported event {event_word:?}"),
            ));
        }
        let source = required_entity(object, ADAPTER, "from")?;
        let target = required_entity(object, ADAPTER, "to")?;

        let mut unknown_object: Option<Vec<u8>> = None;
        let object_code = match object.get("object") {
            None => {
                return Err(IngestError::missing_field(ADAPTER, "object"));
            }
            Some(value) => {
                let word = string_field(value, ADAPTER, "object")?;
                non_empty(&word, ADAPTER, "object")?;
                bound(&word, ADAPTER, "object")?;
                let code = rt.classify_object(word.as_bytes())?;
                if code == crate::ir::OBJECT_UNKNOWN {
                    unknown_object = Some(word.as_bytes().to_vec());
                }
                code
            }
        };

        let quantity = match object.get("quantity") {
            None => None,
            Some(value) => {
                let qty = value.as_i64().ok_or_else(|| {
                    IngestError::malformed(ADAPTER, "field quantity must be an integer")
                })?;
                Some(qty)
            }
        };

        match rt.validate(event, true, true, quantity.is_some(), quantity.unwrap_or(0))? {
            0 => {}
            1 => {
                return Err(IngestError::unsupported(
                    ADAPTER,
                    "event failed MNCS validation",
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

        let object_spelling = match unknown_object {
            Some(spelling) => spelling,
            None => object
                .get("object")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
        };
        let facts = CanonicalFacts {
            event,
            source: source.as_bytes().to_vec(),
            target: target.as_bytes().to_vec(),
            object_code,
            object_spelling,
            quantity,
        };
        let spans = ["event", "from", "to", "object", "quantity"]
            .into_iter()
            .filter(|field| *field != "quantity" || quantity.is_some())
            .filter(|field| *field != "object" || object.contains_key("object"))
            .map(|field| Span {
                field: format!("json:{field}"),
                start: None,
                end: None,
            })
            .collect();
        Ok(assemble(
            AdapterKind::Structured,
            "mncs.ingest/ingest_validate+classify",
            input,
            facts,
            spans,
            object_code == crate::ir::OBJECT_UNKNOWN,
        ))
    }
}

fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    adapter: &str,
    field: &str,
) -> Result<String, IngestError> {
    match object.get(field) {
        None => Err(IngestError::missing_field(adapter, field)),
        Some(value) => string_field(value, adapter, field),
    }
}

fn string_field(
    value: &serde_json::Value,
    adapter: &str,
    field: &str,
) -> Result<String, IngestError> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| IngestError::malformed(adapter, format!("field {field} must be a string")))
}

fn required_entity(
    object: &serde_json::Map<String, serde_json::Value>,
    adapter: &str,
    field: &str,
) -> Result<String, IngestError> {
    let value = required_string(object, adapter, field)?;
    non_empty(&value, adapter, field)?;
    bound(&value, adapter, field)?;
    Ok(value)
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
