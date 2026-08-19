#!/usr/bin/env bash
# .claude/hooks/comment-lint-stop.sh
#
# Stop hook: blocks the end of an agent turn while comment-lint reports
# findings, so the agent rewrites its own comments before finishing.
#
# Exit code semantics for a Stop hook:
#   0  allow the turn to end
#   2  BLOCK the turn from ending; stderr is fed back to the agent
#   1  NON-BLOCKING error -- shows a hook-error notice and continues.
#      Do not use 1 here. comment-lint exits 1 on findings, so this
#      wrapper must translate that into 2.
#
# Correction rounds: a fresh Stop episode (stop_hook_active=false) starts
# a round counter at 0. Each time the hook blocks (exit 2) it increments
# a per-session counter file. Once the counter reaches 3, the hook stops
# blocking even if findings remain -- an agent that hasn't fixed things
# in 3 tries is unlikely to on a 4th, and blocking forever would hang
# the session. It emits a systemMessage so the user knows findings may
# remain, then exits 0.
#
# Scoping: files are read from the transcript's tool_use history (Edit,
# Write, MultiEdit, NotebookEdit) rather than from `git diff`, since a
# turn's edits may already be committed or otherwise invisible to git
# by the time Stop fires. If the transcript can't be read, fall back to
# the old dirty-files scoping.
#
# Install under .claude/hooks/, not as a plugin: plugin-installed Stop
# hooks have a known bug where exit 2 halts instead of continuing.
set -uo pipefail

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

INPUT=$(cat)

HAVE_JQ=0
command -v jq >/dev/null 2>&1 && HAVE_JQ=1

EXT_PATTERN='\.(rb|rake|gemspec|js|jsx|mjs|cjs|ts|tsx)$'

if [ "$HAVE_JQ" -eq 1 ]; then
  TRANSCRIPT_PATH=$(printf '%s' "$INPUT" | jq -r '.transcript_path // empty')
  STOP_HOOK_ACTIVE=$(printf '%s' "$INPUT" | jq -r '.stop_hook_active // false')
  SESSION_ID=$(printf '%s' "$INPUT" | jq -r '.session_id // empty')
else
  TRANSCRIPT_PATH=$(printf '%s' "$INPUT" | grep -o '"transcript_path"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed -E 's/.*:[[:space:]]*"([^"]*)"/\1/')
  case "$INPUT" in *'"stop_hook_active":true'*|*'"stop_hook_active": true'*) STOP_HOOK_ACTIVE=true ;; *) STOP_HOOK_ACTIVE=false ;; esac
  SESSION_ID=$(printf '%s' "$INPUT" | grep -o '"session_id"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed -E 's/.*:[[:space:]]*"([^"]*)"/\1/')
fi

# Without jq and without a session_id we can't safely run bounded rounds
# (session_id is needed to key the counter file); fall back to the old
# one-shot behavior rather than risk blocking forever.
if [ "$HAVE_JQ" -eq 0 ] && [ -z "$SESSION_ID" ]; then
  if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
    exit 0
  fi
fi

# --- File scoping -----------------------------------------------------
# NUL-delimited paths must go through a file, not a command substitution:
# $(...) strips null bytes and would concatenate every path into one.
FILES=$(mktemp)
RAW_LIST=$(mktemp)
trap 'rm -f "$FILES" "$RAW_LIST"' EXIT

SCOPED_FROM_TRANSCRIPT=0
if [ "$HAVE_JQ" -eq 1 ] && [ -n "$TRANSCRIPT_PATH" ] && [ -r "$TRANSCRIPT_PATH" ]; then
  jq -R 'fromjson? | .message.content[]? | select(.type=="tool_use") | select(.name=="Edit" or .name=="Write" or .name=="MultiEdit" or .name=="NotebookEdit") | .input.file_path // empty' \
    "$TRANSCRIPT_PATH" 2>/dev/null | jq -r 'select(. != null)' 2>/dev/null > "$RAW_LIST"
  SCOPED_FROM_TRANSCRIPT=1
fi

if [ "$SCOPED_FROM_TRANSCRIPT" -eq 1 ]; then
  sort -u "$RAW_LIST" | while IFS= read -r f; do
    [ -z "$f" ] && continue
    [ -f "$f" ] || continue
    case "$f" in
      *.rb|*.rake|*.gemspec|*.js|*.jsx|*.mjs|*.cjs|*.ts|*.tsx) printf '%s\0' "$f" ;;
    esac
  done > "$FILES"
else
  {
    git diff --name-only -z
    git diff --cached --name-only -z
    git ls-files --others --exclude-standard -z
  } 2>/dev/null | tr '\0' '\n' | grep -E "$EXT_PATTERN" | sort -u | while IFS= read -r f; do
    [ -f "$f" ] && printf '%s\0' "$f"
  done > "$FILES"
fi

[ ! -s "$FILES" ] && exit 0

# --- Bounded correction rounds -----------------------------------------
ROUNDS_FILE=""
ROUNDS=0
if [ -n "$SESSION_ID" ]; then
  ROUNDS_FILE="${TMPDIR:-/tmp}/comment-lint-rounds-${SESSION_ID}"
  if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
    if [ -f "$ROUNDS_FILE" ]; then
      ROUNDS=$(cat "$ROUNDS_FILE" 2>/dev/null)
      case "$ROUNDS" in ''|*[!0-9]*) ROUNDS=0 ;; esac
    fi
  else
    rm -f "$ROUNDS_FILE"
    ROUNDS=0
  fi

  if [ "$ROUNDS" -ge 3 ]; then
    rm -f "$ROUNDS_FILE"
    printf '%s\n' '{"systemMessage":"comment-lint: findings may remain after 3 correction rounds; giving up"}'
    exit 0
  fi
fi

# --- Resolve the binary -------------------------------------------------
BIN="${COMMENT_LINT_BIN:-}"
if [ -z "$BIN" ]; then
  export PATH="$PATH:$HOME/.cargo/bin"
  BIN=$(command -v comment-lint 2>/dev/null)
fi

if [ -z "$BIN" ]; then
  printf '%s\n' '{"systemMessage":"comment-lint unavailable or failed (exit 127); comments were not checked"}'
  exit 0
fi

# --- Run the linter -------------------------------------------------
OUT=$("$BIN" \
  --files0-from - \
  --rules "${STE_RULES:-RED001,RED002,STE001,STE002,STE006,STE007}" \
  --format agent < "$FILES" 2>/dev/null)
STATUS=$?

# 0 = clean, 1 = findings. Anything else is a tool failure -- never
# block the agent because the linter itself broke, but say so loudly.
if [ "$STATUS" -gt 1 ]; then
  printf '{"systemMessage":"comment-lint unavailable or failed (exit %s); comments were not checked"}\n' "$STATUS"
  [ -n "$ROUNDS_FILE" ] && rm -f "$ROUNDS_FILE"
  exit 0
fi

if [ "$STATUS" -eq 0 ]; then
  [ -n "$ROUNDS_FILE" ] && rm -f "$ROUNDS_FILE"
  exit 0
fi

# STATUS -eq 1: findings.
if [ -n "$ROUNDS_FILE" ]; then
  printf '%s\n' "$((ROUNDS + 1))" > "$ROUNDS_FILE"
fi
printf '%s\n' "$STDOUT_OUT" >&2
exit 2
