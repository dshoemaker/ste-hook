# comment-lint

A linter for **source-code comments**, built to stop LLM comment bloat:
narration comments that restate the adjacent code, change-log residue
("now uses X"), and filler prose. A rule-based subset of ASD-STE100
Simplified Technical English supplies the secondary prose rules. Single
Rust binary, no runtime dependencies.

Primary surface: a Claude Code **Stop hook** that blocks the end of an
agent turn while findings remain, so the agent rewrites or deletes its own
comments. See [PLAN.md](PLAN.md) for the design decisions and
[docs/adr/](docs/adr/) for the two that will otherwise look arbitrary.

Ruby, JavaScript, TypeScript, and TSX via tree-sitter.

## Rules

| Code | Severity | Check |
| --- | --- | --- |
| `RED001` | error | comment restates the adjacent code |
| `RED002` | error | narration or change-log phrasing ("now uses", "previously", "fixed a") |
| `STE001` | error | word limits, **doc comments only**: 20 procedural, 25 descriptive |
| `STE002` | error | max 6 sentences per block |
| `STE006` | error | imprecise or wordy phrase ("in order to", "utilize", "simply") |
| `STE007` | error | semicolon |
| `STE003`–`STE005` | warning | passive voice, complex verbs, `-ing` forms — off by default |

Two positions matter. A **doc comment** sits directly above a `def`,
`class`, or function and gets the word limits. A **body comment** lives
inside code and has no length cap — rationale ("why", constraints, units)
is the one comment category worth keeping, and it is never punished for
being thorough. An STE-perfect four-word narration comment is a finding;
a 30-word rationale comment is not. That asymmetry is the design
([ADR-0001](docs/adr/0001-bloat-is-the-goal-ste-is-the-means.md)).

`RED001` is deliberately strict: it fires only when every content word of
the comment already appears in the identifiers of the code it is attached
to (strict line adjacency, camelCase/snake_case split, light stemming).
Overlap plus any surplus information passes.

## Block joining

Consecutive line comments at the same indentation are joined into one
paragraph before rules run, so a sentence split across three lines is
evaluated as one sentence. Every finding maps back to its real line and
column. A trailing comment (code before it on the same line) is never
linted.

## Masking

Comments are not prose. Backticked spans, URLs, `snake_case`, `camelCase`,
`Foo::Bar.baz`, `SCREAMING`, `1.2.3`, and `call(args)` are blanked to
same-length filler before prose rules run, so offsets stay valid and
identifiers never trigger findings. `RED001` runs on the unmasked text —
masking blanks exactly the identifiers its overlap test needs.

## Suppression

```ruby
# comment-lint:ignore STE001 -- quoted verbatim from the upstream RFC
# A very long sentence that must stay exactly as written ...
```

Bare `comment-lint:ignore` suppresses every rule for the next block; a
comma list scopes it. The directive line itself is not linted and breaks
the block.

## Use

```console
comment-lint app/**/*.rb
comment-lint --rules RED001,RED002,STE006 --format jsonl src/
git diff --name-only -z | comment-lint --files0-from - --format jsonl
```

Exit `0` when clean, `1` when findings exist, anything above 1 is a tool
failure. `--format agent` produces the feedback block the Stop hook sends
back to the agent: rule, span, and guidance — never replacement prose.
There is no autofix and no `fix` command by design.

## Install (per project, opt-in)

1. `cargo install --path .`
2. Copy `hooks/comment-lint-stop.sh` into the target repo's
   `.claude/hooks/` and merge `settings.json` into its
   `.claude/settings.json`. Not as a plugin — plugin-installed Stop hooks
   have a known bug where exit 2 halts instead of continuing.
3. Append `CLAUDE-comment-style.md` to the repo's `CLAUDE.md` so the agent
   writes compliant comments the first time.
4. Optional pre-commit gate: see [docs/hk-recipe.md](docs/hk-recipe.md).

The hook lints only files the session's agent actually edited (read from
the transcript), allows up to three correction rounds per stop, and never
blocks when the linter itself fails — it warns instead. `STE_RULES`
overrides the default rule set; `COMMENT_LINT_BIN` pins the binary.

## Attribution

The passive-voice, complex-verb, `-ing`, and imprecise-phrase heuristics
are ported from [johnsaigle/ste-lint](https://github.com/johnsaigle/ste-lint)
(MIT, Copyright (c) 2026 ste-lint contributors).

The block-joining predicate follows RuboCop's `Layout/EmptyComment`.

## Notice

ASD-STE100 is a copyright and registered trademark of ASD, Brussels
(EUTM 017966390). The `STE`-prefixed rules implement structural checks
inspired by the published writing rules; the `RED` rules are original to
this tool and unrelated to the specification. This tool does not reproduce
the specification text or the controlled dictionary, is unaffiliated with
ASD or the STEMG, and is not certified by either. Request the official
specification free of charge from asd-ste100.org.
