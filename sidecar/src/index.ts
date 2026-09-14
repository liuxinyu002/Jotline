// sidecar 骨架（Phase-2，sidecar-runtime spec）：stdin 逐行 JSON-RPC（health.ping → 同 id pong）
// + ping-pong 成功后经 HTTP 回调主进程（ADR-2 通道验证，标识头 X-Jotline-Sidecar: boot）。
//
// 纪律（design D6）：
// - stdout 仅承载 JSON-RPC 消息（IPC 专用通道）——禁用 console.log；
// - 全部日志走 stderr，NDJSON 形态（{"ts","level","msg",…}），由主进程汇聚渲染；
// - 回调目标（端口 / token）经 spawn 环境变量注入，零契约变更（注册表仅 health.ping）；
// - 代码仅用 Node 兼容 API（readline / fetch），Bun 与 Node 均可直跑（Phase-8 起 Bun 编译单文件）。

import * as readline from "node:readline";
import type { IpcHealthRequest, IpcHealthResponse } from "@jotline/contracts";

/** stderr NDJSON 日志（stdout 为 IPC 专用通道，禁用 console.log）。 */
function log(
  level: "info" | "warn" | "error",
  msg: string,
  fields?: Record<string, unknown>,
): void {
  process.stderr.write(
    `${JSON.stringify({ ts: new Date().toISOString(), level, msg, ...fields })}\n`,
  );
}

/** 回调目标（主进程 spawn 时注入，design D6「配置不经协议通道」）。 */
const API_PORT = process.env.JOTLINE_API_PORT ?? "4765";
const DEV_TOKEN = process.env.JOTLINE_DEV_TOKEN ?? "dev-token";

/** boot 回调（仅一次）：ping-pong 成功后回调 `GET /api/projects`（单步限时 2s）。 */
let bootCallbackStarted = false;
function bootCallback(): void {
  if (bootCallbackStarted) return;
  bootCallbackStarted = true;
  const url = `http://127.0.0.1:${API_PORT}/api/projects`;
  log("info", "boot 回调开始", { url });
  fetch(url, {
    headers: {
      Authorization: `Bearer ${DEV_TOKEN}`,
      "X-Jotline-Sidecar": "boot",
    },
    signal: AbortSignal.timeout(2000),
  })
    .then((res) => {
      if (!res.ok) {
        log("warn", "boot 回调非 2xx", { status: res.status });
        return;
      }
      log("info", "boot 回调完成", { status: res.status });
    })
    .catch((err: unknown) => {
      log("warn", "boot 回调失败", { error: String(err) });
    });
}

const rl = readline.createInterface({ input: process.stdin });

rl.on("line", (line: string) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  let msg: unknown;
  try {
    msg = JSON.parse(trimmed);
  } catch {
    log("warn", "无法解析的 IPC 输入行", { line: trimmed.slice(0, 100) });
    return;
  }
  const req = msg as IpcHealthRequest;
  if (req.method === "health.ping" && typeof req.id === "string") {
    // pong：同一 id 的 JSON-RPC 成功响应（IpcHealthResponse 形态，PROTOCOL.md §7）
    const resp: IpcHealthResponse = {
      jsonrpc: "2.0",
      id: req.id,
      result: { status: "ok" },
    };
    process.stdout.write(`${JSON.stringify(resp)}\n`);
    log("info", "health.ping 已响应", { id: req.id });
    bootCallback();
  }
  // 未知方法：Phase-2 注册表仅 health.ping，静默忽略（不伪造错误响应）
});

log("info", "sidecar 已启动", { api_port: API_PORT });
