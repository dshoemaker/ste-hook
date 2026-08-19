# Suppression directive is `comment-lint:ignore`; rule IDs split by origin

The suppression directive is named after the tool (`comment-lint:ignore`,
RuboCop convention), not `ste:ignore` as originally prototyped. Rule IDs
split by provenance: STE001–007 for rules ported from or inspired by
ASD-STE100 writing rules, RED00x for the redundancy/narration family, which
is original to this tool and has nothing to do with the STE specification.

Why now: directives get committed into source files permanently, so the
syntax was only cheap to change before first deployment. Why split: the
README's trademark notice claims only "inspired by" status with ASD-STE100;
numbering original rules as STE008+ would misattribute them to the spec,
and `ste:ignore RED001` would be incoherent. Verbosity is acceptable
because suppressions are meant to be rare.
