# MNCS Ingest Language-Pressure Ledger

Prior run (transfer convergence slice, pin `d7cc953`) recorded 11
pressures in narrative form. This run reconciles each against the
current language (pin `f7c1ba3`, Source Profile 0.14) and carries the
survivors forward as stable `INGEST-P-###` artifacts in the format a
later `mncs-language` agent can execute directly.

## Reconciliation of prior-run pressures

| Prior ID | New ID | Verdict | Note |
|---|---|---|---|
| P-STRING-01 (no strings) | INGEST-P-001 | still reproduces | No text value at 0.14 |
| P-STRING-02 (no slicing) | INGEST-P-002 | partially relieved | 0.14 adds `copy_span`, exact→view `[0..n]`, narrowing; view`[a..b]` still missing |
| P-STRING-03 (no UTF-8 scalars) | INGEST-P-003 | still reproduces | Only producer-attested flags in stdlib |
| P-COLL-01 (no maps) | INGEST-P-004 | partially relieved | `text_map` table lookup exists in stdlib; adopted via vendoring |
| P-FOLD-01 (dead-lane writes) | INGEST-P-005 | still reproduces | No liveness guards at 0.14 |
| P-SELECT-01 (strict select) | INGEST-P-006 | partially relieved | `checked_index` discharge at 0.14; lazy conditionals still absent |
| P-DIAG-01 (hidden inner diagnostics) | INGEST-P-007 | still reproduces | Re-probed 2026-09-11: identical behavior |
| P-PROFILE-01 (u64 domains) | — | language resolved | Closed; 0.13+ admits u64 domains |
| P-MOD-01 (no stdlib distribution) | INGEST-P-008 | still reproduces | Vendoring is the workaround that proves it |
| P-ABI-01 (windows+counts) | INGEST-P-009 | still reproduces | Unchanged at 0.14 |
| P-PERF-01 (per-call cost) | INGEST-P-010 | partially relieved | `mncs-embed` retained sessions exist; ingest has not adopted them yet (adoption debt) |

New pressures from this run continue at INGEST-P-011+.

Vendored standard-library sources live under `language/vendor/` (pinned
at the workspace pin; see `language/vendor/VENDORING.md`). Vendoring is
the documented workaround for INGEST-P-008, not a fork: vendored files
are byte-identical to upstream at the pin.

---

## INGEST-P-001 — No string/text value: open-vocabulary identity cannot round-trip

## Problem
The ingest pipeline needs bounded validated UTF-8/text slices with
delimiter search and source-span preservation. Entity spellings and
unknown-object spellings must cross MNCS with byte-exact identity, and
canonical labels must render from codes, without host-side tables
drifting from MNCS tables.

## Reproducer
`language/mncs/ingest/parse.mncs::parse_transfer` returns
`source_start/source_length` integer spans instead of the spelling, and
`src/canonical.rs::object_label` mirrors `vocab.mncs` in Rust:

```mncs
// There is no expression of "return the matched word": the classifier
// can only return THAT it matched (a code), never WHAT matched.
fn classify_object(text: [byte; up_to 64], start: u64, length: u64) -> (result: i64)
```

Executable: `tests/adapters.rs::vocabulary_codes_match_host_rendering`
pins both sides of the duplication.

## Expected semantics
A bounded text value (explicit max bytes, UTF-8 validated at the
boundary) that MNCS can construct, return, compare, and project back to
bytes — no heap allocation, no unbounded string type.

## Current result
Status quo at 0.14: bytes and fixed arrays only. Equality is bifurcated
by force (`ingest_codes_equal` + `ingest_spell_equal`, host ANDs the
verdicts).

## Architectural impact
Host slices spans out of the original input; a two-entry render table
duplicates MNCS vocabulary. A 200-word vocabulary would be a drift
hazard. (Tranche A carries this further: unescaped field VALUES must
materialize as fixed `[byte; 64]` buffers plus lengths.)

## Severity
Major.

## Owning layer
Type system / value model; standard library (validated-text wrapper).

## Acceptance test
`ingest_parse` returns the source spelling as a value (not a span);
`src/canonical.rs::object_label` is deleted with no replacement; the
vocabulary contract test still passes.

---

## INGEST-P-002 — No arbitrary view sub-slicing (partially relieved by 0.14)

## Problem
A streaming decoder needs a bounded zero-copy cursor that advances
through a leased byte region while preserving provenance and bounds:
carve word/field/line spans out of a sentence/chunk view and hand the
span itself to shared classifiers.

## Reproducer
Pre-0.14 `vocab.mncs` takes `(text, start, length)` triples with manual
clamping because `[byte; up_to 64]` cannot be sub-viewed
(`span_byte`, `span_eq`, `span_prefix_eq` prologues ≈15% of the file).

## Expected semantics
Bounded borrow-slices over views with compiler-checked bounds, so a
callee receives a view it cannot index out of.

## Current result
Partially relieved at 0.14: `copy_span(dst, dst_at, src, src_at, len)`
stages spans into exact buffers, exact arrays slice to views
(`buf[0..n]`), and views narrow (`[E; up_to A]` → `[E; up_to B]`).
Still missing: `view[a..b]` sub-slicing of a view directly. Tranche A
proves the composition (`stage` + `copy_span` + narrow) and records
where it falls short.

## Architectural impact
Span-triple convention retained in `vocab.mncs`; new streaming code
stages through exact buffers with extra copies that a sub-slice would
avoid (measured in Tranche A benchmarks as copies/byte).

## Severity
Major (was blocker-adjacent; 0.14 reduced it).

## Owning layer
Borrow/view model; compiler (bounds discharge for dynamic ranges).

## Acceptance test
`vocab.mncs` classifiers take one view argument each; the
`(text, start, length)` prologues disappear with no added copies.

---

## INGEST-P-003 — No UTF-8 scalar semantics in MNCS

## Problem
Unicode entity names (`Zoë`) must survive ingestion with exact identity,
and eventually the pipeline needs to validate (and later fold) text
itself rather than trusting the host's pre-validation.

## Reproducer
`vocab.mncs::is_entity_word` accepts bytes `>= 128` opaquely; the host
validates UTF-8 first (`tests/adapters.rs::
unicode_entities_pass_through_all_adapters`). Stdlib deals only in
producer-attested `utf8_valid` flags (`text_view`), never validation.

## Expected semantics
Producer-attested UTF-8 views plus a minimal scalar-step primitive
(first-scalar/rest), so classifiers can walk characters without owning
decoding tables.

## Current result
Still reproduces at 0.14: no validator, no scalar step, no
non-ASCII fold in stdlib or intrinsics.

## Architectural impact
Host validates; MNCS passes high bytes through. Any future
normalization (case-insensitive entity convergence, NFC) must live on
the host, outside semantic authority.

## Severity
Moderate.

## Owning layer
Standard library (validator); type system (validated-text marker).

## Acceptance test
An MNCS function rejects invalid UTF-8 (`[0xC3, 0x28]`) and folds one
non-ASCII case pair, executed on all backends.

---

## INGEST-P-004 — No finite maps; keyword tables are linear chains (partially relieved)

## Problem
Classify ~25 keywords (verbs, number words, objects) — and, in
Tranche A, record keys and delimiters — through one deterministic
table instead of hand-written if-chains.

## Reproducer
Pre-relief `classify_word_number`: eleven sequential `span_eq` calls
(≈700 byte comparisons per word worst case).

## Expected semantics
A bounded, deterministic finite map (const keys, total lookup with an
explicit miss) with identical values on every backend.

## Current result
Partially relieved: `mncs.std.text_map.v1` provides exactly this
(`lookup16/lookup32` over caller-owned tables, last-entry-wins). It is
adopted via vendoring (`language/vendor/`); keys must still be staged
into exact `[byte; 16]` buffers first (needs INGEST-P-002 fully solved
to be zero-copy).

## Architectural impact
Linear chains retained in the transfer slice (stable, tested);
`text_map` adopted for new record-key classification in Tranche A.
Vendoring is the INGEST-P-008 workaround.

## Severity
Moderate.

## Owning layer
Standard library (exists); distribution (INGEST-P-008).

## Acceptance test
`classify_word_number` is one table lookup with a proven miss case at
equal or better step cost, importing the table from a pinned stdlib
identity with no vendored copy.

---

## INGEST-P-005 — Folds have no dead-iteration protection

## Problem
Fold over fixed storage while only some lanes are live (word classes,
digit accumulation, field staging). Dead lanes still execute `next`.

## Reproducer
Two genuine transfer-slice bugs, both caught by
`tests/convergence.rs`, not the compiler: dead lanes overwrote
`classes[0]` with `-1` (source entity lost, every sentence failed);
digit accumulation multiplied by 10 on dead lanes (quantity garbage
`5594136148269072384` for input `3`).

## Expected semantics
Liveness-gated writes (a `where`-style fold guard the compiler
understands) or checked indexing that makes dead-lane writes
impossible rather than discouraged.

## Current result
Still reproduces at 0.14: eleven folds carry hand-written
keep-patterns (`select(live, new, keep)`); `ingest_debug` exposes
`[count, verb_idx, verb_count, classes]` so tests pin each stage.

## Architectural impact
Every new fold (Tranche A/B/C add many) must be audited for the same
pattern; the compiler cannot distinguish a live write from a dead
clobber.

## Severity
Major (most bug-dense surface encountered).

## Owning layer
Language design (iteration); compiler (liveness analysis or guarded
fold form).

## Acceptance test
Deleting the `keep_class` guard in `classify_all` is a compile error
(or provably identity); the convergence suite still passes.

---

## INGEST-P-006 — Strict `select` forces defensive clamping (partially relieved)

## Problem
Index arrays with computed positions without doubling
index-expression noise or risking a trap on a guarded-away projection.

## Reproducer
`span_start`/`span_length` clamp twice (sentinel `-1` plus storage
bound) because both `select` candidates evaluate — the established
`text_scan` idiom.

## Expected semantics
A lazy conditional (evaluate only the taken side) where strictness buys
nothing, keeping strict `select` where timing uniformity matters; plus
the 0.14 `checked_index` discharge form adopted where it fits.

## Current result
Partially relieved: `checked_index(sequence, index)` (0.14) retains one
dominating bounds check per sequence and discharges projection
obligations on all backends. Lazy conditionals still absent; `replace`
and cross-sequence uses keep historical obligations.

## Architectural impact
Clamp-then-project idiom retained; Tranche B adopts `checked_index`
for decoder hot paths and measures the difference.

## Severity
Ergonomic (correctness-adjacent).

## Owning layer
Language design (evaluation order); compiler (discharge).

## Acceptance test
`span_start` expresses the `-1` case without evaluating the
projection; timing-sensitive stdlib behavior unchanged.

---

## INGEST-P-007 — Nested module failures hide inner diagnostics

## Problem
Compile a multi-module program and fix errors in leaf modules without
manual bisection.

## Reproducer
Re-probed 2026-09-11 on the current pin: a leaf field named `over`
yields only `MNE172 Error line 13 col 5: imported module
'mncs.ingest.parse' failed to parse [MNP127, MNP007]` on the root —
codes without positions or messages. Locating it required recompiling
each leaf as its own root (`LanguageRuntime::diagnose_root`).

## Expected semantics
Propagate the first inner diagnostic (module path, span, message) with
the `MNE172` wrapper instead of bare codes.

## Current result
Still reproduces exactly.

## Architectural impact
`diagnose_root` helper kept in-tree; multi-module development pays a
bisection loop per leaf error, discouraging the recommended shape.

## Severity
Moderate (diagnostic/tooling).

## Owning layer
Compiler (frontend); tooling.

## Acceptance test
The `over`-field program reports `MNP127` with leaf path and line
through the root compile, with no extra host code.

---

## INGEST-P-008 — No stdlib distribution for downstream crates

## Problem
Reuse bounded byte/sort/stream primitives (`chunk`, `json_stream`,
`encoding`, `sha256`, `text_map`) instead of reimplementing them per
crate.

## Reproducer
`mncs.std.*` resolves in the CLI only through `MNCS_LIBRARY_PATH`; an
in-process consumer must vendor sources or point a resolver at another
repository's working tree. `language/vendor/` + `VENDORING.md` in this
repo is the workaround that proves the gap.

## Expected semantics
A versioned stdlib artifact (content-addressed bundle or registry pin)
that `ReferenceCompiler` consumers resolve without filesystem coupling
to the language checkout.

## Current result
Still reproduces; need grew (this run vendors six modules).

## Architectural impact
Vendored copies must be re-pinned by hand; duplicated primitives risk
drift (byte-identical + noted, but manual).

## Severity
Moderate (ecosystem).

## Owning layer
Distribution/packaging; compiler (resolver).

## Acceptance test
A module imports `mncs.std.chunk.v1` from a pinned stdlib identity and
the ingest build touches no vendored copies.

---

## INGEST-P-009 — Fixed windows + counts cross every boundary

## Problem
Pass sentences, words, chunks, and code vectors between host and MNCS
without per-call padding/length ceremony that both sides must agree on
out-of-band.

## Reproducer
Every ingest entry point is `(storage, explicit length)`:
`ingest_parse(text, length)`, `next_line(carry, carry_len, view,
view_len, eof)`. The host pads `[u64; 8]` windows; MNCS clamps.

## Expected semantics
Length-carrying sequence values whose declared length the callee trusts
without a second parameter.

## Current result
Still reproduces at 0.14 (views carry runtime length internally, but
entry points still take explicit lengths by convention and ABI).

## Architectural impact
Ceremony per call; units (bytes vs scalars) agreed out-of-band.
Tranche G measures the serialization volume.

## Severity
Ergonomic.

## Owning layer
ABI/embed; type system.

## Acceptance test
`ingest_parse` takes one argument; the host passes N bytes and MNCS
observes length N with no `length` parameter.

---

## INGEST-P-010 — Reference-execution cost per call (partially relieved)

## Problem
Ingestion is a high-frequency boundary; per-observation verdicts must
be cheap. Prior measurement (old pin): ~140 ms floor per trivial call
in release, ~6 s `parse_transfer` in debug.

## Reproducer
`cargo run --offline --example` timing probes (recorded in run notes);
suite wall time dominated by elaboration + per-call overhead.

## Expected semantics
A session/amortized execution handle paying program setup once, or a
supported backend-execution embedding keeping per-call cost near
interpretation cost.

## Current result
Partially relieved on the language side: `mncs-embed` provides retained
`Session::open` + `call`/`call_batch` (Rust + C ABI). Ingest has NOT
adopted it — current cost is ingest adoption debt until Tranche G
measures embed vs `execute_with_policy`. True residual overhead TBD.

Re-probed 2026-09-11 (debug host, contended laptop, serial tests):
single reference calls land in the low seconds; the serial manifest
tail (13 split widths + 60-case fuzz + golden + bounds + regression)
took 4779 s wall, the serial manifest depth check 1271 s. Four
orphaned pre-fix test binaries plus a peer `mncs-language` workspace
run visibly multiplied suite time (~6x on isolated CSV cases), so
future runs should clear strays first — contention, not verdicts,
dominates. No new language gap surfaced while taking the manifest
scanner/extractor green: every failure root-caused above the language
line (fold gating, expect-state overload, host error mapping).

## Architectural impact
Reference execution everywhere; suites share one runtime per binary.
Tranche G adopts embed sessions and re-measures; do NOT file
"subprocess overhead" (ingest never shelled out).

## Severity
Performance.

## Owning layer
ABI/embed (exists); ingest adoption (this run); runtime (residual).

## Acceptance test
`classify_event` sustains ≥100 calls/second in a release host while
still executing MNCS (any backend), with identical verdicts.

---

## New pressures from this run

---

## INGEST-P-011 — Stdlib scans/readers are width-monomorphic

## Problem
Reuse `mncs.std.chunk` newline scans and `mncs.std.encoding` endian
readers at 256/392-byte stage widths instead of reimplementing the
same 5-line folds per width.

## Reproducer
`mncs.std.chunk.find_newline` takes `[byte; up_to 64]`; `encoding.
read_u16_le` takes `[byte; up_to 64]`. A 256-byte chunk view or a
392-byte frame stage cannot call them: narrowing a wider live span
into a 64-capacity view fails closed at runtime. `mncs.flow.lines`
and `mncs.flow.frames` therefore reimplement both folds locally.

## Expected semantics
Capacity-generic views (`[byte; up_to N]` with `N: Nat`) so one scan
serves every stage width, or widening conversions with explicit
runtime span checks.

## Current result
Reproduces at 0.14: generics cover exact sizes and needle widths
(`text_scan.contains<N>`), but view capacities are fixed per
signature. (Note: exact→view `[0..n]` slicing exists; view→view
sub-slicing does not — see INGEST-P-002.)

## Architectural impact
Duplicated folds per width in `lines.mncs`/`frames.mncs`; vendored
`chunk`/`encoding` usable only where 64 bytes suffice.

## Severity
Moderate.

## Owning layer
Type system (capacity generics); standard library.

## Acceptance test
`lines.mncs` calls vendored `chunk.find_newline`-equivalent and
`frames.mncs` calls vendored `encoding.read_u16_le` with zero local
folds; behavior and tests unchanged.

---

## INGEST-P-012+ reserved for tranche C–H findings.
