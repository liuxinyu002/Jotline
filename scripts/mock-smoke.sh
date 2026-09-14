#!/usr/bin/env bash
# Mock 冒烟断言（CI contracts job 调用；Roadmap Phase-1 四条验证命令的服务端断言）。
# 前置：Mock 已在 127.0.0.1:4766 启动（pnpm --filter @jotline/mock run start）。
set -euo pipefail

BASE="http://127.0.0.1:4766"
AUTH="Authorization: Bearer dev-token"

# ① 200 + ULID / ISO-8601 抽查
curl -sf "$BASE/api/projects" -H "$AUTH" | python3 -c "
import json, re, sys
d = json.load(sys.stdin)
assert d['page']['total'] >= 1
p = d['items'][0]
assert re.match(r'^prj_[0-9A-HJKMNP-TV-Z]{26}$', p['id']), p['id']
assert re.match(r'^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$', p['created_at']), p['created_at']
print('① projects 200 + ULID/ISO-8601 OK')
"

# ② search snippet + provenance 命中
curl -sf "$BASE/api/search?q=oracle" -H "$AUTH" | python3 -c "
import json, sys
d = json.load(sys.stdin)
assert len(d) >= 1, 'oracle 应有命中'
assert 'snippet' in d[0] and 'provenance' in d[0] and 'score' in d[0]
print('② search snippet+provenance 命中 OK')
"

# ③ SSE 3 条事件后关闭
EVENTS=$(curl -sN --max-time 10 "$BASE/api/stream" -H "$AUTH" -H "X-Mock-Scenario: stream-demo" | grep -c '^event:' || true)
if [ "$EVENTS" != "3" ]; then
  echo "③ stream 事件数异常: $EVENTS（期望 3）" >&2
  exit 1
fi
echo "③ stream 3 条事件后关闭 OK"

# ④ 无 token → 401
CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/projects")
if [ "$CODE" != "401" ]; then
  echo "④ 无 token 状态码异常: $CODE（期望 401）" >&2
  exit 1
fi
echo "④ 无 token 401 OK"

# ⑤ SSE query token 等价形态（wire-protocol delta：EventSource 无法携带 header）
EVENTS_QT=$(curl -sN --max-time 10 "$BASE/api/stream?token=dev-token" -H "X-Mock-Scenario: stream-demo" | grep -c '^event:' || true)
if [ "$EVENTS_QT" != "3" ]; then
  echo "⑤ stream(query token) 事件数异常: $EVENTS_QT（期望 3，与 header 形态等价）" >&2
  exit 1
fi
echo "⑤ stream query token 等价 OK"

# ⑥ CORS 预检（跨源 :1420 → Mock，design D5 同步修复项）
PREFLIGHT=$(curl -s -o /dev/null -D - -X OPTIONS "$BASE/api/projects" \
  -H "Origin: http://localhost:1420" -H "Access-Control-Request-Method: GET")
echo "$PREFLIGHT" | grep -Eq '^HTTP/[0-9.]+ 204' || {
  echo "⑥ CORS 预检状态非 204：$(echo "$PREFLIGHT" | head -1)" >&2
  exit 1
}
echo "$PREFLIGHT" | grep -qi '^access-control-allow-origin: http://localhost:1420' || {
  echo "⑥ CORS 预检缺 allow-origin 回显" >&2
  exit 1
}
echo "⑥ CORS 预检 OK"

echo "✓ Mock 冒烟全部通过"
