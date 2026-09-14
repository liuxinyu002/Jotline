#!/usr/bin/env node
// dev 编排（design D7，零新依赖——仅 Node 内置模块）：
//   手写 parse .env → 并行 spawn（cargo run -p server 与 pnpm --filter app dev，CWD 一律仓库根）
//   → 就绪探测（轮询 4765 带 token 请求 + 1420 页面）→ 打印两端地址 → SIGINT 全链 kill
//
// sidecar 不由编排管理：其生命周期归主进程（架构定式，ADR-2）。
// 「三进程」指运行时结果（主进程 + 前端 + 主进程拉起的 sidecar），非编排直管三个。

import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const DEFAULTS = {
  port: 4765,
  token: "dev-token",
  dataDir: "./vault",
  webPort: 1420,
};

/** 手写 parse .env（格式同 .env.example：KEY=VALUE，# 注释，可选引号包裹）。 */
function parseDotEnv(file) {
  const env = {};
  if (!existsSync(file)) return env;
  for (const line of readFileSync(file, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq < 0) continue;
    const key = trimmed.slice(0, eq).trim();
    let value = trimmed.slice(eq + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    env[key] = value;
  }
  return env;
}

const dotEnv = parseDotEnv(resolve(ROOT, ".env"));
const port = Number(dotEnv.JOTLINE_API_PORT ?? DEFAULTS.port);
const token = dotEnv.JOTLINE_DEV_TOKEN ?? DEFAULTS.token;
const dataDir = dotEnv.JOTLINE_DATA_DIR ?? DEFAULTS.dataDir;

/** 子进程（detached = 独立进程组，便于整组 kill 不留孤儿）。 */
function launch(name, command, args, env) {
  const child = spawn(command, args, {
    cwd: ROOT, // CWD 锚定仓库根：./vault 相对路径语义稳定（README「本地开发」注意项）
    env: { ...process.env, ...env },
    stdio: ["ignore", "pipe", "pipe"],
    detached: true,
  });
  const prefix = `[${name}]`;
  const pipe = (from, to) => {
    from.setEncoding("utf8");
    let buf = "";
    from.on("data", (chunk) => {
      buf += chunk;
      let idx;
      while ((idx = buf.indexOf("\n")) >= 0) {
        to.write(`${prefix} ${buf.slice(0, idx + 1)}`);
        buf = buf.slice(idx + 1);
      }
    });
    from.on("end", () => {
      if (buf) to.write(`${prefix} ${buf}\n`);
    });
  };
  pipe(child.stdout, process.stdout);
  pipe(child.stderr, process.stderr);
  return child;
}

console.log(`[dev] 数据目录：${resolve(ROOT, dataDir)}`);
console.log("[dev] 拉起主进程（cargo run -p server）与前端（pnpm --filter app dev）…");

const core = launch(
  "core",
  "cargo",
  ["run", "-p", "server", "--bin", "jotline-server"],
  { JOTLINE_DEV_TOKEN: token, JOTLINE_DATA_DIR: dataDir, JOTLINE_API_PORT: String(port) },
);
const web = launch(
  "web",
  "pnpm",
  ["--filter", "app", "dev"],
  { VITE_DEV_TOKEN: token, VITE_API_BASE_URL: `http://127.0.0.1:${port}` },
);

const children = [
  { name: "core", child: core },
  { name: "web", child: web },
];

/** 就绪探测：轮询直至 ok 或超时（毫秒）。 */
async function waitReady(label, probe, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    try {
      if (await probe()) return;
    } catch {
      // 连接失败 = 未就绪，继续轮询
    }
    if (Date.now() > deadline) {
      throw new Error(`${label} 在 ${Math.round(timeoutMs / 1000)}s 内未就绪`);
    }
    await new Promise((r) => setTimeout(r, 500));
  }
}

async function probeApi() {
  const res = await fetch(`http://127.0.0.1:${port}/api/projects`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  return res.ok;
}

async function probeWeb() {
  const res = await fetch(`http://127.0.0.1:${DEFAULTS.webPort}/`);
  return res.ok;
}

// 主进程给足编译时间（首次全量构建较慢）；前端 dev server 快
const CORE_TIMEOUT_MS = 300_000;
const WEB_TIMEOUT_MS = 60_000;

try {
  await Promise.all([
    waitReady(`主进程（127.0.0.1:${port}）`, probeApi, CORE_TIMEOUT_MS),
    waitReady(`前端 dev server（127.0.0.1:${DEFAULTS.webPort}）`, probeWeb, WEB_TIMEOUT_MS),
  ]);
} catch (err) {
  console.error(`\n✗ 就绪探测失败：${err.message}`);
  console.error("  → 检查上方 [core] / [web] 日志定位原因（端口占用 / 编译失败 / 配置错误）");
  killAll();
  process.exit(1);
}

console.log("");
console.log(`✓ 就绪：领域 API http://127.0.0.1:${port} ｜ 前端 http://127.0.0.1:${DEFAULTS.webPort}`);
console.log("  （Ctrl-C 退出全部进程，含主进程拉起的 sidecar）");

function killAll() {
  for (const { name, child } of children) {
    if (child.exitCode !== null) continue;
    try {
      process.kill(-child.pid, "SIGINT"); // 整组信号：cargo 与其子进程一并退出
    } catch {
      try {
        child.kill("SIGINT");
      } catch {
        // 已退出
      }
    }
    console.log(`[dev] 已终止 ${name}（pid ${child.pid}）`);
  }
}

process.on("SIGINT", () => {
  console.log("\n[dev] 收到中断，退出全部子进程…");
  killAll();
  process.exit(0);
});

// 任一子进程意外退出 → 全链退出（避免半链假活）
for (const { name, child } of children) {
  child.on("exit", (code, signal) => {
    if (signal === "SIGINT") return; // killAll 触发的正常退出
    console.error(`[dev] ${name} 已退出（code=${code} signal=${signal}），退出全部进程`);
    killAll();
    process.exit(1);
  });
}

// 挂起主循环（保持 stdio 转发与信号处理）
setInterval(() => {}, 1 << 30);
