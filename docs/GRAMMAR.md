# Bounded transfer grammar (slice v1)

This is the **complete** text contract. Anything outside it is a
controlled error, never a guess. The grammar is executed by
`mncs.ingest.parse` (`language/mncs/ingest/parse.mncs`); this document
describes exactly what that program decides.

## Forms

```text
active:  Entity VERB Entity (Number)? Object "."
passive: Entity RECEIVE-VERB (Number)? Object FROM Entity "."
```

## Vocabulary

| Slot | Accepted spellings (ASCII case-insensitive) |
|---|---|
| `VERB` | `gave`, `give`, `gives` |
| `RECEIVE-VERB` | `received`, `receive`, `receives` |
| `FROM` | `from` |
| `Number` | `zero`..`ten`, or 1–3 decimal digits (`0`..`999`) |
| `Object` (known) | `apple`, `apples`, `orange`, `oranges` → codes 1, 2 |
| `Object` (unknown) | any other lowercase ASCII word, kept verbatim as an explicit unknown |
| `Entity` | capitalized ASCII word (`Alexander`), or word starting with a non-ASCII byte (UTF-8 pass-through; the host validates UTF-8, MNCS treats the bytes opaquely) |

## Shape rules

- At most 64 input bytes, at most 8 whitespace-separated words.
- Exactly one verb; exactly two entities (one per side).
- At most one object word and at most one number word, both after the verb.
- The passive form requires exactly one `from` before the source entity.
- The sentence ends with a single `.`; trailing spaces are tolerated.
- Every word must fill exactly one role slot; leftovers fail closed.

## Statuses

| Code | Meaning | Host error |
|---|---|---|
| 0 | ok | — |
| 1 | no supported verb found | `Unsupported` |
| 2 | a participant is missing (source, target, object, or passive `from`) | `MissingField` |
| 3 | a digit-led word after the verb is not a valid quantity | `Malformed` |
| 4 | input is outside the grammar (extra words, doubled verb, stray markers) | `Malformed` |
| 5 | empty input | `Malformed` |

## Deliberate boundaries

- No articles (`the`), no adverbs, no tense beyond the listed verbs.
- Number words stop at `ten`; `twelve` reads as an unknown-object
  candidate and fails closed on shape rather than guessing a value.
- Entity identity is spelling-exact (`Finn` ≠ `finn`); a lowercase
  entity-shaped word collides with the object slot and fails closed.
- A missing quantity is absence, not zero (`Some(0)` vs `None` is
  load-bearing through equality).
- Unknown objects converge only with byte-identical spellings.
