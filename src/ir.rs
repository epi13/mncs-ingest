//! Canonical semantic IR for the deterministic convergence slice.
//!
//! The model is deliberately small: entities, one event kind (`transfer`),
//! a closed object vocabulary resolved by executed MNCS policy, quantities,
//! four relation roles, and explicit unknowns. Semantic authority over the
//! closed vocabulary lives in `language/mncs/ingest/*.mncs`; this module is
//! the typed host-side carrier plus deterministic layout (ordering,
//! serialization, hashing).
//!
//! Equality rule: two fragments are *semantically* equal when their
//! [`Fragment::semantic_key`] strings are identical. Provenance is excluded
//! from that key on purpose — equivalent meaning from distinct sources must
//! compare equal while retaining distinct provenance.

use serde::{Deserialize, Serialize};

/// Versioned schema identity. Bumped only on breaking canonical changes.
pub const FRAGMENT_SCHEMA_VERSION: &str = "mncs-ingest/fragment-v1";

/// Canonical event code for `transfer`. Assigned by executed MNCS policy
/// (`mncs.ingest.vocab.classify_event`); the host never invents it.
pub const EVENT_TRANSFER: i64 = 1;

/// Canonical object codes, assigned by executed MNCS policy.
pub const OBJECT_APPLE: i64 = 1;
pub const OBJECT_ORANGE: i64 = 2;
/// Marker for an object word outside the closed vocabulary.
pub const OBJECT_UNKNOWN: i64 = 0;

/// Transport bound: the executed MNCS parser scans at most this many input
/// bytes. Inputs beyond it are rejected before MNCS execution.
pub const MAX_TEXT_BYTES: usize = 64;
/// Transport bound: at most this many whitespace-separated words per input.
pub const MAX_WORDS: usize = 8;
/// Transport bound: entity/object spellings longer than this are rejected.
pub const MAX_SPELLING_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    Text,
    Structured,
    Native,
}

impl AdapterKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Structured => "structured",
            Self::Native => "native",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Byte-exact structured or already-validated native input.
    Direct,
    /// Bounded-grammar text parse; structure was interpreted, not given.
    Parsed,
    /// Fragment is partial: at least one unknown is load-bearing.
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtomKind {
    Entity,
    Event,
    Object,
    Quantity,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Source,
    Target,
    Object,
    Quantity,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Target => "target",
            Self::Object => "object",
            Self::Quantity => "quantity",
        }
    }
}

/// One typed semantic unit. `id` is a deterministic layout index assigned
/// after canonical sorting, never a stable cross-fragment identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Atom {
    pub id: u64,
    pub kind: AtomKind,
    /// Canonical spelling: entities keep source spelling, known objects use
    /// the singular canonical form, quantities render decimally.
    pub label: String,
    /// Only meaningful for quantities. `None` vs `Some(0)` is load-bearing:
    /// a missing quantity and an explicit zero must not collapse.
    pub value: Option<i64>,
}

/// A directed role assignment from the frame event to a participant atom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    pub role: Role,
    pub event: u64,
    pub target: u64,
}

/// A source span or structured field supporting one interpretation step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// E.g. `word:0`, `json:from`, `native:source`.
    pub field: String,
    pub start: Option<u64>,
    pub end: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub adapter: AdapterKind,
    /// Hex sha256 over the exact raw input bytes. Audit/replay key.
    pub signal_id: String,
    /// Which adapter interpretation produced this fragment.
    pub interpreter: String,
    pub spans: Vec<Span>,
    pub confidence: Confidence,
}

/// The canonical semantic fragment handed to downstream consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fragment {
    pub schema: String,
    pub atoms: Vec<Atom>,
    pub relations: Vec<Relation>,
    /// Explicit unknowns, e.g. `object:unknown:parsnip`. Never empty when
    /// confidence is [`Confidence::Uncertain`].
    pub unknowns: Vec<String>,
    pub provenance: Provenance,
}

impl Fragment {
    /// Deterministic semantic key. Excludes provenance and atom ids by
    /// construction: ids are layout artifacts, provenance is identity.
    pub fn semantic_key(&self) -> String {
        let mut out = String::from("frag:v1");
        for atom in &self.atoms {
            out.push_str(&format!(
                "\natom {:?} {} {}",
                atom.kind,
                escape(&atom.label),
                atom.value
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_owned())
            ));
        }
        for rel in &self.relations {
            out.push_str(&format!(
                "\nrel {} {} {}",
                rel.role.as_str(),
                rel.event,
                rel.target
            ));
        }
        for unknown in &self.unknowns {
            out.push_str(&format!("\nunknown {}", escape(unknown)));
        }
        out
    }

    pub fn semantically_equal(&self, other: &Fragment) -> bool {
        self.schema == other.schema && self.semantic_key() == other.semantic_key()
    }

    /// Canonical bytes: schema line + semantic key. Deterministic across
    /// runs, processes, and adapters.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        format!("{}\n{}", self.schema, self.semantic_key()).into_bytes()
    }
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(' ', "\\s")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fragment() -> Fragment {
        Fragment {
            schema: FRAGMENT_SCHEMA_VERSION.to_owned(),
            atoms: vec![Atom {
                id: 0,
                kind: AtomKind::Entity,
                label: "Alexander".to_owned(),
                value: None,
            }],
            relations: vec![],
            unknowns: vec![],
            provenance: Provenance {
                adapter: AdapterKind::Text,
                signal_id: "00".to_owned(),
                interpreter: "test".to_owned(),
                spans: vec![],
                confidence: Confidence::Parsed,
            },
        }
    }

    #[test]
    fn semantic_key_ignores_provenance_and_ids() {
        let mut a = fragment();
        let mut b = fragment();
        b.atoms[0].id = 99;
        b.provenance.adapter = AdapterKind::Native;
        b.provenance.signal_id = "ff".to_owned();
        assert!(a.semantically_equal(&b));
        a.atoms[0].label = "Finn".to_owned();
        assert!(!a.semantically_equal(&b));
    }

    #[test]
    fn zero_quantity_differs_from_missing_quantity() {
        let mut a = fragment();
        let mut b = fragment();
        a.atoms.push(Atom {
            id: 1,
            kind: AtomKind::Quantity,
            label: "0".to_owned(),
            value: Some(0),
        });
        b.atoms.push(Atom {
            id: 1,
            kind: AtomKind::Quantity,
            label: "-".to_owned(),
            value: None,
        });
        assert!(!a.semantically_equal(&b));
    }
}
