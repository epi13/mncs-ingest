# Vendored standard library

`vendor/` holds byte-identical copies of `mncs-language` library sources
at the workspace pin (`f7c1ba3…`; see `Cargo.toml`). They satisfy
`mncs.core.*` / `mncs.std.*` imports for ingest's own MNCS modules via
the vendor fallback in `src/language.rs` (ingest-first resolution).

## Contents

- `mncs/core/logic.mncs`, `mncs/core/bytes.mncs` — boolean/byte predicates
- `mncs/std/chunk.mncs` — chunk cursors, cross-chunk spans, newline scans
- `mncs/std/json_stream.mncs` — chunked JSON structural validation
- `mncs/std/encoding.mncs` — endian readers, version codec
- `mncs/std/sha256.mncs` — pure bounded SHA-256 (streaming update/finalize)
- `mncs/std/text_view.mncs`, `mncs/std/text_scan.mncs`, `mncs/std/text_map.mncs` — spans, literal scans, code tables

## Why vendored instead of referenced

There is no versioned stdlib distribution a downstream crate can depend
on (INGEST-P-008). Pointing the resolver at the language checkout's
live `library/` would couple ingest builds to another repository's
working tree. Vendoring pinned bytes is reproducible and auditable.

## Re-pinning

```bash
REV=<new-pin>
for f in $(cd vendor && find . -type f); do
  git -C <mncs-language> show $REV:library/${f#./mncs/} > vendor/$f
done
```

Verify with `cmp` against `git show` (all files must be byte-identical;
any local patch must be called out here — currently there are none).
