// sidecar 契约测试（sidecar-runtime spec；node --test 零新依赖——sidecar 仅用
// Node 兼容 API，Node 24 原生直跑 TS，与 Bun 运行时行为等价，design D6）。
//
// 覆盖：health.ping → 同 id pong（JSON-RPC 2.0）、stdout IPC 纯净、stderr NDJSON、
// env 注入的 HTTP 回调通道（Bearer token + X-Jotline-Sidecar: boot）、边界行为
// （未知方法静默忽略 / 畸形行不崩溃）。

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const SIDECAR_ENTRY = fileURLToPath(
  new URL("../src/index.ts", import.meta.url),
);

/** 流 → 逐行收集（NDJSON / 逐行 JSON 均适用）。 */
function collectLines(stream) {
  const lines = [];
  let buf = "";
  stream.setEncoding("utf8");
  stream.on("data", (chunk) => {
    buf += chunk;
    let idx = buf.indexOf("\n");
    while (idx >= 0) {
      lines.push(buf.slice(0, idx));
      buf = buf.slice(idx + 1);
      idx = buf.indexOf("\n");
    }
  });
  return lines;
}

/** 拉起 sidecar（回调目标指向 stub 主进程，避免触碰真实 4765）。 */
function startSidecar({ apiPort, token }) {
  const child = spawn(process.execPath, [SIDECAR_ENTRY], {
    env: {
      ...process.env,
      JOTLINE_API_PORT: String(apiPort),
      JOTLINE_DEV_TOKEN: token,
    },
    stdio: ["pipe", "pipe", "pipe"],
  });
  return {
    child,
    stdin: child.stdin,
    stdoutLines: collectLines(child.stdout),
    stderrLines: collectLines(child.stderr),
    async kill() {
      child.kill();
      await new Promise((resolve) => child.once("exit", resolve));
    },
  };
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** 等待下一行 stdout（超时即失败）。 */
async function nextStdoutLine(sc, timeoutMs = 5000) {
  const start = Date.now();
  while (sc.stdoutLines.length === 0) {
    if (sc.child.exitCode !== null) {
      throw new Error(`sidecar 提前退出（code=${sc.child.exitCode}）`);
    }
    if (Date.now() - start > timeoutMs) throw new Error("等待 stdout 超时");
    await sleep(10);
  }
  return sc.stdoutLines.shift();
}

/** 轮询直至条件成立（超时即失败）。 */
async function waitUntil(fn, what, timeoutMs = 5000) {
  const start = Date.now();
  while (!fn()) {
    if (Date.now() - start > timeoutMs) throw new Error(`等待超时：${what}`);
    await sleep(10);
  }
}

/** 断言双流纪律：stdout 每行均为 JSON-RPC；stderr 每行均为含 ts/level/msg 的 NDJSON。 */
function assertStreamDiscipline(sc) {
  for (const line of sc.stdoutLines) {
    if (line.trim() === "") continue;
    const msg = JSON.parse(line);
    assert.equal(msg.jsonrpc, "2.0", `stdout 非 JSON-RPC 行：${line}`);
    assert.ok(
      "id" in msg && "result" in msg,
      `stdout 响应缺 id/result：${line}`,
    );
  }
  for (const line of sc.stderrLines) {
    if (line.trim() === "") continue;
    const entry = JSON.parse(line);
    assert.ok(typeof entry.ts === "string", `stderr 行缺 ts：${line}`);
    assert.ok(typeof entry.level === "string", `stderr 行缺 level：${line}`);
    assert.ok(typeof entry.msg === "string", `stderr 行缺 msg：${line}`);
  }
}

/** 模拟主进程领域 API（记录请求头），供回调通道断言。 */
async function startStubApi() {
  const requests = [];
  const server = createServer((req, res) => {
    requests.push({
      url: req.url,
      authorization: req.headers.authorization,
      sidecarHeader: req.headers["x-jotline-sidecar"],
    });
    res.writeHead(200, { "content-type": "application/json" });
    res.end(
      JSON.stringify({ items: [], page: { total: 0, limit: 50, offset: 0 } }),
    );
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return {
    port: server.address().port,
    requests,
    async close() {
      await new Promise((resolve) => server.close(resolve));
    },
  };
}

test("health.ping → 同 id pong（JSON-RPC 2.0 信封）", async () => {
  const stub = await startStubApi();
  const sc = startSidecar({ apiPort: stub.port, token: "dev-token" });
  try {
    sc.stdin.write('{"jsonrpc":"2.0","id":"t1","method":"health.ping"}\n');
    const resp = JSON.parse(await nextStdoutLine(sc));
    assert.equal(resp.jsonrpc, "2.0");
    assert.equal(resp.id, "t1", "pong 必须同 id");
    assert.equal(resp.result.status, "ok");

    // 等回调与日志落定后核对双流纪律
    await waitUntil(
      () => sc.stderrLines.some((l) => l.includes("boot 回调完成")),
      "boot 回调完成日志",
    );
    assertStreamDiscipline(sc);
  } finally {
    await sc.kill();
    await stub.close();
  }
});

test("回调通道经 env 注入：Bearer token + X-Jotline-Sidecar: boot + 200", async () => {
  const stub = await startStubApi();
  const sc = startSidecar({ apiPort: stub.port, token: "harden-token" });
  try {
    sc.stdin.write('{"jsonrpc":"2.0","id":"c1","method":"health.ping"}\n');
    await nextStdoutLine(sc); // pong 之后 sidecar 发起回调

    await waitUntil(
      () => stub.requests.length === 1,
      "回调请求到达 stub 主进程",
    );
    const req = stub.requests[0];
    assert.equal(req.url, "/api/projects");
    assert.equal(
      req.authorization,
      "Bearer harden-token",
      "token 须经 env 注入",
    );
    assert.equal(req.sidecarHeader, "boot", "须携带来源标识头");

    // stderr 记录回调完成（NDJSON 结构由 assertStreamDiscipline 核对）
    await waitUntil(
      () => sc.stderrLines.some((l) => l.includes("boot 回调完成")),
      "boot 回调完成日志",
    );
    assertStreamDiscipline(sc);
  } finally {
    await sc.kill();
    await stub.close();
  }
});

test("边界：未知方法静默忽略、畸形行不崩溃（注册表保持仅 health.ping）", async () => {
  const stub = await startStubApi();
  const sc = startSidecar({ apiPort: stub.port, token: "dev-token" });
  try {
    // 未知方法：不得产生任何 stdout 响应
    sc.stdin.write('{"jsonrpc":"2.0","id":"u1","method":"future.method"}\n');
    // 畸形行：stderr warn，进程不得退出
    sc.stdin.write("not-json\n");
    await sleep(300);
    assert.equal(sc.stdoutLines.length, 0, `不应有响应：${sc.stdoutLines}`);
    assert.ok(
      sc.stderrLines.some((l) => l.includes("无法解析的 IPC 输入行")),
      `应有畸形行告警：${sc.stderrLines}`,
    );
    assert.equal(sc.child.exitCode, null, "进程不应崩溃");

    // 随后 ping 仍正常响应
    sc.stdin.write('{"jsonrpc":"2.0","id":"t2","method":"health.ping"}\n');
    const resp = JSON.parse(await nextStdoutLine(sc));
    assert.equal(resp.id, "t2");
    assert.equal(resp.result.status, "ok");
  } finally {
    await sc.kill();
    await stub.close();
  }
});
