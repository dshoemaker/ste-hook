# comment-lint

STE-inspired linter for **source code comments**. Single Rust binary, no
runtime dependencies, wired into [hk](https://hk.jdx.dev) as a check step.

Consecutive line comments at the same indentation are joined into one
paragraph before rules run, so a sentence split across three lines is
evaluated as one sentence. Every finding maps back to its real line and
column.

Ruby, JavaScript, TypeScript, and TSX via tree-sitter.

## Why a new binary

`ste-lint` is the closest existing tool and its heuristics are reused here,
but it cannot be used as a library — `src/lib.rs` exports only `pub fn run()`,
every module is private. It also reads prose files only, and its stdin path
calls `Document::new(source, false)`, which disables code masking. Extracting
comments and remapping offsets is required either way; doing it in-process
avoids a temp-file shim.

## Rules

| Code | Severity | Check |
| --- | --- | --- |
| `STE001` | error | 20 words procedural, 25 descriptive |
| `STE002` | error | max 6 sentences per block |
| `STE003` | warning | possible passive voice |
| `STE004` | warning | complex verb construction |
| `STE005` | warning | `-ing` form needing review |
| `STE006` | warning | imprecise or wordy phrase |
| `STE007` | error | semicolon |

`STE003` is suppressed where `STE004` already covers the span, so
`has been opened` reports once.

## Suppression

```ruby
# ste:ignore STE001 -- quoted verbatim from the upstream RFC
# A very long sentence that must stay exactly as written ...
```

Bare `ste:ignore` suppresses every rule for the next block; a comma list
scopes it. The directive line itself is not linted and breaks the block.

## Use

```console
comment-lint app/**/*.rb
comment-lint --rules STE001,STE006 --format jsonl src/
git diff --name-only -z | comment-lint --files0-from - --format jsonl
```

Exit `0` when clean, `1` when findings exist.

## Masking

Comments are not prose. Backticked spans, URLs, `snake_case`, `camelCase`,
`Foo::Bar.baz`, `SCREAMING`, `1.2.3`, and `call(args)` are blanked to
same-length filler before any rule runs, so offsets stay valid and
identifiers never reach the prose rules.

## Attribution

The passive-voice, complex-verb, `-ing`, and imprecise-phrase heuristics are
ported from [johnsaigle/ste-lint](https://github.com/johnsaigle/ste-lint)
(MIT, Copyright (c) 2026 ste-lint contributors).

The block-joining predicate follows RuboCop's `Layout/EmptyComment`.

## Notice

ASD-STE100 is a copyright and registered trademark of ASD, Brussels
(EUTM 017966390). This tool implements structural checks inspired by the
published writing rules. It does not reproduce the specification text or the
controlled dictionary, is unaffiliated with ASD or the STEMG, and is not
certified by either. Request the official specification free of charge from
asd-ste100.org.
