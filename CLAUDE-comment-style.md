# Comment style (enforced)

Comments in this repo are linted by `comment-lint`. A Stop hook runs it on
the files you edited and will not let a turn finish while findings remain.
Write comments this way the first time. Correcting them afterwards costs a
round trip.

## The prime rule: most comments should not exist

A comment that restates what the code already says is a defect (`RED001`).
A comment that narrates the editing process is a defect (`RED002`). Before
writing a comment, ask: does this say something the code cannot?

Rejected — narration and change-log residue:

```ruby
# Find the widget by slug.
find_widget_by_slug(slug)

# Now uses the connection pool. Fixed a race in the warm path.
```

Accepted — rationale the code cannot express:

```ruby
# The upstream API rejects concurrent lookups, so this stays serial.
find_widget_by_slug(slug)
```

Editing history belongs in the commit message, never in a comment. Never
write "now", "previously", "updated to", "as requested", or "this change".

## Prose rules

- **Doc comments** (directly above a `def`, `class`, or function):
  25 words maximum per sentence, 20 when the sentence starts with an
  imperative verb ("Set", "Return", "Check"). **Body comments** (inside a
  method) have no length cap — a rationale comment can take the words it
  needs.
- **Six sentences maximum per block.** A block is a run of consecutive
  comment lines at the same indentation; the checker joins them and counts
  across the whole run.
- **No semicolons.** Write two sentences.
- **No filler or vague intensifiers.** Avoid: `in order to`, `utilize`,
  `prior to`, `a number of`, `due to the fact that`, `obviously`, `simply`,
  `clearly`, `basically`, `seamless`, `robust`, `crucial`, `delve`,
  `moreover`, `additionally`.
- **Active voice, simple tenses.** "The initializer loads the config",
  not "the config is loaded by the initializer".

Identifiers, backticked spans, and URLs are masked before checking, so
`Foo::Bar.baz`, `snake_case`, and links never trigger prose findings. You
do not need to avoid them.

## When a finding fires

- `RED001`/`RED002`: delete the comment, or replace it with rationale the
  code cannot express. Deletion is usually correct.
- `STE` rules: rewrite the comment. Do not delete it, and do not strip it
  down to something uninformative — split it into two sentences instead.

## Escape hatch

If a finding genuinely does not apply, suppress it on the line above the
block and say why:

```ruby
# comment-lint:ignore STE001 -- quoted verbatim from the upstream RFC
```

Bare `# comment-lint:ignore` suppresses every rule for the next block. Use
either form sparingly. Do not suppress a finding to make the hook pass
when rewriting the sentence would work.
