#!/usr/bin/env bash
# pre-commit hook（内环，秒级）：改动契约源或场景时强制跑 contracts:check。
# 零依赖安装：仅当暂存区涉及契约相关路径时才执行，避免无关提交被拖慢。
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

STAGED=$(git diff --cached --name-only)

if grep -qE '^(src-tauri/crates/(contracts|server)/|contracts/|scripts/|Cargo\.(toml|lock))' <<< "$STAGED"; then
  echo "〔pre-commit〕暂存改动涉及契约域，执行 contracts:check …"
  ./scripts/contracts-check.sh
else
  echo "〔pre-commit〕无契约域改动，跳过 contracts:check"
fi
