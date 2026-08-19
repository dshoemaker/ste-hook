# CONTEXT

Ubiquitous language for comment-lint. Glossary only — no implementation detail.

## Terms

**Comment block** — A run of consecutive line comments at the same
indentation, joined into one paragraph before any rule runs. The unit all
rules operate on. A trailing comment (code before it on the same line)
never joins a block.

**Doc comment** — A comment block attached to a definition (immediately
above a `def`, `class`, `module`, or function). Subject to word limits.

**Body comment** — A comment block inside a method or function body, not
attached to a definition. Exempt from word limits; subject to redundancy
and filler rules.

**Rationale comment** — A body comment stating a constraint or reason the
code cannot express itself. The category the linter must never punish.

**Narration comment** — A comment that restates what the adjacent code
already says, or narrates the editing process ("now uses X",
"as requested"). The primary bloat category; the linter's main target.

**Redundancy rule** — A rule that flags narration comments by comparing a
comment's content words against the identifiers of the code it is attached
to, or by matching change-log/narration phrasing.

**Blocking set** — The rules whose findings block (agent turn or commit).
Severity is effectively binary: a rule is blocking or invisible, because
only blocking findings are fed back to the agent.

**Masking** — Overwriting code-like spans (backticks, URLs, identifiers,
versions, calls) in a comment with same-length filler before rules run, so
prose rules never fire on code and every offset stays valid.

**Provenance map** — Per-byte mapping from a joined block back to the
original file line and column, so a finding in joined text reports a real
source location.

**Suppression directive** — `comment-lint:ignore` on the line above a
block. Scoped (`comment-lint:ignore STE001`) suppresses listed rules; bare
suppresses all rules for the next block. The agent's escape hatch that
makes blocking safe.

**Forcing function** — The Stop hook: blocks the end of an agent turn while
blocking findings remain, so the agent rewrites its own comments. Bounded:
after a fixed number of correction rounds the turn ends regardless.

**Rule family** — `STE00x`: prose rules ported from or inspired by
ASD-STE100. `RED00x`: redundancy rules (narration detection), original to
this tool and unrelated to the STE specification.

## Resolved decisions (session 2026-08-19)

- The goal is stopping LLM comment bloat; STE rules are a means, not the end.
- Redundancy rules (narration detection) are the primary layer and require
  attachment context (tree-sitter sibling traversal) — to be built.
- Word limits apply only to doc comments; body comments are exempt.
- Enforcement is the Stop hook alone; no PostToolUse hook, no pre-commit
  gate in v1 (hk wiring stays documented for later).
- The Stop hook lints only files the agent touched, discovered from the
  session transcript; dirty-files is the fallback, not the rule.
- The gate allows up to ~3 correction rounds per turn, then yields.
- Linter failure never blocks a turn, but is reported loudly to the user.
- Deployment is per-project opt-in, dogfooded in one repo first.
- Redundancy rules block from day one in the dogfood repo.
- Directive is `comment-lint:ignore`; STE001–007 keep their IDs; new
  redundancy rules use the RED prefix.
- Work order: characterization tests → tree-sitter 0.25 migration → new
  rules TDD. Hook rewrite proceeds in parallel.
