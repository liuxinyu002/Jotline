// executeCard 事件挂靠单测（proposal 违约修复②）：事件行的 note_id 仅在本 entry
// 创建笔记时关联——不取「最近创建的笔记」误挂前驱 entry，也不凭空缺省。
import assert from "node:assert/strict";
import test from "node:test";
import type { components } from "@jotline/contracts";
import { type CardEntity, SemanticState } from "../src/state.ts";

const AT = { at: "2025-06-12T06:30:00Z", at_source: "content_time" } as const;
const PROV = { source: "manual", origin: "manual", imported: false } as const;

function pendingCard(
  id: string,
  entries: components["schemas"]["CardEntry"][],
): CardEntity {
  return {
    id,
    status: "pending",
    content: {
      type: "card.batch",
      project: "prj_01J8Z3PRJ10000000000000000",
      entries,
    },
    created_at: "2025-06-12T06:32:00Z",
    updated_at: "2025-06-12T06:32:00Z",
  };
}

function newState(): SemanticState {
  return new SemanticState(() => {});
}

test("executeCard 挂靠①：entry 同含 create_note 与 event → note_id 指向本 entry 笔记", () => {
  const state = newState();
  state.cards.set(
    "crd_1",
    pendingCard("crd_1", [
      {
        action: "create_note",
        title: "T1",
        body: "B1",
        provenance: PROV,
        event: { type: "问题", stage: "测试验证", ...AT },
      },
    ]),
  );
  const r = state.executeCard("crd_1");
  assert.equal(r.note_ids.length, 1);
  assert.equal(r.event_ids.length, 1);
  const event = state.events.get(r.event_ids[0]) as { note_id?: string };
  assert.equal(
    event.note_id,
    r.note_ids[0],
    "note_id 应指向本 entry 创建的笔记",
  );
});

test("executeCard 挂靠②：entry 仅 event 且前 entry 有笔记 → note_id 不误挂前驱", () => {
  const state = newState();
  state.cards.set(
    "crd_2",
    pendingCard("crd_2", [
      { action: "create_note", title: "T2", body: "B2", provenance: PROV },
      {
        action: "add_event",
        title: "",
        body: "",
        provenance: PROV,
        event: { type: "会议", ...AT },
      },
    ]),
  );
  const r = state.executeCard("crd_2");
  assert.equal(r.note_ids.length, 1, "前 entry 的笔记正常创建");
  assert.equal(r.event_ids.length, 1);
  const event = state.events.get(r.event_ids[0]) as { note_id?: string };
  assert.equal(
    event.note_id,
    undefined,
    "事件不挂靠前驱 entry 的笔记（不取 noteIds.at(-1)）",
  );
});

test("executeCard 挂靠③：首 entry 仅 event → note_id 缺省", () => {
  const state = newState();
  state.cards.set(
    "crd_3",
    pendingCard("crd_3", [
      {
        action: "add_event",
        title: "",
        body: "",
        provenance: PROV,
        event: { type: "会议", ...AT },
      },
    ]),
  );
  const r = state.executeCard("crd_3");
  assert.equal(r.note_ids.length, 0);
  assert.equal(r.event_ids.length, 1);
  const event = state.events.get(r.event_ids[0]) as { note_id?: string };
  assert.equal(event.note_id, undefined, "无本 entry 笔记 → note_id 缺省");
});
