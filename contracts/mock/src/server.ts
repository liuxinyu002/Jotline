// Mock 服务入口：加载契约与场景 → 校验（失败拒绝启动）→ 监听 127.0.0.1:4766（MOCK_PORT 可覆盖）。
import {
  createServer,
  type IncomingMessage,
  type ServerResponse,
} from "node:http";
import { buildEngine } from "./bootstrap.ts";
import { DEV_TOKEN } from "./router.ts";

const PORT = Number(process.env.MOCK_PORT ?? 4766);

const { engine, errors } = buildEngine((msg) => console.log(`[mock] ${msg}`));
if (!engine || errors.length > 0) {
  console.error("✗ 场景数据校验失败，拒绝启动：");
  for (const e of errors) console.error(`  ${e.file}: ${e.message}`);
  process.exit(1);
}

const server = createServer((req, res) => {
  handleCors(req, res);
  if (res.writableEnded) return;
  engine.handle(req, res).catch((err) => {
    console.error("[mock] 内部错误：", err);
    if (!res.headersSent) {
      res.writeHead(500, { "content-type": "application/json; charset=utf-8" });
    }
    res.end(JSON.stringify({ code: "internal", message: "Mock 内部错误" }));
  });
});

server.listen(PORT, "127.0.0.1", () => {
  console.log(
    `[mock] 契约 Mock 已启动：http://127.0.0.1:${PORT}（Bearer ${DEV_TOKEN}）`,
  );
  console.log(
    `[mock] 激活场景：${engine.activeScenario}；控制端点：/_mock/scenarios`,
  );
});

// CORS（与真实主进程同约定，design D5）：允许 http://localhost:* / http://127.0.0.1:* 跨源；
// 预检（OPTIONS + Access-Control-Request-Method）直接应答 204，不进入路由。
function handleCors(req: IncomingMessage, res: ServerResponse): void {
  const origin = req.headers.origin;
  if (
    typeof origin !== "string" ||
    !(
      origin.startsWith("http://localhost:") ||
      origin.startsWith("http://127.0.0.1:")
    )
  ) {
    return;
  }
  res.setHeader("Access-Control-Allow-Origin", origin);
  res.setHeader("Vary", "Origin");
  if (
    req.method === "OPTIONS" &&
    typeof req.headers["access-control-request-method"] === "string"
  ) {
    res.setHeader("Access-Control-Allow-Methods", "GET,POST,PATCH,OPTIONS");
    res.setHeader("Access-Control-Allow-Headers", "authorization,content-type");
    res.writeHead(204);
    res.end();
  }
}
