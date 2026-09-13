#!/usr/bin/env bash
# Wire 纪律机器检查（SPEC §3.2 冻结项）：
#   禁 untagged enum、禁 serde(flatten)、禁字段级 camelCase rename。
#   命中即非零退出并指明违规位置（contract-generation spec：Wire 禁用清单的机器检查）。
set -euo pipefail
cd "$(dirname "$0")/.."

SRC="src-tauri/crates/contracts/src"

status=0

# untagged / flatten
if grep -rn -E '#\[serde\((.*,)?(untagged|flatten)' "$SRC" ; then
  echo "✗ Wire 纪律违规：untagged / flatten 禁用（tagged enum 与显式字段替代）" >&2
  status=1
fi

# camelCase rename（rename_all = "camelCase"，或 rename = "camelCaseValue" 形态）
# 「card.batch」「2.0」等非 camelCase 字面量不受影响。
if grep -rn -E 'rename(_all)?\s*=\s*"[a-z]+[A-Z][A-Za-z0-9]*"' "$SRC" ; then
  echo "✗ Wire 纪律违规：camelCase rename 禁用（字段命名一律 snake_case）" >&2
  status=1
fi
if grep -rn -E 'rename_all\s*=\s*"camelCase"' "$SRC" ; then
  echo "✗ Wire 纪律违规：rename_all = \"camelCase\" 禁用" >&2
  status=1
fi

if [[ $status -ne 0 ]]; then
  exit 1
fi
echo "✓ Wire 纪律 grep 通过"
