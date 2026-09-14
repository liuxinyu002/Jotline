// 响应侧契约校验单测（design D4）：语义分发结果在发送前按契约 Schema 断言，
// 违约 → 500 错误信封（操作 / 状态码 / 违约字段）+ 日志显形。
// 构造方式：DI 组装引擎，注入「故意更严」的响应 Schema（必填字段语义内核不产出）
// ——经真实 handle() 管线触发违约分支，永久锚定校验机制本身。
import assert from "node:assert/strict";
import { createServer } from "node:http";
import test from "node:test";
import { ContractValidator } from "../src/ajv.ts";
import { MockEngine } from "../src/router.ts";
import { SseHub } from "../src/sse.ts";
import { SemanticState } from "../src/state.ts";
import type { OpenAPIV3_1 } from "../src/types.ts";

/** 构造「响应必含语义内核不产出的字段」的契约文档（list_projects 200）。 */
function strictDoc(): OpenAPIV3_1.Document {
  return {
    openapi: "3.1.0",
    info: { title: "test", version: "0.0.0" },
    paths: {
      "/api/projects": {
        get: {
          operationId: "list_projects",
          responses: {
            "200": {
              description: "列表",
              content: {
                "application/json": {
                  schema: {
                    type: "object",
                    properties: {
                      nonexistent_field: { type: "string" },
                      items: { type: "array" },
                      page: { type: "object" },
                    },
                    required: ["nonexistent_field", "items", "page"],
                  },
                },
              },
            },
          },
        },
      },
    },
  } as unknown as OpenAPIV3_1.Document;
}

test("响应侧校验：违约响应不送出 → 500 信封定位违约字段并记日志", async () => {
  const doc = strictDoc();
  const validator = new ContractValidator(doc);
  const logs: string[] = [];
  const sse = new SseHub();
  const state = new SemanticState((e) => sse.broadcast(e));
  const engine = new MockEngine(
    doc,
    new Map([["default", { name: "default", description: "", seed: {} }]]),
    validator,
    state,
    sse,
    (msg) => logs.push(msg),
  );

  const server = createServer((req, res) => {
    engine.handle(req, res).catch((err) => {
      throw err;
    });
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const addr = server.address();
  assert.ok(addr && typeof addr === "object");
  const url = `http://127.0.0.1:${addr.port}`;

  const res = await fetch(`${url}/api/projects`, {
    headers: { authorization: "Bearer dev-token" },
  });
  assert.equal(res.status, 500, "违约响应应替换为 500 信封");
  const envelope = (await res.json()) as {
    code: string;
    message: string;
    detail: { field: string; message: string }[];
  };
  assert.equal(envelope.code, "internal");
  assert.match(envelope.message, /list_projects 200/);
  assert.ok(
    envelope.detail.some(
      (d) =>
        d.field === "nonexistent_field" &&
        d.message.includes("nonexistent_field"),
    ),
    `detail 应定位到违约字段：${JSON.stringify(envelope.detail)}`,
  );
  assert.ok(
    logs.some((l) => l.includes("响应违反契约") && l.includes("list_projects")),
    `日志应记录违约详情：${JSON.stringify(logs)}`,
  );

  await new Promise<void>((resolve) => server.close(() => resolve()));
});
