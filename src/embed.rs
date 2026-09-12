//! Retained embed sessions (Tranche G).
//!
//! The reference interpreter (`execute_with_policy`) pays full program
//! setup per call (~1.5 s fixed overhead in debug, ~140 ms in release).
//! `mncs-embed` retains artifact-level preparation across calls: the
//! program is lowered once per backend, the session reuses it, and every
//! call reports its step count. This module adopts that path for the
//! streaming hot verdicts while keeping the reference path as the
//! verification baseline — parity between them is tested, not assumed.
//!
//! Sessions have single-threaded host discipline, so the session sits
//! behind a mutex. Contention is measured in the benchmarks rather than
//! wished away.

use std::sync::Mutex;

use mncs_codegen::{lower_with_backend, plan_for_backend};
use mncs_embed::{Artifact, CallOptions, Session};
use mncs_model::{ArtifactRepresentation, CompilerArtifactRef, SSA_SCHEMA_VERSION};

use crate::error::IngestError;
use crate::language::{FieldOut, LineOut};

pub const EMBED_BACKEND: &str = "mncs-research-bytecode";
pub const EMBED_BUDGET: u64 = 500_000;

pub struct EmbedRuntime {
    session: Mutex<Session>,
    artifact_sha256: String,
}

impl EmbedRuntime {
    pub fn new(program: &mncs_model::Program) -> Result<Self, IngestError> {
        let ssa = program
            .lower_to_ssa()
            .map_err(|error| IngestError::Language(format!("ssa lowering failed: {error}")))?;
        let fingerprint = ssa
            .fingerprint()
            .map_err(|error| IngestError::Language(format!("ssa fingerprint failed: {error}")))?;
        let selected = CompilerArtifactRef::new(
            ArtifactRepresentation::SelectedSsa,
            SSA_SCHEMA_VERSION,
            fingerprint,
        );
        let plan = plan_for_backend(EMBED_BACKEND, selected.clone()).ok_or_else(|| {
            IngestError::Language(format!("no lowering plan for {EMBED_BACKEND}"))
        })?;
        let lowered = lower_with_backend(EMBED_BACKEND, program, &ssa, selected, &plan);
        let artifact = lowered.artifact.ok_or_else(|| {
            IngestError::Language(format!(
                "lowering refused: {:?}",
                lowered.diagnostics
            ))
        })?;
        let json = serde_json::to_vec(&artifact).map_err(|error| {
            IngestError::Language(format!("artifact serialization failed: {error}"))
        })?;
        let embedded =
            Artifact::from_json(&json).map_err(|error| IngestError::Language(error.to_string()))?;
        let sha = embedded.digest().to_owned();
        let session =
            Session::open(embedded).map_err(|error| IngestError::Language(error.to_string()))?;
        Ok(Self {
            session: Mutex::new(session),
            artifact_sha256: sha,
        })
    }

    pub fn artifact_sha256(&self) -> &str {
        &self.artifact_sha256
    }

    fn call(
        &self,
        function: &str,
        arguments: Vec<mncs_model::ExecutionValue>,
    ) -> Result<(Vec<mncs_model::ExecutionValue>, u64), IngestError> {
        let session = self.session.lock().map_err(|_| {
            IngestError::Language("embed session lock poisoned".to_owned())
        })?;
        let output = session.call(
            "mncs.ingest",
            function,
            arguments,
            &CallOptions::budgeted(EMBED_BUDGET),
        );
        if output.status != "returned" {
            return Err(IngestError::Language(format!(
                "{function} via embed: {}: {}",
                output.status,
                output.failure_reason.unwrap_or_default()
            )));
        }
        Ok((output.returned, output.steps))
    }

    /// Streaming line step with step count for benchmark attribution.
    pub fn next_line(
        &self,
        carry: &[u8],
        view: &[u8],
        eof: bool,
    ) -> Result<(LineOut, u64), IngestError> {
        let (record, steps) = self.call(
            "ingest_next_line",
            vec![
                crate::language::bytes256_for(carry),
                crate::language::u64_for(carry.len() as u64),
                crate::language::bytes_for(view),
                crate::language::u64_for(view.len() as u64),
                crate::language::bool_for(eof),
            ],
        )?;
        let fields = crate::language::record_fields(&record, "ingest_next_line")?;
        Ok((crate::language::read_line_out(&fields)?, steps))
    }

    pub fn field_at(&self, line: &[u8], start: u64) -> Result<(FieldOut, u64), IngestError> {
        let (record, steps) = self.call(
            "ingest_field_at",
            vec![
                crate::language::bytes256_for(line),
                crate::language::u64_for(line.len() as u64),
                crate::language::u64_for(start),
            ],
        )?;
        let fields = crate::language::record_fields(&record, "ingest_field_at")?;
        Ok((crate::language::read_field_out(&fields)?, steps))
    }

    pub fn classify_event(&self, word: &[u8]) -> Result<(i64, u64), IngestError> {
        let (values, steps) = self.call(
            "ingest_classify_event",
            vec![
                crate::language::bytes_for(word),
                crate::language::u64_for(word.len() as u64),
            ],
        )?;
        match values.first() {
            Some(mncs_model::ExecutionValue::Integer { value, .. }) => {
                let code = i64::try_from(*value).map_err(|_| {
                    IngestError::Language("classify_event out of range".to_owned())
                })?;
                Ok((code, steps))
            }
            other => Err(IngestError::Language(format!(
                "expected integer from classify_event, got {other:?}"
            ))),
        }
    }
}
