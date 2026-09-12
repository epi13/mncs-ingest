//! Streaming binary-frame ingestion (Tranche B).
//!
//! Wire format (telemetry frame v1, little-endian; see `docs/STREAMING.md`):
//!
//! ```text
//! offset  size  field
//! 0       2     magic 0x4D43 ("MC")
//! 2       1     version (1 supported)
//! 3       1     kind tag (0 sensor, 1 event, 2 heartbeat)
//! 4       2     payload length (0..=128)
//! 6       N     payload bytes
//! 6+N     2     checksum: sum of all preceding frame bytes mod 65536
//! ```
//!
//! Like the text path, the host windows the stream into ≤256-byte views
//! and holds the carry; every framing, endian, range, version, tag, and
//! checksum decision executes in `mncs.flow.frames`. Incomplete input
//! mid-stream asks for more; the same bytes at end-of-input are
//! truncated — the two are never confused.

use crate::error::IngestError;
use crate::language::LanguageRuntime;

pub const VIEW_BYTES: usize = 256;
pub const MAX_PAYLOAD_BYTES: usize = 128;

/// One decoded frame with its source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub version: u64,
    pub kind: u64,
    pub payload: Vec<u8>,
    /// Byte range of the whole frame (header through checksum).
    pub byte_start: u64,
    pub byte_end: u64,
}

/// Incremental frame reader over an arbitrary chunk stream.
#[derive(Debug, Default)]
pub struct FrameReader {
    carry: Vec<u8>,
    stream_pos: u64,
}

impl FrameReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one chunk (any size); returns frames completed, in order.
    pub fn feed(&mut self, rt: &LanguageRuntime, chunk: &[u8]) -> Result<Vec<Frame>, IngestError> {
        let mut frames = Vec::new();
        for view in chunk.chunks(VIEW_BYTES) {
            let carry_before = self.carry.len() as u64;
            let out = rt.decode_frame(&self.carry, view, false)?;
            self.carry = out.carry.clone();
            self.stream_pos += out.consumed as u64;
            let frame_start = self
                .stream_pos
                .saturating_sub(out.consumed as u64)
                .saturating_sub(carry_before);
            self.drain_status(&out, frame_start, &mut frames)?;
            self.drain_carry(rt, &mut frames)?;
        }
        Ok(frames)
    }

    /// End of input: complete cleanly or report truncation.
    pub fn finish(&mut self, rt: &LanguageRuntime) -> Result<Vec<Frame>, IngestError> {
        let mut frames = Vec::new();
        loop {
            let carry_before = self.carry.len() as u64;
            let out = rt.decode_frame(&self.carry, &[], true)?;
            self.carry = out.carry.clone();
            self.stream_pos += out.consumed as u64;
            let frame_start = self
                .stream_pos
                .saturating_sub(out.consumed as u64)
                .saturating_sub(carry_before);
            match out.status {
                0 => frames.push(Self::to_frame(&out, frame_start)),
                2 => break,
                other => return Err(frame_status(other)),
            }
        }
        Ok(frames)
    }

    fn drain_status(
        &mut self,
        out: &crate::language::FrameOut,
        frame_start: u64,
        frames: &mut Vec<Frame>,
    ) -> Result<(), IngestError> {
        match out.status {
            0 => {
                frames.push(Self::to_frame(out, frame_start));
                Ok(())
            }
            1 => Ok(()),
            other => Err(frame_status(other)),
        }
    }

    fn drain_carry(
        &mut self,
        rt: &LanguageRuntime,
        frames: &mut Vec<Frame>,
    ) -> Result<(), IngestError> {
        loop {
            if self.carry.is_empty() {
                return Ok(());
            }
            let carry_before = self.carry.len() as u64;
            let out = rt.decode_frame(&self.carry, &[], false)?;
            self.carry = out.carry.clone();
            self.stream_pos += out.consumed as u64;
            let frame_start = self
                .stream_pos
                .saturating_sub(out.consumed as u64)
                .saturating_sub(carry_before);
            match out.status {
                0 => frames.push(Self::to_frame(&out, frame_start)),
                1 => return Ok(()),
                other => return Err(frame_status(other)),
            }
        }
    }

    fn to_frame(out: &crate::language::FrameOut, frame_start: u64) -> Frame {
        Frame {
            version: out.version,
            kind: out.kind,
            payload: out.payload.clone(),
            byte_start: frame_start,
            byte_end: frame_start + out.frame_len as u64,
        }
    }
}

fn frame_status(status: i64) -> IngestError {
    match status {
        3 => IngestError::malformed("frame", "bad magic or header"),
        4 => IngestError::malformed("frame", "checksum mismatch"),
        5 => IngestError::malformed("frame", "truncated frame at end of input"),
        6 => IngestError::unsupported("frame", "unsupported frame version"),
        7 => IngestError::unsupported("frame", "unknown frame kind tag"),
        other => IngestError::Language(format!("unknown frame status {other}")),
    }
}

/// Test/dev helper: encode one frame (verification infrastructure; the
/// decoder stays MNCS-owned).
pub fn encode_frame(version: u8, kind: u8, payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() <= MAX_PAYLOAD_BYTES);
    let mut frame = vec![0x43, 0x4D, version, kind];
    frame.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    frame.extend_from_slice(payload);
    let sum: u32 = frame.iter().map(|byte| *byte as u32).sum();
    frame.extend_from_slice(&((sum % 65536) as u16).to_le_bytes());
    frame
}
