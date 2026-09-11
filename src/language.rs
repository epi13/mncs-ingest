//! Executed MNCS authority.
//!
//! [`LanguageRuntime`] compiles `language/mncs/ingest.mncs` (plus its
//! `mncs.ingest.*` dependencies) through the pinned reference compiler and
//! executes semantic verdicts through the reference interpreter — and, in
//! [`LanguageRuntime::backend_matrix`], through real backend pipelines.
//! Adapters transport bytes across the boundary; every classification,
//! validation, equality, and ordering verdict is executed MNCS code.
//!
//! Module layout mirrors `mncs-memory`'s proven `language.rs` pattern; the
//! `.mncs` sources it loads are ingest's own under `language/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mncs_compiler::{ModuleResolver, ReferenceCompiler};
use mncs_model::{
    execute_with_policy, ExecutionPolicy, ExecutionRequest, ExecutionStatus, ExecutionTarget,
    ExecutionValue, Program, EXECUTION_REQUEST_SCHEMA_VERSION,
};
use mncs_syntax::{SourceArtifactKind, SourceEnvelope, SourceOrigin, SourceOriginKind};

use crate::error::IngestError;

/// Generous but finite fuel for the bounded slice: the parser folds over at
/// most 64 bytes with small constant fan-out per position.
const STEP_BUDGET: u64 = 500_000;

/// Parse verdict mirroring `mncs.ingest.parse.TransferParse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferParse {
    pub status: i64,
    pub verb: i64,
    pub verb_span: Option<(u64, u64)>,
    pub source: Option<(u64, u64)>,
    pub target: Option<(u64, u64)>,
    pub object_code: i64,
    pub object: Option<(u64, u64)>,
    pub quantity: i64,
    pub quantity_present: bool,
    pub quantity_span: Option<(u64, u64)>,
}

impl TransferParse {
    pub fn ok(&self) -> bool {
        self.status == 0
    }
}

/// Parse status codes assigned by `mncs.ingest.parse`.
pub mod status {
    pub const OK: i64 = 0;
    pub const UNSUPPORTED_VERB: i64 = 1;
    pub const MISSING_FIELD: i64 = 2;
    pub const MALFORMED_QUANTITY: i64 = 3;
    pub const BAD_SHAPE: i64 = 4;
    pub const EMPTY: i64 = 5;
}

pub struct LanguageRuntime {
    source_root: PathBuf,
    program: Program,
    semantic_identity: String,
}

impl LanguageRuntime {
    /// Load from `language/` inside the crate directory.
    pub fn from_crate_dir() -> Result<Self, IngestError> {
        Self::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("language"))
    }

    pub fn new(source_root: impl Into<PathBuf>) -> Result<Self, IngestError> {
        let source_root = source_root.into();
        let core_path = source_root.join("mncs/ingest.mncs");
        let text = fs::read_to_string(&core_path)
            .map_err(|e| IngestError::Language(format!("unable to read MNCS source: {e}")))?;
        let envelope = SourceEnvelope::new(
            SourceArtifactKind::Program,
            core_path.to_string_lossy().to_string(),
            SourceOrigin {
                kind: SourceOriginKind::Path,
                locator: Some(core_path.to_string_lossy().to_string()),
            },
            text,
        );
        let resolver = FileModuleResolver {
            root: source_root.clone(),
        };
        let compiler = ReferenceCompiler::default();
        let front_end = compiler.front_end_with_resolver(envelope, &resolver);
        if !front_end.is_valid() {
            return Err(IngestError::Language(format!(
                "MNCS semantic source is invalid: {}",
                format_diagnostics(&front_end.diagnostics)
            )));
        }
        let program = front_end
            .program
            .ok_or_else(|| IngestError::Language("front end produced no program".to_owned()))?;
        let semantic_identity = program
            .content_fingerprint()
            .map_err(|e| IngestError::Language(format!("fingerprint failed: {e}")))?;
        Ok(Self {
            source_root,
            program,
            semantic_identity,
        })
    }

    pub fn semantic_identity(&self) -> &str {
        &self.semantic_identity
    }

    /// Compile any root file against the same resolver and return the full
    /// diagnostic list. Debugging aid for adapter program development.
    pub fn diagnose_root(source_root: &Path, root_file: &str) -> String {
        let core_path = source_root.join(root_file);
        let text = match fs::read_to_string(&core_path) {
            Ok(text) => text,
            Err(error) => return format!("unreadable: {error}"),
        };
        let envelope = SourceEnvelope::new(
            SourceArtifactKind::Program,
            core_path.to_string_lossy().to_string(),
            SourceOrigin {
                kind: SourceOriginKind::Path,
                locator: Some(core_path.to_string_lossy().to_string()),
            },
            text,
        );
        let resolver = FileModuleResolver {
            root: source_root.to_path_buf(),
        };
        let compiler = ReferenceCompiler::default();
        let front_end = compiler.front_end_with_resolver(envelope, &resolver);
        format_diagnostics(&front_end.diagnostics)
    }

    pub fn source_root(&self) -> &Path {
        &self.source_root
    }

    /// Bounded text path: raw sentence bytes in, canonical role spans out.
    pub fn parse_transfer(&self, text: &[u8]) -> Result<TransferParse, IngestError> {
        let record = self.call_record(
            "ingest_parse",
            vec![bytes_value(text), u64_value(text.len() as u64)],
        )?;
        Ok(TransferParse {
            status: field_i64(&record, "status")?,
            verb: field_i64(&record, "verb")?,
            verb_span: field_span(&record, "verb_start", "verb_length")?,
            source: field_span(&record, "source_start", "source_length")?,
            target: field_span(&record, "target_start", "target_length")?,
            object_code: field_i64(&record, "object_code")?,
            object: field_span(&record, "object_start", "object_length")?,
            quantity: field_i64(&record, "quantity")?,
            quantity_present: field_bool(&record, "quantity_present")?,
            quantity_span: field_span(&record, "quantity_start", "quantity_length")?,
        })
    }

    pub fn classify_event(&self, word: &[u8]) -> Result<i64, IngestError> {
        self.call_i64(
            "ingest_classify_event",
            vec![bytes_value(word), u64_value(word.len() as u64)],
        )
    }

    pub fn classify_object(&self, word: &[u8]) -> Result<i64, IngestError> {
        self.call_i64(
            "ingest_classify_object",
            vec![bytes_value(word), u64_value(word.len() as u64)],
        )
    }

    pub fn validate(
        &self,
        event: i64,
        src_present: bool,
        tgt_present: bool,
        qty_present: bool,
        qty: i64,
    ) -> Result<i64, IngestError> {
        self.call_i64(
            "ingest_validate",
            vec![
                i64_value(event),
                bool_value(src_present),
                bool_value(tgt_present),
                bool_value(qty_present),
                i64_value(qty),
            ],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn codes_equal(
        &self,
        a_event: i64,
        a_object: i64,
        a_qty_present: bool,
        a_qty: i64,
        b_event: i64,
        b_object: i64,
        b_qty_present: bool,
        b_qty: i64,
    ) -> Result<bool, IngestError> {
        Ok(self.call_i64(
            "ingest_codes_equal",
            vec![
                i64_value(a_event),
                i64_value(a_object),
                bool_value(a_qty_present),
                i64_value(a_qty),
                i64_value(b_event),
                i64_value(b_object),
                bool_value(b_qty_present),
                i64_value(b_qty),
            ],
        )? == 1)
    }

    pub fn spell_equal(&self, a: &[u8], b: &[u8]) -> Result<bool, IngestError> {
        Ok(self.call_i64(
            "ingest_spell_equal",
            vec![
                bytes_value(a),
                u64_value(a.len() as u64),
                bytes_value(b),
                u64_value(b.len() as u64),
            ],
        )? == 1)
    }

    /// Deterministic ordering executed in MNCS over a code projection.
    pub fn sort_codes(&self, codes: &[u64]) -> Result<Vec<u64>, IngestError> {
        let result = self.call(
            "ingest_sort_codes",
            vec![u64_window(codes), u64_value(codes.len() as u64)],
        )?;
        match result.first() {
            Some(ExecutionValue::Sequence { values }) => values
                .iter()
                .map(|value| match value {
                    ExecutionValue::Integer { value, .. } => u64::try_from(*value).map_err(|_| {
                        IngestError::Language("sort_codes returned out-of-range lane".to_owned())
                    }),
                    other => Err(IngestError::Language(format!(
                        "sort_codes returned non-integer lane: {other:?}"
                    ))),
                })
                .take(codes.len())
                .collect(),
            other => Err(IngestError::Language(format!(
                "expected sequence from sort_codes, got {other:?}"
            ))),
        }
    }

    /// Diagnostics probe: `[count, verb_idx, verb_count, c0..c4]`.
    pub fn debug_pipeline(&self, text: &[u8]) -> Result<Vec<i64>, IngestError> {
        match self
            .call(
                "ingest_debug",
                vec![bytes_value(text), u64_value(text.len() as u64)],
            )?
            .first()
        {
            Some(ExecutionValue::Sequence { values }) => values
                .iter()
                .map(|value| match value {
                    ExecutionValue::Integer { value, .. } => i64::try_from(*value)
                        .map_err(|_| IngestError::Language("debug lane out of range".to_owned())),
                    other => Err(IngestError::Language(format!(
                        "debug lane not integer: {other:?}"
                    ))),
                })
                .collect(),
            other => Err(IngestError::Language(format!(
                "expected sequence from ingest_debug, got {other:?}"
            ))),
        }
    }

    pub fn is_sorted(&self, codes: &[u64]) -> Result<bool, IngestError> {
        Ok(self.call_i64(
            "ingest_is_sorted",
            vec![u64_window(codes), u64_value(codes.len() as u64)],
        )? == 1)
    }

    /// Cross-backend execution proof: the same MNCS ordering entry point
    /// compiled and executed through real backend pipelines.
    pub fn backend_matrix(&self) -> Result<Vec<BackendObservation>, IngestError> {
        use mncs_codegen::{execute_backend, lower_with_backend, plan_for_backend};
        use mncs_model::{ArtifactRepresentation, CompilerArtifactRef, SSA_SCHEMA_VERSION};

        let mut observations = Vec::new();
        for backend in ["mncs-research-bytecode", "mncs-portable-wasm-mvp"] {
            let ssa = self
                .program
                .lower_to_ssa()
                .map_err(|e| IngestError::Language(format!("ssa lowering failed: {e}")))?;
            let fingerprint = ssa
                .fingerprint()
                .map_err(|e| IngestError::Language(format!("ssa fingerprint failed: {e}")))?;
            let selected = CompilerArtifactRef::new(
                ArtifactRepresentation::SelectedSsa,
                SSA_SCHEMA_VERSION,
                fingerprint,
            );
            let Some(plan) = plan_for_backend(backend, selected.clone()) else {
                observations.push(BackendObservation {
                    backend: backend.to_owned(),
                    compilation: "unknown".to_owned(),
                    execution: "unsupported".to_owned(),
                    note: "backend adapter unavailable".to_owned(),
                });
                continue;
            };
            let lowered = lower_with_backend(backend, &self.program, &ssa, selected, &plan);
            let Some(artifact) = lowered.artifact else {
                observations.push(BackendObservation {
                    backend: backend.to_owned(),
                    compilation: format!("{:?}", lowered.status),
                    execution: "unsupported".to_owned(),
                    note: format!("lowering refused: {:?}", lowered.diagnostics),
                });
                continue;
            };
            let request = ExecutionRequest {
                schema_version: EXECUTION_REQUEST_SCHEMA_VERSION.to_owned(),
                target: ExecutionTarget {
                    module: "mncs.ingest".to_owned(),
                    function: "ingest_is_sorted".to_owned(),
                },
                arguments: vec![u64_window(&[3, 1, 2, 0, 0, 0, 0, 0]), u64_value(3)],
                step_budget: STEP_BUDGET,
                policy: ExecutionPolicy::default(),
                host_grants: Vec::new(),
                call_depth_budget: None,
            };
            let execution = execute_backend(&artifact, &request);
            observations.push(BackendObservation {
                backend: backend.to_owned(),
                compilation: format!("{:?}", lowered.status),
                execution: format!("{:?}", execution.status),
                note: "bounded backend execution of the MNCS ordering witness".to_owned(),
            });
        }
        Ok(observations)
    }

    fn call_i64(&self, function: &str, arguments: Vec<ExecutionValue>) -> Result<i64, IngestError> {
        match self.call(function, arguments)?.first() {
            Some(ExecutionValue::Integer { value, .. }) => i64::try_from(*value).map_err(|_| {
                IngestError::Language(format!("{function} returned out-of-range integer"))
            }),
            other => Err(IngestError::Language(format!(
                "expected integer from {function}, got {other:?}"
            ))),
        }
    }

    fn call_record(
        &self,
        function: &str,
        arguments: Vec<ExecutionValue>,
    ) -> Result<Vec<(String, ExecutionValue)>, IngestError> {
        match self.call(function, arguments)?.first() {
            Some(ExecutionValue::Record { fields, .. }) => Ok(fields.to_vec()),
            other => Err(IngestError::Language(format!(
                "expected record from {function}, got {other:?}"
            ))),
        }
    }

    fn call(
        &self,
        function: &str,
        arguments: Vec<ExecutionValue>,
    ) -> Result<Vec<ExecutionValue>, IngestError> {
        let request = ExecutionRequest {
            schema_version: EXECUTION_REQUEST_SCHEMA_VERSION.to_owned(),
            target: ExecutionTarget {
                module: "mncs.ingest".to_owned(),
                function: function.to_owned(),
            },
            arguments,
            step_budget: STEP_BUDGET,
            policy: ExecutionPolicy::default(),
            host_grants: Vec::new(),
            call_depth_budget: None,
        };
        let result = execute_with_policy(&self.program, &request);
        if result.status != ExecutionStatus::Returned {
            return Err(IngestError::Language(format!(
                "{function}: status was {:?}: {}",
                result.status,
                result
                    .failure
                    .map(|failure| failure.reason)
                    .unwrap_or_else(|| "no reason".to_owned())
            )));
        }
        Ok(result.returned)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendObservation {
    pub backend: String,
    pub compilation: String,
    pub execution: String,
    pub note: String,
}

fn field_i64(record: &[(String, ExecutionValue)], name: &str) -> Result<i64, IngestError> {
    match record.iter().find(|(key, _)| key == name) {
        Some((_, ExecutionValue::Integer { value, .. })) => i64::try_from(*value)
            .map_err(|_| IngestError::Language(format!("field {name} out of range"))),
        other => Err(IngestError::Language(format!(
            "field {name} is not an integer: {other:?}"
        ))),
    }
}

fn field_bool(record: &[(String, ExecutionValue)], name: &str) -> Result<bool, IngestError> {
    match record.iter().find(|(key, _)| key == name) {
        Some((_, ExecutionValue::Boolean { value })) => Ok(*value),
        other => Err(IngestError::Language(format!(
            "field {name} is not a boolean: {other:?}"
        ))),
    }
}

fn field_span(
    record: &[(String, ExecutionValue)],
    start_name: &str,
    len_name: &str,
) -> Result<Option<(u64, u64)>, IngestError> {
    let start = field_i64(record, start_name)?;
    let length = field_i64(record, len_name)?;
    if start < 0 || length < 0 {
        return Ok(None);
    }
    Ok(Some((start as u64, length as u64)))
}

fn bytes_value(bytes: &[u8]) -> ExecutionValue {
    ExecutionValue::Sequence {
        values: Arc::new(
            bytes
                .iter()
                .map(|byte| ExecutionValue::Byte {
                    value: i128::from(*byte),
                })
                .collect(),
        ),
    }
}

fn u64_window(codes: &[u64]) -> ExecutionValue {
    let mut values: Vec<ExecutionValue> = codes
        .iter()
        .map(|code| ExecutionValue::Integer {
            value: i128::from(*code),
            ty: mncs_model::IntegerType {
                bits: 64,
                signed: false,
            },
        })
        .collect();
    while values.len() < 8 {
        values.push(ExecutionValue::Integer {
            value: 0,
            ty: mncs_model::IntegerType {
                bits: 64,
                signed: false,
            },
        });
    }
    ExecutionValue::Sequence {
        values: Arc::new(values),
    }
}

fn u64_value(value: u64) -> ExecutionValue {
    ExecutionValue::Integer {
        value: i128::from(value),
        ty: mncs_model::IntegerType {
            bits: 64,
            signed: false,
        },
    }
}

fn i64_value(value: i64) -> ExecutionValue {
    ExecutionValue::Integer {
        value: i128::from(value),
        ty: mncs_model::IntegerType {
            bits: 64,
            signed: true,
        },
    }
}

fn bool_value(value: bool) -> ExecutionValue {
    ExecutionValue::Boolean { value }
}

fn format_diagnostics(diagnostics: &[mncs_syntax::SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "{} {:?} line {} col {}: {}",
                diagnostic.code,
                diagnostic.severity,
                diagnostic.span.line,
                diagnostic.span.column,
                diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

struct FileModuleResolver {
    root: PathBuf,
}

impl ModuleResolver for FileModuleResolver {
    fn resolve(&self, module: &str) -> Option<SourceEnvelope> {
        let path = self.root.join(format!("{}.mncs", module.replace('.', "/")));
        let text = fs::read_to_string(&path).ok()?;
        Some(SourceEnvelope::new(
            SourceArtifactKind::Program,
            path.to_string_lossy().to_string(),
            SourceOrigin {
                kind: SourceOriginKind::Path,
                locator: Some(path.to_string_lossy().to_string()),
            },
            text,
        ))
    }
}
