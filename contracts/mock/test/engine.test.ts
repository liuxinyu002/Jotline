// 引擎级单测（design D7）：buildEngine 组装引擎 → 临时 HTTP 服务（ephemeral 端口）
// → fetch 驱动真实 handle() 管线（鉴权 / 路由 / 剧本 / 语义分发）。
// 与 mock-smoke 职责分离：单测管形状与语义，冒烟管进程与协议。
import assert from "node:assert/strict";
import { createServer } from "node:http";
import test from "node:test";
import { buildEngine } from "../src/bootstrap.ts";

const AUTH = { authorization: "Bearer dev-token" };
const SEED_PROJECT = "prj_01J8Z3PRJ10000000000000000";

/** 组装引擎并挂到临时端口；返回基地址与关停函数。 */
async function startEngine(): Promise<{
  url: string;
  close: () => Promise<void>;
}> {
  const { engine, errors } = buildEngine(() => {});
  assert.ok(engine, `引擎组装失败：${JSON.stringify(errors)}`);
  const server = createServer((req, res) => {
    engine.handle(req, res).catch((err) => {
      throw err;
    });
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const addr = server.address();
  assert.ok(addr && typeof addr === "object");
  return {
    url: `http://127.0.0.1:${addr.port}`,
    close: () => new Promise<void>((resolve) => server.close(() => resolve())),
  };
}

test("种子播种：启动即含 default 场景种子（ULID / ISO-8601 形状）", async () => {
  const { url, close } = await startEngine();
  const res = await fetch(`${url}/api/projects`, { headers: AUTH });
  assert.equal(res.status, 200);
  const d = (await res.json()) as {
    items: Array<{ id: string; created_at: string }>;
    page: { total: number };
  };
  assert.ok(d.page.total >= 1, "default 场景应播种至少一个项目");
  for (const p of d.items) {
    assert.match(p.id, /^prj_[0-9A-HJKMNP-TV-Z]{26}$/, `ULID 形状：${p.id}`);
    assert.match(
      p.created_at,
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/,
      `ISO-8601 形状：${p.created_at}`,
    );
  }
  await close();
});

test("写后读一致：POST /api/notes → GET /api/notes/{id} 内容一致", async () => {
  const { url, close } = await startEngine();
  const create = await fetch(`${url}/api/notes`, {
    method: "POST",
    headers: { ...AUTH, "content-type": "application/json" },
    body: JSON.stringify({
      project_id: SEED_PROJECT,
      title: "单测写入",
      body: "写后读一致验证",
      tags: ["环境"],
    }),
  });
  assert.equal(create.status, 201);
  const created = (await create.json()) as { id: string; title: string };
  assert.match(created.id, /^itm_[0-9A-HJKMNP-TV-Z]{26}$/);

  const read = await fetch(`${url}/api/notes/${created.id}`, { headers: AUTH });
  assert.equal(read.status, 200);
  const note = (await read.json()) as {
    id: string;
    title: string;
    body: string;
    tags: string[];
  };
  assert.equal(note.id, created.id);
  assert.equal(note.title, "单测写入");
  assert.equal(note.body, "写后读一致验证");
  assert.deepEqual(note.tags, ["环境"]);
  await close();
});

test("SSE 广播：订阅期间写操作 → 收到 note_created 帧（event / id / data）", async () => {
  const { url, close } = await startEngine();
  const controller = new AbortController();
  const stream = await fetch(`${url}/api/stream`, {
    headers: AUTH,
    signal: controller.signal,
  });
  assert.equal(stream.status, 200);
  assert.match(stream.headers.get("content-type") ?? "", /^text\/event-stream/);

  // 订阅建立后执行写操作 → 广播帧应抵达本订阅者
  const create = await fetch(`${url}/api/notes`, {
    method: "POST",
    headers: { ...AUTH, "content-type": "application/json" },
    body: JSON.stringify({
      project_id: SEED_PROJECT,
      title: "广播源",
      body: "SSE 广播验证",
    }),
  });
  assert.equal(create.status, 201);

  const reader = stream.body?.getReader();
  assert.ok(reader, "SSE 响应应有 body 流");
  const decoder = new TextDecoder();
  let received = "";
  try {
    const deadline = Date.now() + 5_000;
    while (Date.now() < deadline) {
      const { value, done } = await reader.read();
      if (done) break;
      received += decoder.decode(value, { stream: true });
      if (received.includes("event: note_created")) break;
    }
  } finally {
    controller.abort();
  }
  assert.ok(
    received.includes(": connected"),
    "订阅建立应收到 connected 注释行",
  );
  assert.ok(received.includes("event: note_created\n"), "广播帧应含 event 行");
  const idLine = received.match(/^id: (\d+)$/m);
  assert.ok(idLine, "广播帧应含 id 行");
  assert.equal(idLine[1], "1", "首个广播 event_id 应为 1（单调递增起点）");
  const dataLine = received.match(/^data: (\{.*\})$/m);
  assert.ok(dataLine, "广播帧应含 data 行");
  const envelope = JSON.parse(dataLine[1]) as {
    type: string;
    event_id: number;
    payload: { title: string };
  };
  assert.equal(envelope.type, "note_created");
  assert.equal(envelope.payload.title, "广播源");
  await close();
});

test("剧本覆写优先级：请求级场景的剧本层优先于语义内核且不改全局", async () => {
  const { url, close } = await startEngine();
  // auth-401 场景：正确 token 下剧本注入 401（优先于语义内核的 200）
  const injected = await fetch(`${url}/api/projects`, {
    headers: { ...AUTH, "x-mock-scenario": "auth-401" },
  });
  assert.equal(injected.status, 401);
  const envelope = (await injected.json()) as { code: string; message: string };
  assert.equal(envelope.code, "unauthorized");
  assert.match(envelope.message, /auth-401/);

  // 请求级场景不改变全局激活场景（后续请求回到 default 的语义行为）
  const after = await fetch(`${url}/api/projects`, { headers: AUTH });
  assert.equal(after.status, 200);
  await close();
});

test("upsert 更新保留 created_at：更新响应与后续 list 均含 created_at", async () => {
  const { url, close } = await startEngine();
  // 更新种子阶段（upsert 携带 id → 更新路径）
  const update = await fetch(`${url}/api/stages`, {
    method: "POST",
    headers: { ...AUTH, "content-type": "application/json" },
    body: JSON.stringify({
      id: "stg_01J8Z3STG10000000000000000",
      project_id: "prj_01J8Z3PRJ10000000000000000",
      name: "项目启动（更名）",
      position: 1,
    }),
  });
  assert.equal(
    update.status,
    200,
    "更新应返回 200（若为 500 说明响应违反契约——created_at 丢失）",
  );
  const stage = (await update.json()) as { created_at?: string };
  assert.ok(
    stage.created_at,
    "更新响应必含 created_at（契约必填，更新路径不得丢失）",
  );

  // 更新后 list 响应同样保留 created_at
  const list = await fetch(
    `${url}/api/stages?project_id=prj_01J8Z3PRJ10000000000000000`,
    { headers: AUTH },
  );
  assert.equal(list.status, 200);
  const listed = (await list.json()) as {
    items: Array<{ id: string; created_at?: string }>;
  };
  const updated = listed.items.find(
    (s) => s.id === "stg_01J8Z3STG10000000000000000",
  );
  assert.ok(updated, "list 应含被更新阶段");
  assert.ok(updated.created_at, "更新后 list 响应必含 created_at");
  await close();
});
