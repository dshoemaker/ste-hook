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
# Install under .claude/hooks/, not as a plugin: plugin-installed Stop
# hooks have a known bug where exit 2 halts instead of continuing.
set -uo pipefail

INPUT=$(cat)

# Loop guard. Claude Code sets stop_hook_active=true when a previous Stop
# hook already kept the turn alive. Without this check a finding the agent
# cannot resolve will block forever.
if command -v jq >/dev/null 2>&1; then
  if [ "$(printf '%s' "$INPUT" | jq -r '.stop_hook_active // false')" = "true" ]; then
    exit 0
  fi
else
  case "$INPUT" in *'"stop_hook_active":true'*) exit 0 ;; esac
fi

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

# Only files this turn actually touched. Linting the whole repo would
# surface pre-existing debt the agent did not write and cannot be
# responsible for.
# NUL-delimited paths must go through a file, not a command substitution:
# $(...) strips null bytes and would concatenate every path into one.
FILES=$(mktemp)
trap 'rm -f "$FILES"' EXIT
{
  git diff --name-only -z
  git diff --cached --name-only -z
  git ls-files --others --exclude-standard -z
} > "$FILES" 2>/dev/null
[ ! -s "$FILES" ] && exit 0

OUT=$(comment-lint \
  --files0-from - \
  --rules "${STE_RULES:-STE001,STE002,STE006,STE007}" \
  --format agent < "$FILES" 2>/dev/null)
STATUS=$?

# 0 = clean. Anything above 1 is a tool failure, not a lint failure:
# never block the agent because the linter itself broke.
[ "$STATUS" -ne 1 ] && exit 0

printf '%s\n' "$OUT" >&2
exit 2
