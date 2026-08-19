# Comment style (enforced)

Comments in this repo are checked against a rule-based subset of ASD-STE100
Simplified Technical English. A Stop hook runs `comment-lint` on changed
files and will not let a turn finish while findings remain.

Write comments this way the first time. Correcting them afterwards costs a
round trip.

## Rules

- **One idea per sentence.** Descriptive sentences: 25 words maximum.
  Procedural sentences (those starting with an imperative verb such as
  "Set", "Return", "Check"): 20 words maximum.
- **Six sentences maximum per comment block.** A block is a run of
  consecutive comment lines at the same indentation; the checker joins them
  and counts sentences across the whole run, not per line.
- **No semicolons.** Write two sentences.
- **Active voice.** "The initializer loads the config", not "the config is
  loaded by the initializer".
- **Simple verb tenses.** Prefer "the pool holds" over "the pool has been
  holding" or "the pool will have held".
- **No `-ing` verb forms** except as a technical noun ("the mapping", "a
  warning").
- **`must` for an obligation, `can` for a possibility.** Not `shall`,
  `should`, `may`, or `might`.
- **No filler or vague intensifiers.** Avoid: `in order to`, `utilize`,
  `prior to`, `a number of`, `due to the fact that`, `obviously`, `simply`,
  `clearly`, `basically`, `seamless`, `robust`, `crucial`, `delve`,
  `moreover`, `additionally`.

## Examples

Rejected:

```ruby
# In order to utilize the connection pool the tenant must obviously be
# resolved first; this is being handled by the middleware.
```

Accepted:

```ruby
# The middleware resolves the tenant before the connection pool runs.
# Set `Current.tenant` first or the pool raises.
```

## Escape hatch

If a finding genuinely does not apply, suppress it on the line above the
block and say why:

```ruby
# ste:ignore STE001 -- quoted verbatim from the upstream RFC
```

Bare `# ste:ignore` suppresses every rule for the next block. Use either
form sparingly. Do not suppress a finding to make the hook pass when
rewriting the sentence would work.

## What not to do

- Do not delete a comment to clear a finding.
- Do not strip a comment down to something uninformative to get under the
  word limit. Split it into two sentences instead.
- Identifiers, backticked spans, and URLs are masked before checking, so
  `Foo::Bar.baz`, `snake_case`, and links never trigger findings. You do
  not need to avoid them.
