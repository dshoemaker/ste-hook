# Comment bloat is the goal; STE is the means

This tool started as an ASD-STE100-inspired prose linter, but the problem
it exists to solve is LLM comment bloat: narration comments that restate
the adjacent code, change-log residue ("now uses X"), and filler. STE-style
prose rules are retained only where they serve that goal. This is why word
limits (STE001) apply only to doc comments while body comments — where
rationale lives — are exempt, why the redundancy family (RED00x) is the
primary blocking layer, and why an otherwise STE-perfect four-word
narration comment is a finding while a 30-word rationale comment is not.
Do not "fix" the body-comment exemption by extending word limits everywhere;
the asymmetry is the design.

Rejected alternative: full STE compliance for all comments, human- and
agent-written. Rejected because the terseness rules punish rationale
comments — the one category worth keeping — hardest, and because compliant
prose can still be pure bloat.
