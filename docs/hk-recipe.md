# Pre-commit gate via hk (optional, not shipped in v1)

v1 enforces on the agent surface only — the Claude Code Stop hook. The CLI
keeps the contract any commit gate needs (exit 0 clean / 1 findings,
`--files0-from -`, `--format jsonl`), so wiring a gate later is a paste-in.
Be aware a pre-commit gate lints *human* comments too, not just agent
output.

Add this to the target repo's `hk.pkl`, then run `hk validate` — the
snippet was written from the hk docs and has not been validated against a
live hk install:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.56.0/hk@1.56.0#/Config.pkl"

local comment_lint = new Step {
    // `types` matches by extension AND shebang, so extensionless Ruby
    // scripts are covered too. A `glob` alone would miss them.
    types = List("ruby", "javascript", "typescript")

    // No `fix` command: findings need a human rewrite or deletion, and
    // an auto-rewrite of prose is not something to run unattended.
    check = new Command {
        argv = List("comment-lint", "--rules", "RED001,RED002,STE001,STE002,STE006,STE007", "{{files}}")
    }

    output_summary = "stdout"

    tests {
        ["flags a wordy phrase"] {
            run = "check"
            write {
                ["{{tmp}}/a.rb"] = "# In order to boot we utilize the pool.\nx = 1\n"
            }
            expect {
                code = 1
                stdout = "STE006"
            }
        }
        ["ignores code-heavy comments"] {
            run = "check"
            write {
                ["{{tmp}}/b.rb"] = "# Call `Widget#find_by_slug` to get the record.\nx = 1\n"
            }
            expect { code = 0 }
        }
    }
}

hooks {
    ["pre-commit"] {
        steps { ["comment-lint"] = comment_lint }
    }
    // `hk check` and manual invocation both resolve here.
    ["check"] {
        steps { ["comment-lint"] = comment_lint }
    }
}
```

Checklist when enabling:

1. `hk validate` — confirm the `Command { argv = ... }` form and the
   standalone `{{files}}` expansion against the hk version in use.
2. `hk check` on a file with a known finding; expect exit 1 and the rule
   code on stdout.
3. Decide whether humans should get the same rule set as the agent —
   `RED001`/`RED002` were tuned against agent output. Dropping them for
   the commit gate (`--rules STE001,STE002,STE006,STE007`) is reasonable.

Do not reimplement the rules in another tool (e.g. a RuboCop cop) — two
implementations of the same checks drift. If a cop is ever added, give it
rules this binary does not own.
