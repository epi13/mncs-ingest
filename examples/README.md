# Examples

Executable convergence fixtures, not illustrations. `gen.rs` ingests the
four inputs below and asserts they converge; its outputs are checked in
so the claim is inspectable without running code.

```text
examples/transfer/
├── text_active.txt    # "Alexander gave Finn 3 apples."
├── text_passive.txt   # "Finn received three apples from Alexander."
├── event.json         # equivalent structured event
├── native.json        # equivalent native code record (documentation of the shape)
├── canonical.txt      # canonical bytes all four converge to (generated)
└── fragment.json      # canonical fragment, schema fragment-v1 (generated)
```

Regenerate with:

```bash
cargo run --offline --example gen
```

`native.json` documents the native input shape; the executed native path
is `NativeTransfer` (see `src/adapters/native.rs` and
`tests/adapters.rs`), since native input is typed values, not bytes.
