//! Tranche B: streaming binary-frame ingestion.
//!
//! Golden vectors, every split point, endian/range behavior, checksum
//! and version/tag rejection, incomplete-vs-malformed separation,
//! bound proving (128/129), fuzz roundtrip, and throughput.

mod common;

use common::rt;
use mncs_ingest::frames::{encode_frame, Frame, FrameReader};
use mncs_ingest::language::LanguageRuntime;
use mncs_ingest::IngestError;

fn read_all(rt: &LanguageRuntime, input: &[u8], chunk: usize) -> Vec<Frame> {
    let mut reader = FrameReader::new();
    let mut frames = Vec::new();
    if chunk == 0 {
        frames.extend(reader.feed(rt, input).unwrap());
    } else {
        for piece in input.chunks(chunk) {
            frames.extend(reader.feed(rt, piece).unwrap());
        }
    }
    frames.extend(reader.finish(rt).unwrap());
    frames
}

fn read_err(rt: &LanguageRuntime, input: &[u8], chunk: usize) -> IngestError {
    let mut reader = FrameReader::new();
    for piece in input.chunks(chunk.max(1)) {
        if let Err(error) = reader.feed(rt, piece) {
            return error;
        }
    }
    reader.finish(rt).expect_err("input must fail")
}

fn stream(frames: &[Vec<u8>]) -> Vec<u8> {
    frames.concat()
}

#[test]
fn golden_frames_decode() {
    let rt = rt();
    let input = stream(&[
        encode_frame(1, 0, b"sensor-1"),
        encode_frame(1, 1, b""),
        encode_frame(1, 2, &[0xde, 0xad, 0xbe, 0xef]),
    ]);
    let frames = read_all(rt, &input, 0);
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].version, 1);
    assert_eq!(frames[0].kind, 0);
    assert_eq!(frames[0].payload, b"sensor-1");
    assert_eq!(frames[1].payload, b"");
    assert_eq!(frames[2].payload, [0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(frames[0].byte_start, 0);
    assert_eq!(frames[0].byte_end, 8 + 8);
    assert_eq!(frames[1].byte_start, frames[0].byte_end);
    assert_eq!(frames[2].byte_start, frames[1].byte_end);
    assert_eq!(frames[2].byte_end, input.len() as u64);
}

#[test]
fn every_split_point_agrees() {
    let rt = rt();
    let input = stream(&[
        encode_frame(1, 0, b"abcdefgh"),
        encode_frame(1, 1, b"xyz"),
        encode_frame(1, 2, b"0123456789"),
    ]);
    let baseline = read_all(rt, &input, 0);
    assert_eq!(baseline.len(), 3);
    for split in 1..input.len() {
        let mut reader = FrameReader::new();
        let mut frames = Vec::new();
        frames.extend(reader.feed(rt, &input[..split]).unwrap());
        frames.extend(reader.feed(rt, &input[split..]).unwrap());
        frames.extend(reader.finish(rt).unwrap());
        assert_eq!(frames, baseline, "split at {split}");
    }
}

#[test]
fn incomplete_is_not_malformed() {
    let rt = rt();
    let full = encode_frame(1, 0, b"payload-bytes");
    // Mid-stream prefix asks for more (no error); the same bytes at
    // end-of-input are truncated.
    for cut in [1usize, 3, 5, 6, 7, 10, full.len() - 1] {
        let mut reader = FrameReader::new();
        let frames = reader.feed(rt, &full[..cut]).unwrap();
        assert!(frames.is_empty(), "cut {cut}: no frame yet, no error");
        let rest = reader.feed(rt, &full[cut..]).unwrap();
        let tail = reader.finish(rt).unwrap();
        assert_eq!(rest.len() + tail.len(), 1, "cut {cut}: frame completes");

        let error = read_err(rt, &full[..cut], 0);
        assert!(
            matches!(error, IngestError::Malformed { .. }),
            "cut {cut}: eof prefix is truncated: {error:?}"
        );
    }
    // Empty input completes cleanly with zero frames.
    let mut reader = FrameReader::new();
    assert!(reader.finish(rt).unwrap().is_empty());
}

#[test]
fn checksum_version_tag_rejected() {
    let rt = rt();
    let mut bad_sum = encode_frame(1, 0, b"data");
    let last = bad_sum.len() - 1;
    bad_sum[last] ^= 0xff;
    assert!(matches!(
        read_err(rt, &bad_sum, 0),
        IngestError::Malformed { .. }
    ));
    let bad_magic = {
        let mut frame = encode_frame(1, 0, b"data");
        frame[0] ^= 0xff;
        frame
    };
    assert!(matches!(
        read_err(rt, &bad_magic, 0),
        IngestError::Malformed { .. }
    ));
    assert!(matches!(
        read_err(rt, &encode_frame(2, 0, b"data"), 0),
        IngestError::Unsupported { .. }
    ));
    assert!(matches!(
        read_err(rt, &encode_frame(1, 9, b"data"), 0),
        IngestError::Unsupported { .. }
    ));
}

#[test]
fn payload_bounds_hold() {
    let rt = rt();
    let maxed = encode_frame(1, 0, &vec![0x55; 128]);
    assert_eq!(read_all(rt, &maxed, 0).len(), 1);
    // A declared length of 129 exceeds the bound: malformed, and the
    // length field itself is little-endian (129 = 0x81 0x00).
    let mut evil = vec![0x43, 0x4D, 1, 0, 0x81, 0x00];
    evil.extend_from_slice(&[0x41; 129]);
    let sum: u32 = evil.iter().map(|byte| *byte as u32).sum();
    evil.extend_from_slice(&((sum % 65536) as u16).to_le_bytes());
    assert!(matches!(
        read_err(rt, &evil, 0),
        IngestError::Malformed { .. }
    ));
}

#[test]
fn seeded_frame_fuzz_roundtrip() {
    let rt = rt();
    let mut state: u64 = 0x0bad_f00d_dead_beef;
    let mut next = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 33) as usize
    };
    for case in 0..80 {
        let count = 1 + next() % 4;
        let mut parts = Vec::new();
        let mut expect = Vec::new();
        for _ in 0..count {
            let len = next() % 40;
            let payload: Vec<u8> = (0..len).map(|_| (next() % 256) as u8).collect();
            let kind = (next() % 3) as u8;
            parts.push(encode_frame(1, kind, &payload));
            expect.push((kind, payload));
        }
        let input = stream(&parts);
        let chunk = 1 + next() % 37;
        let frames = read_all(rt, &input, chunk);
        assert_eq!(frames.len(), expect.len(), "case {case}");
        for (frame, (kind, payload)) in frames.iter().zip(&expect) {
            assert_eq!(frame.kind as u8, *kind);
            assert_eq!(&frame.payload, payload);
        }
    }
}

#[test]
fn frame_throughput_benchmark_reports() {
    let rt = rt();
    let mut input = Vec::new();
    for index in 0..1500u32 {
        let mut payload = Vec::new();
        payload.extend_from_slice(&(index as u16).to_le_bytes());
        payload.extend_from_slice(&index.to_le_bytes());
        payload.extend_from_slice(&(index as i32 * 7).to_le_bytes());
        input.extend_from_slice(&encode_frame(1, 0, &payload));
    }
    let total_bytes = input.len();
    let start = std::time::Instant::now();
    let frames = read_all(rt, &input, 64);
    let elapsed = start.elapsed();
    assert_eq!(frames.len(), 1500);
    let secs = elapsed.as_secs_f64();
    eprintln!(
        "frame-stream: {} frames, {} bytes in {:.2}s ({:.0} frames/s, {:.0} B/s)",
        frames.len(),
        total_bytes,
        secs,
        frames.len() as f64 / secs,
        total_bytes as f64 / secs
    );
}
