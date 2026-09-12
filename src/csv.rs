//! Streaming delimited-record ingestion (Tranche A).
//!
//! Format (CSV-lite, byte-oriented; see `docs/STREAMING.md`): records are
//! `\n`-terminated lines (`\r\n` tolerated), fields split on `,`, at most
//! 8 fields of at most 63 unescaped bytes each, `\` escapes any byte
//! except newline/CR. Lines are at most 255 raw bytes.
//!
//! Division of labor: the host windows the byte stream into ≤256-byte
//! views and holds the carry between calls (transport). Every boundary,
//! escape, validation, and cursor decision executes in
//! `mncs.flow.lines` / `mncs.flow.fields`. The host never splits lines
//! or fields itself — that would hide the pressure this workload exists
//! to create.

use crate::error::IngestError;
use crate::language::LanguageRuntime;

pub const VIEW_BYTES: usize = 256;
pub const MAX_FIELDS: usize = 8;
pub const MAX_LINE_BYTES: usize = 255;
pub const MAX_FIELD_BYTES: usize = 63;

/// One ingested record with its source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRecord {
    pub fields: Vec<Vec<u8>>,
    /// 1-based line number in the stream.
    pub line_no: u64,
    /// Byte range of the raw line in the stream (excluding terminator).
    pub byte_start: u64,
    pub byte_end: u64,
}

/// Incremental delimited-record reader. Feed arbitrary chunks; finish at
/// end-of-input. All parser state the language owns crosses each call
/// (carry bytes); the host only stores what MNCS returned.
#[derive(Debug, Default)]
pub struct CsvReader {
    carry: Vec<u8>,
    line_no: u64,
    stream_pos: u64,
}

impl CsvReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one chunk (any size; internally windowed to [`VIEW_BYTES`]).
    /// Returns the records completed by this chunk, in order.
    pub fn feed(&mut self, rt: &LanguageRuntime, chunk: &[u8]) -> Result<Vec<CsvRecord>, IngestError> {
        let mut records = Vec::new();
        for view in chunk.chunks(VIEW_BYTES) {
            let carry_before = self.carry.len() as u64;
            let out = rt.next_line(&self.carry, view, false)?;
            self.carry = out.carry.clone();
            self.stream_pos += out.consumed as u64;
            // The produced line is the oldest staged content: it starts
            // where the pre-call carry started.
            let line_start = self.stream_pos
                .saturating_sub(out.consumed as u64)
                .saturating_sub(carry_before);
            self.drain_completed(rt, out.status, &out.line, line_start, &mut records)?;
            self.drain_carry(rt, &mut records)?;
        }
        Ok(records)
    }

    /// End of input: flush the final tail (or complete cleanly).
    pub fn finish(&mut self, rt: &LanguageRuntime) -> Result<Vec<CsvRecord>, IngestError> {
        let mut records = Vec::new();
        loop {
            let carry_before = self.carry.len() as u64;
            let out = rt.next_line(&self.carry, &[], true)?;
            self.carry = out.carry.clone();
            self.stream_pos += out.consumed as u64;
            let line_start = self.stream_pos
                .saturating_sub(out.consumed as u64)
                .saturating_sub(carry_before);
            match out.status {
                0 => {
                    records.push(self.parse_line(rt, &out.line, line_start)?);
                }
                2 => break,
                other => return Err(line_status(other, self.line_no + 1)),
            }
        }
        Ok(records)
    }

    fn drain_completed(
        &mut self,
        rt: &LanguageRuntime,
        status: i64,
        line: &[u8],
        line_start: u64,
        records: &mut Vec<CsvRecord>,
    ) -> Result<(), IngestError> {
        match status {
            0 => {
                records.push(self.parse_line(rt, line, line_start)?);
                Ok(())
            }
            1 => Ok(()),
            other => Err(line_status(other, self.line_no + 1)),
        }
    }

    /// A produced line may leave a carry holding further full lines;
    /// re-enter with an empty view until the carry stops producing.
    fn drain_carry(
        &mut self,
        rt: &LanguageRuntime,
        records: &mut Vec<CsvRecord>,
    ) -> Result<(), IngestError> {
        loop {
            if self.carry.is_empty() {
                return Ok(());
            }
            let carry_before = self.carry.len() as u64;
            let out = rt.next_line(&self.carry, &[], false)?;
            self.carry = out.carry.clone();
            self.stream_pos += out.consumed as u64;
            let line_start = self.stream_pos
                .saturating_sub(out.consumed as u64)
                .saturating_sub(carry_before);
            match out.status {
                0 => records.push(self.parse_line(rt, &out.line, line_start)?),
                1 => return Ok(()),
                other => return Err(line_status(other, self.line_no + 1)),
            }
        }
    }

    fn parse_line(
        &mut self,
        rt: &LanguageRuntime,
        line: &[u8],
        line_start: u64,
    ) -> Result<CsvRecord, IngestError> {
        let fields = parse_fields(rt, line, self.line_no + 1)?;
        self.line_no += 1;
        Ok(CsvRecord {
            fields,
            line_no: self.line_no,
            byte_start: line_start,
            byte_end: line_start + line.len() as u64,
        })
    }
}

/// Split one raw line into unescaped fields. The 8-field bound is
/// enforced here against `has_more` so the schema bound stays visible.
pub fn parse_fields(
    rt: &LanguageRuntime,
    line: &[u8],
    line_no: u64,
) -> Result<Vec<Vec<u8>>, IngestError> {
    let mut fields = Vec::new();
    let mut start = 0u64;
    loop {
        let field = rt.field_at(line, start)?;
        match field.status {
            0 => {
                fields.push(field.value);
                if fields.len() > MAX_FIELDS {
                    return Err(IngestError::malformed(
                        "csv",
                        format!("line {line_no}: more than {MAX_FIELDS} fields"),
                    ));
                }
                if field.has_more {
                    start = field.next_start;
                } else {
                    return Ok(fields);
                }
            }
            1 => return Ok(fields),
            2 => {
                return Err(IngestError::malformed(
                    "csv",
                    format!("line {line_no}: malformed field escape or control byte"),
                ));
            }
            3 => {
                return Err(IngestError::Overlong {
                    adapter: "csv".to_owned(),
                    detail: format!("line {line_no}: field exceeds {MAX_FIELD_BYTES} bytes"),
                });
            }
            other => {
                return Err(IngestError::Language(format!(
                    "unknown field status {other}"
                )));
            }
        }
    }
}

fn line_status(status: i64, line_no: u64) -> IngestError {
    match status {
        3 => IngestError::malformed("csv", format!("line {line_no}: malformed bytes")),
        4 => IngestError::Overlong {
            adapter: "csv".to_owned(),
            detail: format!("line {line_no}: line exceeds {MAX_LINE_BYTES} bytes"),
        },
        other => IngestError::Language(format!("unknown line status {other}")),
    }
}
