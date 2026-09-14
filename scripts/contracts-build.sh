#!/usr/bin/env bash
# 契约构建（design D3 一条管线覆盖三端）：
#   1) Rust 契约源 → contracts/openapi.json + contracts/tools/*.schema.json（确定性键排序）
#   2) openapi-typescript → contracts/generated/src/schema.d.ts + 命名导出入口 index.ts
#   3) Mock 场景数据校验（场景文件按契约 Schema 校验，失败即红）
set -euo pipefail
cd "$(dirname "$0")/.."

# 1) Rust 侧生成
cargo run -q -p server --bin gen --locked

# 2) TS 侧组装
pnpm exec openapi-typescript contracts/openapi.json -o contracts/generated/src/schema.d.ts
node scripts/gen-ts-index.mjs

# 3) 场景数据校验（Mock 包提供；场景目录为空时跳过）
if compgen -G "contracts/mock/scenarios/*.json" > /dev/null; then
  pnpm --filter @jotline/mock run validate
else
  echo "（无场景文件，跳过场景校验）"
fi
