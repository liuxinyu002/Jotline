// 契约类型 round-trip 类型测试（spike S1/S2/S4 结论的持续验证载体）。
//
// 样例数据与 Rust 侧 cargo 单测（capture.rs / stream.rs）同源——均取自
// PRD §7.3 冻结样例与契约字段语义；本文件零运行时（typecheck 即测试通过）。
import type {
  CardResponse,
  CreateNoteRequest,
  IpcHealthRequest,
  IpcHealthResponse,
  NoteResponse,
  SearchQuery,
  StreamEnvelope,
} from "@jotline/contracts";
import { client, DEV_TOKEN } from "./api-client";

// ---------------------------------------------------------------------------
// S1：tagged enum（含 card.batch 点号 tag）→ TS union narrowing
// ---------------------------------------------------------------------------

const prdCardSample = {
  id: "crd_01J8Z3A7B4C5D6E7F8G9H0JKMN",
  status: "pending",
  content: {
    type: "card.batch",
    project: "prj_01J8Z3A7B4C5D6E7F8G9H0JKMN",
    confidence: "high",
    reason: "内容提及 AMS 系统 UAT 环境",
    entries: [
      {
        action: "create_note",
        title: "UAT 环境数据库连接问题处理",
        target_dir: "问题与变更/",
        format: "markdown",
        tags: ["环境", "数据"],
        group_name: "UAT 环境搭建",
        event: {
          type: "问题",
          stage: "测试验证",
          at: "2025-06-12T06:30:00Z",
          at_source: "content_time",
        },
        todos: [{ title: "部署手册补充「监听自检」步骤" }],
        provenance: {
          source: "paste",
          origin: "微信群-客户群",
          imported: false,
        },
        body: "## 现象\n…",
      },
    ],
  },
  created_at: "2025-06-12T06:32:00Z",
  updated_at: "2025-06-12T06:32:00Z",
} satisfies CardResponse;

// narrowing：type 判别后 payload 字段类型收窄
if (prdCardSample.content.type === "card.batch") {
  const entries: number = prdCardSample.content.entries.length;
  const firstTitle: string = prdCardSample.content.entries[0].title;
  const atSource: "content_time" =
    prdCardSample.content.entries[0].event?.at_source ?? "capture_time";
  console.log(entries, firstTitle, atSource);
}

// ---------------------------------------------------------------------------
// S1（SSE 信封 union）：事件 type 判别 narrowing
// ---------------------------------------------------------------------------

const sseEvents: StreamEnvelope[] = [
  { type: "note_created", event_id: 1, payload: {} as NoteResponse },
  { type: "project_updated", event_id: 2, payload: {} as never },
  {
    type: "capture_status_changed",
    event_id: 3,
    payload: {
      capture_id: "cap_01J8Z3A7B4C5D6E7F8G9H0JKMN",
      status: "card_ready",
    },
  },
];
for (const event of sseEvents) {
  switch (event.type) {
    case "note_created":
      console.log(event.payload.id);
      break;
    case "capture_status_changed":
      console.log(event.payload.status);
      break;
    default:
      console.log(event.event_id);
  }
}

// ---------------------------------------------------------------------------
// S2：可选 = 字段省略（无 | null）
// ---------------------------------------------------------------------------

const createNote: CreateNoteRequest = {
  project_id: "prj_01J8Z3A7B4C5D6E7F8G9H0JKMN",
  title: "切片验证",
  body: "hello jotline",
  // target_dir / tags / format 全部省略——可选字段不写即无
};
// @ts-expect-error 可选字段不接受 null（Wire 纪律：禁止 required-nullable）
const bad: CreateNoteRequest = { ...createNote, target_dir: null };

// ---------------------------------------------------------------------------
// S4 / S7：openapi-fetch 消费生成类型（OpenAPI 3.1）
// ---------------------------------------------------------------------------

async function smoke() {
  const search = await client.GET("/api/search", {
    params: { query: { q: "oracle", top_k: 5 } satisfies Partial<SearchQuery> },
    headers: { Authorization: `Bearer ${DEV_TOKEN}` },
  });
  if (search.data) {
    for (const hit of search.data) {
      console.log(hit.id, hit.snippet, hit.provenance.origin, hit.score);
    }
  }
  const created = await client.POST("/api/notes", {
    body: createNote,
    headers: { Authorization: `Bearer ${DEV_TOKEN}` },
  });
  console.log(created.data?.id);
  // 401 信封类型
  if (created.error) {
    console.log(created.error.code, created.error.message);
  }
}

// ---------------------------------------------------------------------------
// stdio JSON-RPC（Ipc* 命名）类型导入
// ---------------------------------------------------------------------------

const healthReq: IpcHealthRequest = {
  jsonrpc: "2.0",
  id: "ping-1",
  method: "health.ping",
};
const healthResp: IpcHealthResponse = {
  jsonrpc: "2.0",
  id: healthReq.id,
  result: { status: "ok" },
};
console.log(healthResp.result.status, bad === undefined, smoke);
