#!/usr/bin/env bash
# tests/hook/run.sh
#
# Plain-assert test harness for hooks/comment-lint-stop.sh. No framework:
# each case sets up a sandbox, invokes the hook, and asserts on exit
# code / stdout / stderr / side files. Exits non-zero on first failure.
set -uo pipefail

HOOK="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/hooks/comment-lint-stop.sh"

SANDBOX=""
cleanup() { [ -n "$SANDBOX" ] && rm -rf "$SANDBOX"; }
trap cleanup EXIT

PASS_COUNT=0
FAIL_COUNT=0

fail() {
  echo "FAIL: $CASE_NAME -- $*"
  exit 1
}

pass() {
  echo "PASS: $CASE_NAME"
  PASS_COUNT=$((PASS_COUNT + 1))
}

# Sets up a fresh sandbox dir with a stub comment-lint binary.
# STUB_EXIT controls the stub's exit code; the stub prints "FINDINGS" to
# stdout when STUB_EXIT=1. Every invocation is recorded (one line) to
# $SANDBOX/invocations, and the NUL-delimited stdin it received is
# recorded to $SANDBOX/last-stdin-nul-count as a record count.
new_sandbox() {
  SANDBOX=$(mktemp -d)
  cat > "$SANDBOX/comment-lint" <<'STUB'
#!/usr/bin/env bash
echo "invoked" >> "$SANDBOX_INVOCATIONS"
tr '\0' '\n' < /dev/stdin | grep -c . > "$SANDBOX_STDIN_COUNT" || true
if [ "${STUB_EXIT:-0}" = "1" ]; then
  echo "FINDINGS"
  exit 1
fi
exit "${STUB_EXIT:-0}"
STUB
  chmod +x "$SANDBOX/comment-lint"
  touch "$SANDBOX/invocations"
  export SANDBOX_INVOCATIONS="$SANDBOX/invocations"
  export SANDBOX_STDIN_COUNT="$SANDBOX/stdin-count"
}

# Writes fixture files (relative to sandbox) and a transcript JSONL that
# references them via Edit/Write tool_use entries, plus one garbage line.
write_transcript() {
  local transcript="$1"
  shift
  {
    echo 'not valid json at all {{{'
    for f in "$@"; do
      printf '{"message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"%s"}}]}}\n' "$f"
    done
  } > "$transcript"
}

run_hook() {
  local stdin_json="$1"
  printf '%s' "$stdin_json" | COMMENT_LINT_BIN="${COMMENT_LINT_BIN:-}" "$HOOK"
}

# --- Case a: clean lint -> exit 0, no systemMessage --------------------
CASE_NAME="a-clean-lint-exit0"
new_sandbox
cd "$SANDBOX" || exit 1
mkdir -p src
echo 'puts "hi"' > src/a.rb
write_transcript "$SANDBOX/transcript.jsonl" "$SANDBOX/src/a.rb"
set +e
STDOUT=$(printf '{"session_id":"sess-a","transcript_path":"%s","stop_hook_active":false,"cwd":"%s","hook_event_name":"Stop"}' "$SANDBOX/transcript.jsonl" "$SANDBOX" | \
  env COMMENT_LINT_BIN="$SANDBOX/comment-lint" STUB_EXIT=0 SANDBOX_INVOCATIONS="$SANDBOX/invocations" SANDBOX_STDIN_COUNT="$SANDBOX/stdin-count" "$HOOK" 2>"$SANDBOX/stderr.out")
STATUS=$?
set -e
[ "$STATUS" -eq 0 ] || fail "expected exit 0, got $STATUS"
case "$STDOUT" in *systemMessage*) fail "unexpected systemMessage in clean path: $STDOUT" ;; esac
pass

# --- Case b: findings, fresh episode -> exit 2, counter=1 --------------
CASE_NAME="b-findings-fresh-exit2-counter1"
new_sandbox
cd "$SANDBOX" || exit 1
mkdir -p src
echo 'puts "hi"' > src/a.rb
write_transcript "$SANDBOX/transcript.jsonl" "$SANDBOX/src/a.rb"
ROUNDS_FILE="${TMPDIR:-/tmp}/comment-lint-rounds-sess-b"
rm -f "$ROUNDS_FILE"
set +e
STDOUT=$(printf '{"session_id":"sess-b","transcript_path":"%s","stop_hook_active":false,"cwd":"%s","hook_event_name":"Stop"}' "$SANDBOX/transcript.jsonl" "$SANDBOX" | \
  env COMMENT_LINT_BIN="$SANDBOX/comment-lint" STUB_EXIT=1 SANDBOX_INVOCATIONS="$SANDBOX/invocations" SANDBOX_STDIN_COUNT="$SANDBOX/stdin-count" "$HOOK" 2>"$SANDBOX/stderr.out")
STATUS=$?
set -e
[ "$STATUS" -eq 2 ] || fail "expected exit 2, got $STATUS"
grep -q FINDINGS "$SANDBOX/stderr.out" || fail "stderr missing FINDINGS: $(cat "$SANDBOX/stderr.out")"
[ -f "$ROUNDS_FILE" ] || fail "rounds file not created"
[ "$(cat "$ROUNDS_FILE")" = "1" ] || fail "expected counter 1, got $(cat "$ROUNDS_FILE")"
rm -f "$ROUNDS_FILE"
pass

# --- Case c: findings, stop_hook_active=true, counter starts at 2 ------
CASE_NAME="c-findings-active-counter2to3"
new_sandbox
cd "$SANDBOX" || exit 1
mkdir -p src
echo 'puts "hi"' > src/a.rb
write_transcript "$SANDBOX/transcript.jsonl" "$SANDBOX/src/a.rb"
ROUNDS_FILE="${TMPDIR:-/tmp}/comment-lint-rounds-sess-c"
printf '2' > "$ROUNDS_FILE"
set +e
STDOUT=$(printf '{"session_id":"sess-c","transcript_path":"%s","stop_hook_active":true,"cwd":"%s","hook_event_name":"Stop"}' "$SANDBOX/transcript.jsonl" "$SANDBOX" | \
  env COMMENT_LINT_BIN="$SANDBOX/comment-lint" STUB_EXIT=1 SANDBOX_INVOCATIONS="$SANDBOX/invocations" SANDBOX_STDIN_COUNT="$SANDBOX/stdin-count" "$HOOK" 2>"$SANDBOX/stderr.out")
STATUS=$?
set -e
[ "$STATUS" -eq 2 ] || fail "expected exit 2, got $STATUS"
[ "$(cat "$ROUNDS_FILE")" = "3" ] || fail "expected counter 3, got $(cat "$ROUNDS_FILE")"
rm -f "$ROUNDS_FILE"
pass

# --- Case d: findings, stop_hook_active=true, counter already 3 --------
CASE_NAME="d-findings-active-counter3-giveup"
new_sandbox
cd "$SANDBOX" || exit 1
mkdir -p src
echo 'puts "hi"' > src/a.rb
write_transcript "$SANDBOX/transcript.jsonl" "$SANDBOX/src/a.rb"
ROUNDS_FILE="${TMPDIR:-/tmp}/comment-lint-rounds-sess-d"
printf '3' > "$ROUNDS_FILE"
set +e
STDOUT=$(printf '{"session_id":"sess-d","transcript_path":"%s","stop_hook_active":true,"cwd":"%s","hook_event_name":"Stop"}' "$SANDBOX/transcript.jsonl" "$SANDBOX" | \
  env COMMENT_LINT_BIN="$SANDBOX/comment-lint" STUB_EXIT=1 SANDBOX_INVOCATIONS="$SANDBOX/invocations" SANDBOX_STDIN_COUNT="$SANDBOX/stdin-count" "$HOOK" 2>"$SANDBOX/stderr.out")
STATUS=$?
set -e
[ "$STATUS" -eq 0 ] || fail "expected exit 0, got $STATUS"
case "$STDOUT" in *systemMessage*rounds*|*rounds*systemMessage*) : ;; *) fail "expected systemMessage mentioning rounds, got: $STDOUT" ;; esac
[ -f "$ROUNDS_FILE" ] && fail "rounds file should have been removed"
pass

# --- Case e: binary unavailable -> exit 0, systemMessage "unavailable" -
CASE_NAME="e-binary-unavailable"
new_sandbox
cd "$SANDBOX" || exit 1
mkdir -p src
echo 'puts "hi"' > src/a.rb
write_transcript "$SANDBOX/transcript.jsonl" "$SANDBOX/src/a.rb"
set +e
STDOUT=$(printf '{"session_id":"sess-e","transcript_path":"%s","stop_hook_active":false,"cwd":"%s","hook_event_name":"Stop"}' "$SANDBOX/transcript.jsonl" "$SANDBOX" | \
  env COMMENT_LINT_BIN="$SANDBOX/does-not-exist" "$HOOK" 2>"$SANDBOX/stderr.out")
STATUS=$?
set -e
[ "$STATUS" -eq 0 ] || fail "expected exit 0, got $STATUS"
case "$STDOUT" in *unavailable*) : ;; *) fail "expected 'unavailable' in stdout, got: $STDOUT" ;; esac
pass

# --- Case f: zero supported files -> exit 0, stub not invoked ----------
CASE_NAME="f-no-supported-files-stub-not-called"
new_sandbox
cd "$SANDBOX" || exit 1
mkdir -p src
echo 'unsupported' > src/a.txt
write_transcript "$SANDBOX/transcript.jsonl" "$SANDBOX/src/a.txt"
set +e
STDOUT=$(printf '{"session_id":"sess-f","transcript_path":"%s","stop_hook_active":false,"cwd":"%s","hook_event_name":"Stop"}' "$SANDBOX/transcript.jsonl" "$SANDBOX" | \
  env COMMENT_LINT_BIN="$SANDBOX/comment-lint" STUB_EXIT=0 SANDBOX_INVOCATIONS="$SANDBOX/invocations" SANDBOX_STDIN_COUNT="$SANDBOX/stdin-count" "$HOOK" 2>"$SANDBOX/stderr.out")
STATUS=$?
set -e
[ "$STATUS" -eq 0 ] || fail "expected exit 0, got $STATUS"
[ -s "$SANDBOX/invocations" ] && fail "stub should not have been invoked"
pass

# --- Case g: transcript missing -> falls back to git scoping -----------
CASE_NAME="g-transcript-missing-git-fallback"
new_sandbox
cd "$SANDBOX" || exit 1
git init -q .
git config user.email test@example.com
git config user.name test
echo 'puts "hi"' > dirty.rb
git add dirty.rb
git commit -q -m init
echo 'puts "changed"' > dirty.rb
set +e
STDOUT=$(printf '{"session_id":"sess-g","transcript_path":"%s/does-not-exist.jsonl","stop_hook_active":false,"cwd":"%s","hook_event_name":"Stop"}' "$SANDBOX" "$SANDBOX" | \
  env COMMENT_LINT_BIN="$SANDBOX/comment-lint" STUB_EXIT=0 SANDBOX_INVOCATIONS="$SANDBOX/invocations" SANDBOX_STDIN_COUNT="$SANDBOX/stdin-count" "$HOOK" 2>"$SANDBOX/stderr.out")
STATUS=$?
set -e
[ "$STATUS" -eq 0 ] || fail "expected exit 0, got $STATUS"
[ -s "$SANDBOX/invocations" ] || fail "stub should have been invoked via git fallback"
pass

# --- Case h: NUL-safety, two files -> two NUL-delimited records --------
CASE_NAME="h-nul-safety-two-files"
new_sandbox
cd "$SANDBOX" || exit 1
mkdir -p src
echo 'puts "one"' > src/one.rb
echo 'puts "two"' > src/two.rb
write_transcript "$SANDBOX/transcript.jsonl" "$SANDBOX/src/one.rb" "$SANDBOX/src/two.rb"
set +e
STDOUT=$(printf '{"session_id":"sess-h","transcript_path":"%s","stop_hook_active":false,"cwd":"%s","hook_event_name":"Stop"}' "$SANDBOX/transcript.jsonl" "$SANDBOX" | \
  env COMMENT_LINT_BIN="$SANDBOX/comment-lint" STUB_EXIT=0 SANDBOX_INVOCATIONS="$SANDBOX/invocations" SANDBOX_STDIN_COUNT="$SANDBOX/stdin-count" "$HOOK" 2>"$SANDBOX/stderr.out")
STATUS=$?
set -e
[ "$STATUS" -eq 0 ] || fail "expected exit 0, got $STATUS"
[ "$(cat "$SANDBOX/stdin-count")" = "2" ] || fail "expected 2 NUL-delimited records, got $(cat "$SANDBOX/stdin-count" 2>/dev/null)"
pass

echo ""
echo "All $PASS_COUNT case(s) passed."
