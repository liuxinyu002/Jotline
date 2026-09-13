#!/usr/bin/env bash
# 契约防漂移检查（CI 内环 / pre-commit / GitHub Actions 共用）：
#   1) 重新生成全部契约产物
#   2) 生成物与已提交版本零 diff（修改契约源而不重新生成 → 显红）
#   3) Wire 纪律 grep（契约源禁 untagged / flatten / camelCase rename）
set -euo pipefail
cd "$(dirname "$0")/.."

# 1) 重新生成（在位覆盖）
./scripts/contracts-build.sh > /dev/null

# 2) 漂移检查：生成物目录（openapi.json / generated / tools）内任何修改或新增都算漂移。
#    注意范围不含 contracts/mock 与 contracts/PROTOCOL.md（手写物）。
if [[ -n "$(git status --porcelain -- contracts/openapi.json contracts/generated contracts/tools)" ]]; then
  echo "✗ 契约生成物与契约源不一致（漂移）：" >&2
  git --no-pager diff -- contracts/openapi.json contracts/generated contracts/tools >&2 || true
  git status --porcelain -- contracts/openapi.json contracts/generated contracts/tools >&2
  echo "  → 运行 pnpm contracts:build 重新生成并提交全部产物" >&2
  exit 1
fi

# 3) Wire 纪律 grep
./scripts/wire-grep.sh

echo "✓ contracts:check 通过（生成物零漂移 + Wire 纪律干净）"
