//! Adapter boundary: source-specific interpretation in, canonical facts out.
//!
//! Each adapter owns its source representation and nothing else. All three
//! obey one contract: extract fields through transport-appropriate means,
//! decide meaning through executed MNCS verdicts, and return [`Ingested`]
//! with full provenance. Adapters never implement retention, retrieval, or
//! any other downstream policy.

pub mod native;
pub mod structured;
pub mod text;

pub use native::{NativeAdapter, NativeTransfer};
pub use structured::StructuredAdapter;
pub use text::TextAdapter;

use crate::canonical::Ingested;
use crate::error::IngestError;
use crate::ir::AdapterKind;
use crate::language::LanguageRuntime;

/// The adapter contract. `Input` differs per source — text takes the raw
/// sentence, structured takes JSON bytes, native takes typed values — but
/// the output contract is one canonical form.
pub trait Adapter {
    type Input<'a>;

    fn kind(&self) -> AdapterKind;

    fn ingest(&self, rt: &LanguageRuntime, input: Self::Input<'_>)
        -> Result<Ingested, IngestError>;
}
