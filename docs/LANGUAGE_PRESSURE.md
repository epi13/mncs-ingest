# MNCS Language Pressure (from building `mncs-ingest`)

Every entry below was earned by implementation work in this repository,
not by toy probes. Each cites the ingest code that hit the limitation,
the workaround in use, and a concrete acceptance test. Severities use
the run's categories (`BLOCKER/HIGH/MEDIUM/LOW/ERGONOMIC/PERFORMANCE/
DIAGNOSTIC/RUNTIME/COMPILER/LANGUAGE-DESIGN`).

`mncs-ingest` pins `mncs-language` rev
`d7cc9536f0507fc34086d8a5784c6177ce5b67b4` and declares Source Profile
0.13 in all four `language/` modules.

---

## P-STRING-01: No string type — open-vocabulary identity cannot round-trip

Area: language-design, value model
Severity: HIGH
Status: open, worked around

### Ingest requirement
Fragments must carry entity spellings (`Alexander`) and unknown-object
spellings (`parsnips`) with byte-exact identity, and render canonical
object labels (`apple`) from canonical codes.

### Current MNCS behavior
MNCS has bytes and fixed byte arrays but no string/text value:
`parse_transfer` returns `source_start/source_length` integer spans into
the caller's buffer, and the host slices the original input
(`src/adapters/text.rs`). The code→label render table
(`object_label` in `src/canonical.rs`) duplicates the MNCS vocabulary
tables because MNCS cannot return the spelling it just classified.

### Why this matters
Semantic equality is bifurcated by force: codes compare in
`ingest_codes_equal`, spellings in `ingest_spell_equal`, and the host
ANDs the verdicts. A two-entry render table is tolerable; a 200-word
vocabulary would be a drift hazard. The `vocabulary_codes_match_host_
rendering` test exists only to pin the duplication.

### Minimal reproducer
```mncs
// There is no expression of "return the matched word": the classifier
// can only return that it matched (a code), never what matched.
fn classify_object(text: [byte; up_to 64], start: u64, length: u64) -> (result: i64)
```

### Current workaround
Span returns + host slicing; mirrored two-entry host table.

### Desired language capability
A bounded text value (explicit max bytes, UTF-8 validated at the
boundary) that MNCS can construct, return, compare, and project back to
bytes — without heap allocation or an unbounded string type.

### Acceptance criteria
`ingest_parse` returns the source spelling as a value (not a span), and
`src/canonical.rs::object_label` is deleted with no replacement.

---

## P-STRING-02: No substring / view-slicing primitive

Area: language-design, sequences
Severity: HIGH
Status: open, designed around

### Ingest requirement
Split one 64-byte sentence into word spans and classify each word with
shared table functions.

### Current MNCS behavior
A `[byte; up_to 64]` view cannot be sub-viewed. Every classifier in
`mncs.ingest.vocab` therefore takes `(text, start, length)` triples and
re-derives clamping internally (`span_byte`, `span_eq`,
`span_prefix_eq`). The structured adapter reuses the same entry points
with `start = 0`, which works but makes every signature carry
positional plumbing the type system cannot check.

### Why this matters
Positional plumbing is a bug farm: start/length swaps are invisible to
the compiler, and every helper repeats the same clamp prologue (~15% of
`vocab.mncs` by line count).

### Minimal reproducer
```mncs
// Wanted: classify_word(word_view) where word_view borrows text[a..b].
// Available: classify_word(text, a, b - a) with manual clamping.
```

### Current workaround
Span-triple convention documented in `vocab.mncs` header.

### Desired language capability
Bounded borrow-slices over sequences/views with compiler-checked
bounds, so a callee receives a view it cannot index out of.

### Acceptance criteria
`vocab.mncs` classifiers take one view argument; the prologue shrinks
to a bounds proof instead of three `u64` parameters.

---

## P-STRING-03: No UTF-8 scalar semantics

Area: language-design, text
Severity: MEDIUM
Status: open, split across the boundary by design

### Ingest requirement
Unicode entity names (`Zoë`) must survive ingestion with exact identity.

### Current MNCS behavior
Byte operations never assume scalar boundaries (the stdlib states this
explicitly). `is_entity_word` accepts bytes `>= 128` opaquely, and the
host validates UTF-8 before MNCS ever sees the span
(`unicode_entities_pass_through_all_adapters`).

### Why this matters
The split is principled today, but MNCS can never *validate*,
normalize, or case-fold non-ASCII text itself. Any future normalization
(case-insensitive entity convergence, NFC) must live on the host.

### Current workaround
Host validates; MNCS passes high bytes through (`vocab.mncs`).

### Desired language capability
Producer-attested UTF-8 views plus a minimal scalar-step primitive
(first-scalar/rest), so classifiers can walk characters without owning
decoding tables.

### Acceptance criteria
An MNCS function rejects invalid UTF-8 (`[0xC3, 0x28]`) and folds a
non-ASCII case pair, executed on all backends.

---

## P-COLL-01: No maps — closed vocabularies scale as linear if-chains

Area: language-design, collections
Severity: MEDIUM
Status: open, acceptable at this vocabulary size

### Ingest requirement
Classify ~25 keywords (verbs, number words, objects) over byte spans.

### Current MNCS behavior
There is no map/dictionary/set value. `classify_word_number` is eleven
sequential `span_eq` calls; each is an 8-wide fold, so one word costs up
to ~700 byte comparisons before digits are even tried.

### Why this matters
Correct and deterministic, but O(vocabulary × width) per word with the
constant paid on every backend. A 200-word vocabulary would be silly to
hand-write and slow to execute.

### Current workaround
Linear chains; vocabulary deliberately closed and tiny.

### Desired language capability
A bounded, deterministic finite map (const keys, total lookup, explicit
miss) with the same value on every backend.

### Acceptance criteria
The eleven number-word branches collapse to one table lookup with a
proven miss case, at equal or better step cost.

---

## P-FOLD-01: Folds have no dead-iteration protection — writes must be identity

Area: language-design, iteration
Severity: MEDIUM
Status: open, disciplined around (two real bugs caught by tests)

### Ingest requirement
Fold over fixed storage while only some lanes are live (word classes,
digit accumulation).

### Current MNCS behavior
`iterate` covers storage; iterations past the live window still execute
`next`. Anything written unconditionally on a dead lane corrupts live
state. Two genuine bugs resulted: dead lanes overwrote `classes[0]`
with `-1` (source entity lost, every sentence failed), and digit
accumulation multiplied by 10 on dead lanes (quantity garbage such as
`5594136148269072384` for input `3`). Both were caught by
`tests/convergence.rs`, not by the compiler.

### Why this matters
The keep-pattern (`select(live, new, keep)`) must be remembered at
every write; the compiler cannot tell a live write from a dead
clobber. This is the most bug-dense surface encountered in the run.

### Current workaround
Audited keep-pattern on all eleven folds; `ingest_debug` exposes
`[count, verb_idx, verb_count, classes]` so tests pin each stage.

### Desired language capability
Either liveness-gated writes (a `where`-style fold guard the compiler
understands) or a checked-indexing mode that makes dead-lane writes
impossible rather than merely discouraged.

### Acceptance criteria
Deleting the `keep_class` guard in `classify_all` is a compile error
(or provably identity); the convergence suite still passes.

---

## P-SELECT-01: Strict `select` forces defensive clamping everywhere

Area: language-design, semantics
Severity: ERGONOMIC (correctness-adjacent)
Status: open, idiom adopted

### Ingest requirement
Index arrays with computed positions (`starts[slot]`, `text[pos]`).

### Current MNCS behavior
Both `select` candidates evaluate, so a guarded-away projection still
traps on an out-of-range index. Every projection in the slice carries an
explicit clamp (`select(i < len, i, 0)`), including projections the guard
already excludes. The stdlib (`text_scan`) documents the same idiom, so
this is established practice — but it roughly doubles index-expression
noise and punishes one missed clamp with a trap.

### Current workaround
Clamp-then-project idiom throughout; `span_start`/`span_length` clamp
twice (sentinel `-1` plus storage bound).

### Desired language capability
A lazy conditional expression (evaluate only the taken side) for the
cases where strictness buys nothing, keeping strict `select` where
timing-channel uniformity matters.

### Acceptance criteria
`span_start` expresses the `-1` case without evaluating the projection;
existing timing-sensitive stdlib code is unaffected.

---

## P-DIAG-01: Nested module failures hide their inner diagnostics

Area: diagnostics, tooling
Severity: MEDIUM
Status: open, worked around

### Ingest requirement
Compile a four-module program and fix errors in leaf modules.

### Current MNCS behavior
Through `front_end_with_resolver`, a leaf parse error surfaces only as
`MNE172: imported module 'mncs.ingest.parse' failed to parse
[MNP127, MNP007]` on the root — codes without positions or messages.
Two concrete cases: a record field named `over` (reserved word,
`MNP127`) and the `u64`-domain/profile mismatch below. Both required
recompiling each leaf as its own root
(`LanguageRuntime::diagnose_root`, kept in the tree) to locate.

### Why this matters
Multi-module programs are the recommended shape, but the error path
discourages them: every leaf error costs a manual bisection loop.

### Current workaround
`diagnose_root` helper + per-module root compilation during development.

### Desired language capability
Propagate the first inner diagnostic (module path, span, message) with
the `MNE172` wrapper instead of bare codes.

### Acceptance criteria
The `over`-field program reports `MNP127` with the leaf path and line
through the root compile, with no extra host code.

---

## P-PROFILE-01: `u64` traversal domains require 0.13 (observed, resolved)

Area: compiler, profiles
Severity: LOW (resolved by declaring the current profile)
Status: resolved — recorded for profile ergonomics

### Ingest requirement
Iterate over `[u64; 8]` span tables at profile 0.10.

### Current MNCS behavior
`MNE194: u64 sequence traversal domains require source profile 0.13 or
later`. Byte-element iteration worked; `u64`-element iteration was
refused. Declaring `mncs 0.13` (the current profile) resolved it with
no other code changes; `bool == bool` (`MNE121` at 0.10) came along the
same way.

### Why this matters
Mild: the fix is one line. It is recorded because the failure mode —
"this ordinary loop needs a newer profile" — will recur for every
downstream crate until profiles stabilize, and because the diagnostic
was accurate and actionable (good behavior worth keeping).

### Acceptance criteria (already met)
All four modules declare 0.13 and elaborate with zero diagnostics.

---

## P-MOD-01: No stdlib distribution for downstream crates

Area: ecosystem, modules
Severity: MEDIUM
Status: open, self-contained by choice

### Ingest requirement
Reuse bounded byte/sort primitives instead of reimplementing them.

### Current MNCS behavior
`mncs.std.text_scan/sort/token_set` exist in the language tree, but
there is no versioned distribution a downstream crate can depend on:
the CLI resolves them through `MNCS_LIBRARY_PATH`, and an in-process
consumer must vendor sources or point a resolver at another
repository's working tree (which may be dirty — it was, during this
run). `mncs-ingest` therefore reimplements ~40 lines of byte helpers
and insertion sort in its own modules.

### Why this matters
Every downstream crate pays the same reimplementation tax or couples
its build to another repo's checkout. Duplicated primitives drift.

### Current workaround
Self-contained `vocab`/`frame` modules; no stdlib imports.

### Desired language capability
A versioned stdlib artifact (content-addressed bundle or registry pin)
that `ReferenceCompiler` consumers can resolve without filesystem
coupling to the language checkout.

### Acceptance criteria
`vocab.mncs` imports its byte helpers from a pinned stdlib identity and
the ingest build touches no vendored copies.

---

## P-ABI-01: Fixed windows + counts cross every boundary

Area: runtime, ABI
Severity: ERGONOMIC
Status: open, conventional

### Ingest requirement
Pass sentences, words, and code vectors between host and MNCS.

### Current MNCS behavior
Every crossing is `(storage, explicit length)`: `[byte; up_to 64]` plus
`u64`, `[u64; 8]` padded plus count. The host pads, MNCS clamps, and
both sides must agree on units (bytes vs scalars). It works — the
`arch.classify` precedent shows the way — but it is ceremony per call.

### Desired language capability
Length-carrying sequence values whose declared length the callee can
trust without a second parameter (or a one-sided fat-pointer spelling).

### Acceptance criteria
`ingest_parse` takes one argument; the host passes N bytes and MNCS
observes length N with no `length` parameter.

---

## P-PERF-01: Reference-execution cost per call is ingestion-scale hostile

Area: performance, runtime
Severity: PERFORMANCE (medium — verification path, not the only runtime)
Status: open, measured

### Ingest requirement
Ingestion is a high-frequency boundary; per-observation verdicts must be
cheap. Measured on the pinned rev, host `debug`/`release` builds:

| operation | debug/call | release/call |
|---|---|---|
| frontend elaborate (once) | 6.3 s | 0.48 s |
| `classify_event("apple")` | 1.57 s | 0.14 s |
| `spell_equal` (9 bytes) | 1.55 s | — |
| `parse_transfer` (29 bytes) | 6.0 s | 1.0 s |

Even the trivial two-branch classifier costs ~140 ms/call in release,
so most of the floor is per-call overhead (request setup, program-wide
pre-execution work), not fold depth. The full test suite takes ~5
minutes, dominated by elaboration plus per-call overhead — mitigated in
the suite by sharing one runtime per binary (`tests/common/mod.rs`).

### Why this matters
A 140 ms floor per verdict rules out reference execution on any
high-frequency ingest path. The mitigation direction is already
visible: `backend_matrix` proves the same entry points execute through
the research-bytecode and portable-WASM pipelines, which are the honest
fast path — but ingest has no backend-execution integration yet, only
reference calls plus a matrix probe.

### Current workaround
Reference execution everywhere; suite shares runtimes.

### Desired language capability
Either a session/amortized execution handle that pays program setup
once, or a supported backend-execution embedding for host crates that
keeps per-call cost near interpretation cost.

### Acceptance criteria
`classify_event` sustains ≥100 calls/second in a release host while
still executing MNCS (any backend), with identical verdicts.

---

## What worked (non-pressures)

- Multi-module linking (`use … as …`, qualified record/finite types,
  cross-module calls) compiled first try after syntax fixes.
- Records, `iterate`/`carrying`/`next`, `select`, `replace`, wrapping
  arithmetic, and byte/integer casts expressed the whole slice.
- Reference execution (`execute_with_policy`) plus real backend
  execution (`backend_matrix`) ran from the pinned crates with no glue
  beyond ~200 lines of host code.
- Diagnostics that did surface positions (`MNE194`, `MNE121`) were
  accurate and actionable.
