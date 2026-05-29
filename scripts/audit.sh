#!/usr/bin/env bash
# audit.sh — runs the BAD_RUST audit against this project.
# READ-ONLY: the edit-agent must not modify this file.

set -euo pipefail
cd "$(dirname "$0")/.."

AUDIT="$HOME/.claude/skills/autobuilder/rules/audit-checks.sh"
mkdir -p target/autobuilder/receipts

if [ ! -x "$AUDIT" ]; then
  echo "audit: skill audit-checks.sh not found at $AUDIT" >&2
  exit 1
fi

"$AUDIT" . > target/autobuilder/receipts/risk-gate.json
