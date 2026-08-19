# comment-lint — implementation note (historical)

> **Superseded.** This is the original handoff, kept as a record. The
> decisions in [PLAN.md](PLAN.md) replace its open questions, and the plan
> has been executed: tests exist, tree-sitter is current, the RED rule
> family and position-aware STE001 are built, the directive is
> `comment-lint:ignore`, and the Stop hook scopes via the transcript.

You are picking up a working prototype. It compiles and its behaviour is
verified. This note tells you what is done, what is not, and what to be
careful about.

## What this is

A single Rust binary that lints **source-code comments** against a
rule-based subset of ASD-STE100 Simplified Technical English. It is wired
into two places:

1. `hk` as a `check` step (pre-commit and `hk check`).
2. A Claude Code **Stop hook**, which blocks the end of an agent turn while
   findings remain, so the agent rewrites its own comments.

There is deliberately **no autofix and no `fix` command**. The tool reports
the rule and the offending span. It never emits replacement prose. This is
a hard design constraint, not an unimplemented feature — do not add one.

## Layout

```
Cargo.toml
src/blocks.rs   tree-sitter comment extraction, block joining, ste:ignore
src/rules.rs    masking, sentence segmentation, STE001-STE007
src/main.rs     CLI, output formats, offset -> line/column mapping
hk.pkl          hk step definition
hooks/comment-lint-stop.sh   Claude Code Stop hook
CLAUDE-comment-style.md      paste into the repo's CLAUDE.md
```

## The three ideas that matter

**1. Block joining.** Consecutive line comments at the same indentation are
joined into one paragraph before any rule runs, so a sentence spanning three
lines is one sentence. The predicate is `previous_row + 1 == row &&
previous_col == col`, taken from RuboCop's `Layout/EmptyComment`. A comment
with code before it on the same line is a trailing note and never joins.

**2. Masking.** Comments are not prose. Backticked spans, URLs,
`snake_case`, `camelCase`, `Foo::Bar.baz`, `SCREAMING`, `1.2.3`, and
`call(args)` are overwritten with same-length NUL filler before the rules
run. Same length in, same length out — this is what keeps every offset
valid. Rules scan the masked string and report against the original.

If you touch `mask()`, **preserve the length invariant**. Breaking it
silently misreports every column in the file.

**3. Provenance map.** `Block.map` holds one `(row, col)` entry per byte of
the joined text. That is how a finding at offset 91 in a joined paragraph
becomes `file.rb:7:17`. Verified by caret alignment against real source.

## Verified behaviour

- Findings map to correct line and column across joined blocks, including
  indented ones.
- Trailing comments excluded; `frozen_string_literal`, `rubocop:`,
  `eslint-`, `@ts-` directives skipped and correctly breaking blocks.
- Code-heavy comments produce zero findings:
  `` # Call `Widget#find_by_slug`. The method uses ActiveRecord::Base.connection. ``
- `# ste:ignore` (blanket) and `# ste:ignore STE006` (scoped) both suppress.
- Stop hook: exits 0 when clean, 2 when findings exist, 0 when
  `stop_hook_active` is true.
- Ruby, JS, TS, TSX all parse.

## Known gaps — do these first

1. **Toolchain.** Built with Rust 1.75 / edition 2021 and tree-sitter 0.20,
   because that is what was available. Move to a current toolchain and
   tree-sitter 0.25+. The grammar API changed: `Parser::set_language(&lang)`
   now takes a reference and the `language()` functions return
   `LanguageFn`. Expect small mechanical fixes in `blocks.rs`.

2. **No tests.** There is not a single `#[test]`. Add them before changing
   anything. Highest-value cases, all currently verified only by hand:
   - block joining across 3 lines maps offsets to the right line/column
   - trailing comment does not join
   - directive line breaks a block
   - masked identifier produces no finding
   - `ste:ignore` scoped and blanket
   - multi-line `/* */` and `=begin` unwrapping

3. **`hk.pkl` is unvalidated.** Written from the docs; `hk validate` was
   never run. Check the `Command { argv = ... }` form and the standalone
   `{{files}}` expansion.

4. **23 MB binary.** Four tree-sitter grammars statically linked. Feature-gate
   the languages if that matters.

5. **Sentence segmentation is naive.** Abbreviation list is 8 entries. It
   handles `e.g.` and dotted identifiers, but will mis-split on unusual
   input. Consider a real segmenter if false splits show up.

6. **`is_procedural` is a 30-word imperative list.** It stands in for POS
   tagging to choose the 20- vs 25-word limit. It defaults to the looser
   descriptive limit when unsure, which is the safe direction.

## Rollout

Ship with `--rules STE001,STE002,STE006,STE007` only. Those four are
objectively checkable with near-zero false positives and are safe to block
on. `STE006` pays for itself immediately against LLM-authored comments.

`STE003`, `STE004`, `STE005` are regex approximations of POS-dependent
rules. Add them at warning severity later, after observing real noise.

## The loop hazard

The Stop hook blocks by exiting 2. If a finding cannot be resolved, the
agent loops. Three guards, all already in place — do not remove any:

- `stop_hook_active` check exits 0 on the second pass.
- `ste:ignore` gives the agent an explicit escape hatch, and the agent
  output tells it so.
- Any exit status above 1 from `comment-lint` is treated as a tool failure,
  not a lint failure, so a broken linter never blocks a turn.

Also note: **exit 1 does not block a Claude Code hook.** Only 2 does. The
binary exits 1 on findings because that is what `hk` and CI expect; the
wrapper script translates it. Keep that translation.

Install the hook under `.claude/hooks/`, not as a plugin — plugin-installed
Stop hooks have a known bug where exit 2 halts instead of continuing.

## On adding a RuboCop cop

A parallel RuboCop cop was prototyped and is **not** included here, on
purpose. Two implementations of the same regexes will drift. If you add a
cop later, give it rules this binary structurally cannot do — ones needing
the comment's attachment to a definition (is this documenting a public
method? does it name parameters that exist?) — and let the binary keep sole
ownership of the prose rules. Do not reimplement STE001–STE007 in Ruby.

## Attribution and licensing

The passive-voice, complex-verb, `-ing`, and imprecise-phrase heuristics are
ported from [johnsaigle/ste-lint](https://github.com/johnsaigle/ste-lint),
MIT, Copyright (c) 2026 ste-lint contributors. Keep the notice in
`rules.rs`.

ASD-STE100 is a copyright and registered trademark of ASD, Brussels
(EUTM 017966390). The specification's own terms forbid reproduction in whole
or in part without written authority. **Do not vendor the controlled
dictionary.** Structural rules are fine; the word list is not. This is why
the tool is rule-based only, and it should stay that way unless someone
obtains explicit permission.
