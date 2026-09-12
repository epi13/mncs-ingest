# Streaming ingestion (Tranches A, B, F)

Two framing families share one resumable pattern: the host windows a
byte stream into ≤256-byte views and holds the carry; every boundary,
escape, validation, and cursor decision executes in MNCS. The host
never splits lines, fields, or frames itself.

## CSV-lite records (`mncs.flow.lines`, `mncs.flow.fields`, `src/csv.rs`)

- Records are `\n`-terminated lines; a trailing `\r` is stripped, a
  mid-line `\r` is malformed. Only `\t` is tolerated among control
  bytes below `0x20`. A dangling `\r` at end-of-input is a truncated
  CRLF and is malformed, not a clean tail.
- Fields split on `,`; at most 8 fields per record, each at most 63
  unescaped bytes. Lines are at most 255 raw bytes.
- `\` escapes any byte except `\n`/`\r` (escaping a terminator is
  malformed — terminators always terminate, so layering stays
  lines-then-fields with no feedback). A trailing lone `\` (including
  at end-of-input) is malformed, not truncation.
- An empty source yields zero records; a lone `\n` yields one
  zero-field record; an unterminated final tail still yields its record
  (CSV convention — truncation applies to escapes/frames, not to a
  missing final newline).
- `next_line` returns at most one line per call plus carry; the host
  drains the carry with empty views. `consumed` counts view bytes, and
  the view cursor rewinds when a post-newline tail exceeds carry
  capacity (no byte is ever dropped or double-counted).
- Record positions (`line_no`, `byte_start/end`) derive from the
  invariant: a produced unit starts where the pre-call carry started.

## Binary frames (`mncs.flow.frames`, `src/frames.rs`)

Telemetry frame v1, little-endian: magic `0x4D43`, version (only 1),
kind tag (0 sensor, 1 event, 2 heartbeat), `u16` payload length
(0..=128), payload, `u16` checksum (byte sum mod 65536).

- A short header or short body mid-stream is `need_more`; the same
  bytes at end-of-input are `truncated` (status 5). Incomplete and
  malformed are never confused: bad magic, oversize length,
  checksum mismatch, unknown version/tag each have their own status.
- Carry capacity is 136 bytes (a full max frame); staging is
  carry ++ view (≤ 392 bytes).

## Manifests (Tranche C: `mncs.flow.nest`, `mncs.flow.manifest`, `src/manifest.rs`)

Bounded JSON subset: top-level objects with string keys and string /
integer / boolean / nested values, depth ≤ 4, documents ≤ 512 bytes.
Two phases: the `nest` scanner validates structure incrementally over
chunk views (bounded decomposed state crosses each call — records
cannot, INGEST-P-009); extraction reads typed pairs from the staged
bytes of a complete document, one pair per `pair_at` call.

- Expect states: 0 need-value, 1 fresh container (object wants a key,
  array wants a value — the innermost stack entry disambiguates), 2
  need-colon, 3 need-comma-or-close, 4 complete (exactly one root).
- A comma ending a skipped (unwanted) value advances pair/phase exactly
  as a phase-5 comma; closers keep phase 5 for the matcher.
- Integers are ±99999 with at most 5 digits; keys ≤ 32 bytes, string
  values ≤ 64 unescaped bytes — oversize is `Overlong`, never silent
  truncation. `\u` escapes are rejected by design (INGEST-P-003).
- Nested values stay opaque `(start, len)` spans; the host re-stages a
  span as its own document (recursion via transport).

## Resumable outcomes (Tranche F)

Both families already implement the state machine: each MNCS step
returns `produced / need_more / complete / malformed(+detail)`, state
is bounded owned data (carry buffers + lengths) held by the host
between calls, and re-feeding is deterministic. Tranche F formalizes
these as finite types (Tranche E) instead of `i64` codes.

## Throughput (Tranche G)

Per-call reference-execution overhead dominates in debug builds; the
retained `mncs-embed` session path (`src/embed.rs`) executes identical
verdicts (parity-tested in `tests/embed_parity.rs`) at a fraction of
the cost. Benchmarks live in the stream test suites and report
records/sec, bytes/sec, calls, and MNCS step counts.
