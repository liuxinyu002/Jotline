#!/usr/bin/env bash
# dev 冒烟（Roadmap Phase-2 验证链的机械化断言，dev-orchestration spec）：
# 隔离临时数据目录启动 server bin → 创建 201 + ULID → 文件断言 → 索引行断言 →
# 检索命中断言 → 错 token 401 → 清理退出码 0；任一环节失败即非零退出并指明环节。
#
# 注意：占用 4765 端口——与 `pnpm dev` 同时运行会因端口冲突而失败。
set -euo pipefail
cd "$(dirname "$0")/.."

BASE="http://127.0.0.1:4765"
AUTH="Authorization: Bearer dev-token"
SEED_PROJECT="prj_01J8Z3A7B4C5D6E7F8G9H0JKMN"

DATA_DIR="$(mktemp -d "${TMPDIR:-/tmp}/jotline-dev-smoke.XXXXXX")"
LOG="$(mktemp "${TMPDIR:-/tmp}/jotline-dev-smoke-log.XXXXXX")"
SERVER_PID=""

cleanup() {
  local rc=$?
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill -INT "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$DATA_DIR" "$LOG"
  exit "$rc"
}
trap cleanup EXIT

fail() { echo "✗ 冒烟失败（环节：$1）：$2" >&2; exit 1; }
step() { echo "✓ $1"; }

echo "→ 构建 server bin（增量编译）"
cargo build -q -p server --bin jotline-server

echo "→ 隔离数据目录启动：$DATA_DIR"
JOTLINE_DATA_DIR="$DATA_DIR" target/debug/jotline-server >"$LOG" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 100); do
  if curl -sf -o /dev/null "$BASE/api/projects" -H "$AUTH" 2>/dev/null; then break; fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    fail "启动" "server 进程提前退出（日志：${LOG}）"
  fi
  sleep 0.2
done
curl -sf -o /dev/null "$BASE/api/projects" -H "$AUTH" 2>/dev/null \
  || fail "启动" "server 未在 20s 内就绪（${BASE}；日志：${LOG}）"
step "① 服务就绪（${BASE}，数据目录已初始化 + seed）"

# ① 创建 201 + ULID 响应
RESP="$(curl -sf -X POST "$BASE/api/notes" -H "$AUTH" -H "Content-Type: application/json" \
  -d "{\"project_id\":\"$SEED_PROJECT\",\"title\":\"切片验证\",\"body\":\"hello jotline\"}")" \
  || fail "创建" "POST /api/notes 非 2xx（项目 seed 是否就绪？）"
NOTE_ID="$(printf '%s' "$RESP" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("id",""))')"
echo "$NOTE_ID" | grep -Eq '^itm_[0-9A-HJKMNP-TV-Z]{26}$' \
  || fail "创建" "响应 ID 非 itm_ 前缀 ULID：$NOTE_ID"
printf '%s' "$RESP" | grep -Eq '"created_at":"[0-9]{4}-[0-9]{2}-[0-9]{2}T[^"]+Z"' \
  || fail "创建" "时间字段非 ISO-8601 UTC：$RESP"
step "② 创建 201 + ULID 响应（${NOTE_ID}）"

# ② 文件断言（真相源）
[ -f "$DATA_DIR/notes/$NOTE_ID.md" ] \
  || fail "文件" "真相源文件缺失：$DATA_DIR/notes/$NOTE_ID.md"
grep -q "hello jotline" "$DATA_DIR/notes/$NOTE_ID.md" \
  || fail "文件" "文件内容不含请求正文"
step "③ 真相源文件落盘（notes/$NOTE_ID.md 含请求正文）"

# ③ 索引行断言
INDEX_HITS="$(sqlite3 "$DATA_DIR/index.sqlite" "select id from items;" 2>/dev/null | grep -c "$NOTE_ID" || true)"
[ "$INDEX_HITS" = "1" ] || fail "索引" "items 表应有且仅有该笔记行（实际命中 $INDEX_HITS 行）"
step "④ SQLite 索引行存在（items）"

# ④ 检索命中断言
SEARCH_RESP="$(curl -sf "$BASE/api/search?q=hello" -H "$AUTH")" \
  || fail "检索" "GET /api/search?q=hello 非 2xx"
echo "$SEARCH_RESP" | grep -q "$NOTE_ID" \
  || fail "检索" "q=hello 未命中刚创建的笔记：$SEARCH_RESP"
step "⑤ 检索命中（q=hello → ${NOTE_ID}）"

# ⑤ 错 token 401
CODE="$(curl -s -o /dev/null -w '%{http_code}' "$BASE/api/projects" -H "Authorization: Bearer wrong")"
[ "$CODE" = "401" ] || fail "鉴权" "错 token 状态码 $CODE（期望 401）"
step "⑥ 错 token 401"

echo "✓ dev 冒烟全部通过（隔离数据目录已清理）"
