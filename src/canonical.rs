//! Canonical fragment assembly and MNCS-executed equality.
//!
//! Layout policy for schema v1 is positional and documented here:
//!
//! ```text
//! atom 0: event    "transfer"
//! atom 1: entity   <source spelling>
//! atom 2: entity   <target spelling>
//! atom 3: object   <canonical singular> | unknown <source spelling>
//! atom 4: quantity <decimal>            | absent when not given
//! ```
//!
//! The builder emits this layout directly; deterministic ordering over
//! variable-size fragments is future work and its MNCS primitive
//! (`ingest_sort_codes` / `ingest_is_sorted`) is pinned by conformance
//! tests, not run per ingest. Semantic equality itself always executes
//! in MNCS ([`mncs_equal`]): the host compares no meaning, only bytes
//! for transport.

use sha2::{Digest, Sha256};

use crate::error::IngestError;
use crate::ir::{
    AdapterKind, Atom, AtomKind, Confidence, Fragment, Provenance, Relation, Role, Span,
    FRAGMENT_SCHEMA_VERSION,
};
use crate::language::LanguageRuntime;

/// Canonical facts: the exact inputs to MNCS equality. Kept next to the
/// fragment through the ingest pipeline; the consumer handoff carries the
/// fragment alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFacts {
    pub event: i64,
    pub source: Vec<u8>,
    pub target: Vec<u8>,
    pub object_code: i64,
    pub object_spelling: Vec<u8>,
    pub quantity: Option<i64>,
}

/// A fragment plus the facts that prove it.
#[derive(Debug, Clone)]
pub struct Ingested {
    pub fragment: Fragment,
    pub facts: CanonicalFacts,
}

/// Render a canonical object code to its singular spelling. This two-entry
/// table mirrors `mncs.ingest.vocab` and exists only because MNCS cannot
/// return strings; `vocab_contract` pins the two sides together.
pub fn object_label(code: i64) -> Option<&'static str> {
    match code {
        1 => Some("apple"),
        2 => Some("orange"),
        _ => None,
    }
}

pub fn signal_id(raw: &[u8]) -> String {
    hex(&Sha256::digest(raw))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[allow(clippy::too_many_arguments)]
pub fn assemble(
    adapter: AdapterKind,
    interpreter: &str,
    raw: &[u8],
    facts: CanonicalFacts,
    spans: Vec<Span>,
    object_was_unknown: bool,
) -> Ingested {
    let object_label = object_label(facts.object_code)
        .map(str::to_owned)
        .unwrap_or_else(|| String::from_utf8_lossy(&facts.object_spelling).into_owned());
    let mut atoms = vec![
        Atom {
            id: 0,
            kind: AtomKind::Event,
            label: "transfer".to_owned(),
            value: None,
        },
        Atom {
            id: 1,
            kind: AtomKind::Entity,
            label: String::from_utf8_lossy(&facts.source).into_owned(),
            value: None,
        },
        Atom {
            id: 2,
            kind: AtomKind::Entity,
            label: String::from_utf8_lossy(&facts.target).into_owned(),
            value: None,
        },
        Atom {
            id: 3,
            kind: if object_was_unknown {
                AtomKind::Unknown
            } else {
                AtomKind::Object
            },
            label: object_label,
            value: None,
        },
    ];
    let mut relations = vec![
        Relation {
            role: Role::Source,
            event: 0,
            target: 1,
        },
        Relation {
            role: Role::Target,
            event: 0,
            target: 2,
        },
        Relation {
            role: Role::Object,
            event: 0,
            target: 3,
        },
    ];
    if let Some(qty) = facts.quantity {
        atoms.push(Atom {
            id: 4,
            kind: AtomKind::Quantity,
            label: qty.to_string(),
            value: Some(qty),
        });
        relations.push(Relation {
            role: Role::Quantity,
            event: 0,
            target: 4,
        });
    }
    let mut unknowns = Vec::new();
    if object_was_unknown {
        unknowns.push(format!(
            "object:unknown:{}",
            String::from_utf8_lossy(&facts.object_spelling)
        ));
    }
    let confidence = match adapter {
        AdapterKind::Text => {
            if object_was_unknown {
                Confidence::Uncertain
            } else {
                Confidence::Parsed
            }
        }
        AdapterKind::Structured | AdapterKind::Native => {
            if object_was_unknown {
                Confidence::Uncertain
            } else {
                Confidence::Direct
            }
        }
    };
    let fragment = Fragment {
        schema: FRAGMENT_SCHEMA_VERSION.to_owned(),
        atoms,
        relations,
        unknowns,
        provenance: Provenance {
            adapter,
            signal_id: signal_id(raw),
            interpreter: interpreter.to_owned(),
            spans,
            confidence,
        },
    };
    Ingested { fragment, facts }
}

/// Semantic equality executed in MNCS: closed-vocabulary codes compare in
/// `ingest_codes_equal`, open-vocabulary spellings in `ingest_spell_equal`.
/// Unknown-object spellings compare only when both sides are unknown
/// (equal codes already imply equal known objects).
pub fn mncs_equal(
    rt: &LanguageRuntime,
    a: &CanonicalFacts,
    b: &CanonicalFacts,
) -> Result<bool, IngestError> {
    if !rt.codes_equal(
        a.event,
        a.object_code,
        a.quantity.is_some(),
        a.quantity.unwrap_or(0),
        b.event,
        b.object_code,
        b.quantity.is_some(),
        b.quantity.unwrap_or(0),
    )? {
        return Ok(false);
    }
    if !rt.spell_equal(&a.source, &b.source)? {
        return Ok(false);
    }
    if !rt.spell_equal(&a.target, &b.target)? {
        return Ok(false);
    }
    if a.object_code == 0 && !rt.spell_equal(&a.object_spelling, &b.object_spelling)? {
        return Ok(false);
    }
    Ok(true)
}
