# comment-lint v1 plan — post-grilling, 2026-08-19

Supersedes the open questions in [IMPLEMENTATION.md](IMPLEMENTATION.md),
which remains the prior agent's handoff record. Decisions below came out of
a grilling session; the reframing is recorded in
[ADR-0001](docs/adr/0001-bloat-is-the-goal-ste-is-the-means.md) and the
naming scheme in
[ADR-0002](docs/adr/0002-directive-and-rule-id-scheme.md). Glossary:
[CONTEXT.md](CONTEXT.md).

## What changed versus the handoff

The goal is stopping LLM comment bloat. STE prose rules are retained only
where they serve that. Consequences:

1. **New primary rule family: RED (redundancy).** Requires the tree-sitter
   attachment context the prior agent identified as feasible but did not
   build.
   - `RED001` — comment restates adjacent code: content words of the block
     substantially overlap the identifiers of the statement/definition it
     attaches to. Threshold is a tuning knob; start strict (flag only
     near-total overlap with no surplus information words).
   - `RED002` — narration/change-log phrasing: "now uses", "previously",
     "this change", "as requested", "first we", "updated to", and kin.
     Near-zero false positives; pure pattern match.
2. **STE001 word limits become position-aware.** They apply only to doc
   comments (block attached to a `def`/`class`/`module`/function). Body
   comments are exempt — rationale comments must never be punished for
   length. Do not extend limits to body comments; see ADR-0001.
3. **Directive renamed** to `comment-lint:ignore` (scoped or bare). STE
   IDs stay; RED rules are not STE-numbered. See ADR-0002. Touches
   `blocks.rs`, the `--format agent` message text, README, and
   CLAUDE-comment-style.md.

Blocking set, day one: `RED001, RED002, STE001, STE002, STE006, STE007`.
RED rules block immediately — the dogfood repo is the tuning environment.
STE003/STE004/STE005 stay out entirely (severity is binary at the hook:
non-blocking findings never reach the agent, so "warning" buys nothing).

## Stop hook rewrite (shell — can proceed in parallel with Rust work)

The hook is the sole enforcement surface in v1. No PostToolUse hook, no
pre-commit gate. Changes to `comment-lint-stop.sh`:

1. **Scope to agent-touched files via the transcript.** Hook stdin includes
   `transcript_path`; extract `file_path` from Edit/Write/MultiEdit/
   NotebookEdit tool calls in the JSONL, dedupe, keep existing files with
   supported extensions. The shipped dirty-files enumeration (`git diff` +
   `--cached` + untracked) becomes the *fallback* when the transcript is
   missing or unparsable — it must never be the primary path, because it
   blocks the agent on the user's own WIP comments.
2. **Bounded correction rounds.** Replace the unconditional
   `stop_hook_active → exit 0` with a counter (temp file keyed by
   `session_id` from hook stdin): re-block until clean or 3 rounds spent,
   reset the counter whenever the lint comes back clean. As shipped, the
   gate guaranteed only one round — fix 4 of 5 findings and the 5th
   shipped silently.
3. **Fail open, loudly.** Any linter exit above 1 (including 127) still
   exits 0, but emits a user-visible warning (Stop hook JSON
   `systemMessage`): "comment-lint unavailable (exit N); comments
   unchecked." Also invoke the binary by absolute path or export an
   explicit PATH including `~/.cargo/bin` — hooks do not run the
   interactive shell profile, so bare `comment-lint` hitting 127 is the
   expected default failure, not an edge case.

## Deployment

- Per-project opt-in: copy `.claude/hooks/comment-lint-stop.sh` + merge
  `settings.json` into each target repo. No user-level install.
- Dogfood in one active repo first; spread only after RED thresholds settle.
- CLAUDE-comment-style.md: rewrite to lead with "do not write narration
  comments" (RED family) rather than STE prose rules, update the directive
  syntax and position-aware limits, then append to the dogfood repo's
  CLAUDE.md. Prevention beats correction — this part of the handoff stands.
- Binary distribution: `cargo install --path .` per machine is fine at
  personal scale.

## No pre-commit in v1 — but keep the recipe

hk is cut from the deliverable. Keep the CLI contract that makes any gate
attachable later: exit 0 clean / 1 findings, `--files0-from -`,
`--format jsonl`. Move `hk.pkl` out of the crate into `docs/hk-recipe.md`
with a short walkthrough (step definition, `types` matching, the two pkl
tests, `hk validate` instruction — it was never run). Update its `--rules`
flag to the v1 blocking set when writing the recipe.

## Work order

0. **Restructure into a buildable crate.** `git init`; move `blocks.rs`,
   `rules.rs`, `main.rs` into `src/`; `comment-lint-stop.sh` into `hooks/`;
   confirm `cargo build` on the current toolchain with tree-sitter 0.20
   (edition 2021 builds fine on modern rustc).
1. **Characterization tests before anything else changes.** The
   hand-verified list from the handoff, as `#[test]`s: 3-line block joining
   maps offsets to correct line/column; trailing comment never joins;
   directive line breaks a block; masked identifier yields no finding;
   scoped and bare ignore; `/* */` and `=begin` unwrapping; masking length
   invariant (masked length == original length on every mask pattern).
2. **tree-sitter 0.25 migration** against the green suite. Known API
   changes: `Parser::set_language(&lang)` takes a reference; grammar crates
   return `LanguageFn`.
3. **New rules, TDD.** Attachment context first (named-sibling traversal:
   is this block's next named sibling a definition? is the block inside a
   body?), then RED002 (patterns), then RED001 (overlap), then STE001
   position-awareness, then the directive rename.
4. **Hook rewrite** per above (parallel-safe with 1–3).
5. **Docs pass**: README, CLAUDE-comment-style.md, hk recipe, agent-format
   message text.

## Implementation constraints to preserve

- **No autofix, ever** — findings report rule, span, guidance; never
  replacement prose. Unchanged from the handoff, still a hard constraint.
- **Masking length invariant** — same length in, same length out.
- **Ordering wrinkle for RED001:** masking blanks exactly the identifiers
  RED001 needs to compare against. RED rules must run against the original
  (unmasked) text and the attached node's identifiers; prose rules keep
  running on the masked text. Do not reorder the pipeline so masking runs
  first for everything.
- **Do not vendor the ASD-STE100 controlled dictionary** (licensing).
  Structural rules only. Keep the ste-lint MIT attribution in `rules.rs`.
- Exit-code translation stays: binary exits 1 on findings (hk/CI
  contract); only the wrapper speaks Stop-hook exit 2.

## Open items (not blocking the build)

- Which repo is the dogfood — pick at deployment time.
- RED001 overlap threshold — tune under real agent traffic.
- 23 MB binary, naive sentence segmentation, 8-entry abbreviation list —
  all explicitly accepted as-is for v1.
