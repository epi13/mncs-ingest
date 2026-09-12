//! Tranche A: streaming delimited-record ingestion.
//!
//! Golden records, adversarial chunkings (every boundary split),
//! escape handling, bound proving (63/64, 255/256, 8/9), malformed
//! battery, truncation, CRLF seams, incremental==one-shot equivalence,
//! a seeded roundtrip fuzz, and a throughput benchmark.

mod common;

use common::rt;
use mncs_ingest::csv::{parse_fields, CsvReader};
use mncs_ingest::language::LanguageRuntime;
use mncs_ingest::IngestError;

fn read_all(rt: &LanguageRuntime, input: &[u8], chunk: usize) -> Vec<mncs_ingest::csv::CsvRecord> {
    let mut reader = CsvReader::new();
    let mut records = Vec::new();
    if chunk == 0 {
        records.extend(reader.feed(rt, input).unwrap());
    } else {
        for piece in input.chunks(chunk) {
            records.extend(reader.feed(rt, piece).unwrap());
        }
    }
    records.extend(reader.finish(rt).unwrap());
    records
}

fn fields_of(records: &[mncs_ingest::csv::CsvRecord]) -> Vec<Vec<String>> {
    records
        .iter()
        .map(|record| {
            record
                .fields
                .iter()
                .map(|field| String::from_utf8_lossy(field).into_owned())
                .collect()
        })
        .collect()
}

fn read_err(rt: &LanguageRuntime, input: &[u8], chunk: usize) -> IngestError {
    let mut reader = CsvReader::new();
    for piece in input.chunks(chunk.max(1)) {
        if let Err(error) = reader.feed(rt, piece) {
            return error;
        }
    }
    reader.finish(rt).expect_err("input must fail")
}

#[test]
fn golden_records_parse() {
    let rt = rt();
    let input = b"name,age,city\nAlexander,31,Oslo\nFinn,29,Bergen\n";
    let records = read_all(rt, input, 0);
    assert_eq!(
        fields_of(&records),
        [
            ["name", "age", "city"],
            ["Alexander", "31", "Oslo"],
            ["Finn", "29", "Bergen"],
        ]
    );
    assert_eq!(records[0].line_no, 1);
    assert_eq!(records[2].line_no, 3);
    assert_eq!(records[1].byte_start, 14);
}

#[test]
fn chunk_boundaries_do_not_change_semantics() {
    let rt = rt();
    let input = b"a,b,c\none,two,three\nx\\,y,z\\\\\nlast,line,here";
    let baseline = fields_of(&read_all(rt, input, 0));
    for chunk in [1usize, 2, 3, 5, 7, 8, 13, 63, 64, 65, 100, 255, 256, 300] {
        assert_eq!(
            fields_of(&read_all(rt, input, chunk)),
            baseline,
            "chunk size {chunk} must agree with one-shot"
        );
    }
    assert_eq!(
        baseline[2],
        ["x,y".to_owned(), "z\\".to_owned()],
        "escapes resolve in values"
    );
    assert_eq!(baseline[3], ["last", "line", "here"], "unterminated tail yields");
}

#[test]
fn crlf_seams_survive_splitting() {
    let rt = rt();
    let input = b"a,b\r\nc,d\r\ne,f";
    let baseline = fields_of(&read_all(rt, input, 0));
    assert_eq!(baseline, [["a", "b"], ["c", "d"], ["e", "f"]]);
    // Split between \r and \n at every seam.
    for split in [4usize, 5, 9, 10] {
        let mut reader = CsvReader::new();
        let mut records = Vec::new();
        records.extend(reader.feed(rt, &input[..split]).unwrap());
        records.extend(reader.feed(rt, &input[split..]).unwrap());
        records.extend(reader.finish(rt).unwrap());
        assert_eq!(fields_of(&records), baseline, "split at {split}");
    }
}

#[test]
fn empty_fields_and_lines_behave() {
    let rt = rt();
    let records = read_all(rt, b"a,,c\n,,\n", 0);
    assert_eq!(fields_of(&records), [["a", "", "c"], ["", "", ""]]);
    // Empty input is a valid empty source: zero records, no error.
    let records = read_all(rt, b"", 0);
    assert!(records.is_empty());
    // A lone newline is one empty (zero-field) line.
    let records = read_all(rt, b"\n", 0);
    assert_eq!(records.len(), 1);
    assert!(records[0].fields.is_empty());
}

#[test]
fn field_and_line_bounds_hold() {
    let rt = rt();
    // 63-byte field ingests; 64-byte field is overlong.
    let ok_line = format!("{},{}\n", "a".repeat(63), "b");
    assert_eq!(read_all(rt, ok_line.as_bytes(), 0).len(), 1);
    let big_line = format!("{},{}\n", "a".repeat(64), "b");
    assert!(matches!(
        read_err(rt, big_line.as_bytes(), 7),
        IngestError::Overlong { .. }
    ));
    // 255-byte line ingests (four max-size 63-byte fields: the line
    // bound and the field bound hold conjunctively, so a 255-byte
    // single field would fail the field bound instead); 256-byte line
    // is overlong.
    let ok_line = format!(
        "{},{},{},{}\n",
        "c".repeat(63),
        "d".repeat(63),
        "e".repeat(63),
        "f".repeat(63)
    );
    assert_eq!(ok_line.len() - 1, 255, "raw line is 255 bytes");
    assert_eq!(read_all(rt, ok_line.as_bytes(), 0).len(), 1);
    let big_line = format!("{}\n", "c".repeat(256));
    assert!(matches!(
        read_err(rt, big_line.as_bytes(), 64),
        IngestError::Overlong { .. }
    ));
    // 8 fields ingest; the 9th fails.
    assert_eq!(read_all(rt, b"1,2,3,4,5,6,7,8\n", 0).len(), 1);
    assert!(matches!(
        read_err(rt, b"1,2,3,4,5,6,7,8,9\n", 3),
        IngestError::Malformed { .. }
    ));
}

#[test]
fn malformed_battery_fails_typed() {
    let rt = rt();
    // Trailing lone backslash.
    assert!(matches!(
        read_err(rt, b"a,b\\\n", 2),
        IngestError::Malformed { .. }
    ));
    // Escape at EOF is malformed, not truncated.
    assert!(matches!(
        read_err(rt, b"a,b\\", 0),
        IngestError::Malformed { .. }
    ));
    // Escaped newline is malformed (terminators always terminate).
    assert!(matches!(
        read_err(rt, b"a\\\n,b\n", 0),
        IngestError::Malformed { .. }
    ));
    // Control bytes rejected.
    assert!(matches!(
        read_err(rt, b"a,\x01\n", 0),
        IngestError::Malformed { .. }
    ));
    // Lone CR mid-line rejected.
    assert!(matches!(
        read_err(rt, b"a\rb,c\n", 0),
        IngestError::Malformed { .. }
    ));
    // Dangling CR at EOF is a truncated CRLF, not a clean tail.
    assert!(matches!(
        read_err(rt, b"a,b\r", 0),
        IngestError::Malformed { .. }
    ));
    // Tab is the one tolerated control.
    assert_eq!(read_all(rt, b"a,\tb\n", 0).len(), 1);
}

#[test]
fn seeded_roundtrip_fuzz() {
    // Deterministic LCG generates valid records (incl. escapes);
    // encode→parse roundtrips and chunking never matters.
    let rt = rt();
    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut next = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 33) as usize
    };
    let alphabet = b"abZ09 -,._";
    for case in 0..150 {
        let nfields = 1 + next() % 5;
        let mut record: Vec<Vec<u8>> = Vec::new();
        for _ in 0..nfields {
            let nbytes = next() % 20;
            let mut field: Vec<u8> = Vec::new();
            for _ in 0..nbytes {
                let byte = alphabet[next() % alphabet.len()];
                if byte == b',' || byte == b'\\' {
                    field.push(b'\\');
                }
                field.push(byte);
            }
            record.push(field);
        }
        let mut encoded = Vec::new();
        for (index, field) in record.iter().enumerate() {
            if index > 0 {
                encoded.push(b',');
            }
            encoded.extend_from_slice(field);
        }
        encoded.push(b'\n');
        let chunk = 1 + next() % 40;
        let parsed = read_all(rt, &encoded, chunk);
        assert_eq!(parsed.len(), 1, "case {case}: one record");
        let expect: Vec<Vec<u8>> = record
            .iter()
            .map(|field| {
                let mut out = Vec::new();
                let mut bytes = field.iter();
                while let Some(byte) = bytes.next() {
                    if *byte == b'\\' {
                        out.push(*bytes.next().unwrap());
                    } else {
                        out.push(*byte);
                    }
                }
                out
            })
            .collect();
        assert_eq!(parsed[0].fields, expect, "case {case}: roundtrip");
    }
}

#[test]
fn fields_exposed_directly() {
    let rt = rt();
    let fields = parse_fields(rt, b"a\\,b,c\\\\", 1).unwrap();
    assert_eq!(fields, [b"a,b".to_vec(), b"c\\".to_vec()]);
}

#[test]
fn throughput_benchmark_reports() {
    let rt = rt();
    // 2000 telemetry-ish records through 64-byte views.
    let mut input = Vec::new();
    for index in 0..2000u32 {
        input.extend_from_slice(format!("sensor-{index},ok,{}\n", index * 7).as_bytes());
    }
    let total_bytes = input.len();
    let start = std::time::Instant::now();
    let records = read_all(rt, &input, 64);
    let elapsed = start.elapsed();
    assert_eq!(records.len(), 2000);
    let secs = elapsed.as_secs_f64();
    eprintln!(
        "csv-stream: {} records, {} bytes in {:.2}s ({:.0} rec/s, {:.0} B/s)",
        records.len(),
        total_bytes,
        secs,
        records.len() as f64 / secs,
        total_bytes as f64 / secs
    );
}
