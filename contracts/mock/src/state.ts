// 语义内核：内存状态（场景播种）+ 写操作变更 + 读反映状态 + SSE 广播。
// 边界钳制（design D5）：仅契约域内基础读写；不实现业务规则（推断 / 聚合 / 降级）。
import type { Scenario } from "./scenarios.ts";

/** Crockford Base32（去 I L O U）。 */
const ENC = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/** 运行时 ULID（48 bit 时间 + 随机；仅用于写操作产生的新实体，场景种子用固定 ULID）。 */
export function newUlid(): string {
  let t = Date.now();
  let timePart = "";
  for (let i = 0; i < 10; i++) {
    timePart = ENC[t % 32] + timePart;
    t = Math.trunc(t / 32);
  }
  const rnd = crypto.getRandomValues(new Uint8Array(16));
  let randPart = "";
  for (let i = 0; i < 16; i++) randPart += ENC[rnd[i] % 32];
  return timePart + randPart;
}

export function nowIso(): string {
  return new Date().toISOString();
}

export function prefixedId(prefix: string): string {
  return `${prefix}_${newUlid()}`;
}

// —— 实体形态（契约生成类型的宽松镜像：字段以 openapi.json 为准，此处只声明内核用到的）——

export interface ProjectEntity {
  id: string;
  name: string;
  sensitive: boolean;
  template_id?: string;
  created_at: string;
  updated_at: string;
}

export interface NoteEntity {
  id: string;
  project_id: string;
  title: string;
  body: string;
  target_dir?: string;
  format: string;
  tags: string[];
  provenance: { source: string; origin: string; imported: boolean };
  created_at: string;
  updated_at: string;
}

export interface TodoEntity {
  id: string;
  project_id?: string;
  title: string;
  due_at?: string;
  status: string;
  owner?: string;
  source: string;
  source_ref?: string;
  created_at: string;
  updated_at: string;
}

export interface CardEntity {
  id: string;
  status: string;
  content: {
    type: string;
    project?: string;
    confidence?: string;
    reason?: string;
    entries: Array<Record<string, unknown>>;
  };
  created_at: string;
  updated_at: string;
}

/** SSE 广播回调（由 sse.ts 注入）。 */
export type Broadcast = (event: { type: string; payload: unknown }) => void;

export class SemanticState {
  projects = new Map<string, ProjectEntity>();
  stages = new Map<string, Record<string, unknown>>();
  tags = new Map<string, Record<string, unknown>>();
  notes = new Map<string, NoteEntity>();
  events = new Map<string, Record<string, unknown>>();
  timelineGroups = new Map<string, Record<string, unknown>>();
  templates = new Map<string, Record<string, unknown>>();
  todos = new Map<string, TodoEntity>();
  captures = new Map<string, Record<string, unknown>>();
  cards = new Map<string, CardEntity>();

  private broadcast: Broadcast;

  constructor(broadcast: Broadcast) {
    this.broadcast = broadcast;
  }

  /** 按场景种子播种（重复启动数据一致——种子 ID 固定）。 */
  seedFrom(scenario: Scenario): void {
    this.clear();
    putAll(this.projects, scenario.seed.projects);
    putAll(this.stages, scenario.seed.stages);
    putAll(this.tags, scenario.seed.tags);
    putAll(this.notes, scenario.seed.notes);
    putAll(this.events, scenario.seed.events);
    putAll(this.timelineGroups, scenario.seed.timeline_groups);
    putAll(this.templates, scenario.seed.templates);
    putAll(this.todos, scenario.seed.todos);
    putAll(this.captures, scenario.seed.captures);
    putAll(this.cards, scenario.seed.cards);
  }

  /** 单调递增 SSE event_id（随全局状态重置归零）。 */
  private seq = 0;

  private emit(type: string, payload: unknown): void {
    this.seq += 1;
    this.broadcast({ type, payload });
  }

  // —— 写操作（语义生效：变更内存状态 + 广播事件）——

  createNote(body: {
    project_id: string;
    title: string;
    body: string;
    target_dir?: string;
    tags?: string[];
    format?: string;
  }): NoteEntity {
    const at = nowIso();
    const note: NoteEntity = {
      id: prefixedId("itm"),
      project_id: body.project_id,
      title: body.title,
      body: body.body,
      target_dir: body.target_dir,
      format: body.format ?? "markdown",
      tags: body.tags ?? [],
      provenance: { source: "manual", origin: "manual", imported: false },
      created_at: at,
      updated_at: at,
    };
    this.notes.set(note.id, note);
    this.emit("note_created", note);
    return note;
  }

  upsertProject(body: {
    id?: string;
    name: string;
    sensitive?: boolean;
    template_id?: string;
  }): { created: boolean; project: ProjectEntity } {
    const at = nowIso();
    if (body.id) {
      const existing = this.projects.get(body.id);
      if (!existing) throw new NotFoundError(body.id);
      existing.name = body.name;
      if (body.sensitive !== undefined) existing.sensitive = body.sensitive;
      if (body.template_id !== undefined)
        existing.template_id = body.template_id;
      existing.updated_at = at;
      this.emit("project_updated", existing);
      return { created: false, project: existing };
    }
    const project: ProjectEntity = {
      id: prefixedId("prj"),
      name: body.name,
      sensitive: body.sensitive ?? false,
      template_id: body.template_id,
      created_at: at,
      updated_at: at,
    };
    this.projects.set(project.id, project);
    this.emit("project_created", project);
    return { created: true, project };
  }

  upsertEntity(
    collection: "stages" | "tags",
    idPrefix: string,
    body: Record<string, unknown>,
  ): { created: boolean; entity: Record<string, unknown> } {
    const map = collection === "stages" ? this.stages : this.tags;
    const at = nowIso();
    const id = body.id as string | undefined;
    if (id) {
      const existing = map.get(id);
      if (!existing) throw new NotFoundError(id);
      Object.assign(existing, {
        ...body,
        id: undefined,
        created_at: undefined,
      });
      // id / created_at 不被覆盖
      existing.id = id;
      existing.updated_at = at;
      return { created: false, entity: existing };
    }
    const entity: Record<string, unknown> = {
      ...body,
      id: prefixedId(idPrefix),
      created_at: at,
      updated_at: at,
    };
    if (collection === "tags" && entity.usage_count === undefined)
      entity.usage_count = 0;
    map.set(entity.id as string, entity);
    return { created: true, entity };
  }

  submitCapture(): {
    capture_id: string;
    status: string;
  } {
    const id = prefixedId("cap");
    const at = nowIso();
    this.captures.set(id, {
      id,
      status: "submitted",
      created_at: at,
      updated_at: at,
    });
    return { capture_id: id, status: "submitted" };
  }

  patchCard(id: string, patch: Record<string, unknown>): CardEntity {
    const card = this.cards.get(id);
    if (!card) throw new NotFoundError(id);
    if (card.status !== "pending")
      throw new ConflictError(
        `卡片已${card.status === "executed" ? "执行" : "丢弃"}，不可调整`,
      );
    for (const key of ["project", "confidence", "reason"] as const) {
      if (patch[key] !== undefined)
        card.content[key] = optionalString(patch[key]);
    }
    if (patch.entries !== undefined)
      card.content.entries = patch.entries as Array<Record<string, unknown>>;
    card.updated_at = nowIso();
    return card;
  }

  executeCard(id: string): {
    card_id: string;
    note_ids: string[];
    event_ids: string[];
    todo_ids: string[];
  } {
    const card = this.cards.get(id);
    if (!card) throw new NotFoundError(id);
    if (card.status !== "pending")
      throw new ConflictError("卡片已执行或已丢弃");
    card.status = "executed";
    card.updated_at = nowIso();
    const noteIds: string[] = [];
    const eventIds: string[] = [];
    const todoIds: string[] = [];
    const at = nowIso();
    for (const entry of card.content.entries) {
      if (entry.action === "create_note") {
        const note = this.createNote({
          project_id: String(card.content.project ?? entry.project_id ?? ""),
          title: String(entry.title ?? ""),
          body: String(entry.body ?? ""),
          target_dir: optionalString(entry.target_dir),
          tags: (entry.tags as string[] | undefined) ?? undefined,
          format: optionalString(entry.format) ?? "markdown",
        });
        noteIds.push(note.id);
      }
      const eventSpec = entry.event as Record<string, unknown> | undefined;
      if (eventSpec) {
        const eventId = prefixedId("evt");
        this.events.set(eventId, {
          id: eventId,
          project_id: card.content.project,
          type: eventSpec.type,
          stage: eventSpec.stage,
          at: eventSpec.at,
          at_source: eventSpec.at_source,
          group_name: entry.group_name ?? eventSpec.group_name,
          note_id: noteIds.at(-1),
          created_at: at,
          updated_at: at,
        });
        eventIds.push(eventId);
      }
      const todos = entry.todos as Array<Record<string, unknown>> | undefined;
      if (todos) {
        for (const t of todos) {
          const todoId = prefixedId("tdo");
          const todo: TodoEntity = {
            id: todoId,
            project_id: card.content.project,
            title: t.title as string,
            due_at: optionalString(t.due),
            status: "open",
            owner: optionalString(t.owner),
            source: "card",
            source_ref: card.id,
            created_at: at,
            updated_at: at,
          };
          this.todos.set(todoId, todo);
          todoIds.push(todoId);
          this.emit("todo_created", todo);
        }
      }
    }
    return {
      card_id: card.id,
      note_ids: noteIds,
      event_ids: eventIds,
      todo_ids: todoIds,
    };
  }

  discardCard(id: string): void {
    const card = this.cards.get(id);
    if (!card) throw new NotFoundError(id);
    if (card.status !== "pending")
      throw new ConflictError("卡片已执行，不可丢弃");
    card.status = "discarded";
    card.updated_at = nowIso();
  }

  /** 检索：子串匹配（title + body）+ 元数据过滤；snippet 含命中上下文。 */
  search(query: {
    q: string;
    project_id?: string;
    target_dir?: string;
    tags?: string[];
    after?: string;
    before?: string;
    top_k?: number;
  }): Array<{
    id: string;
    title: string;
    snippet: string;
    provenance: unknown;
    score: number;
  }> {
    const needle = query.q.toLowerCase();
    const hits = [];
    for (const note of this.notes.values()) {
      if (query.project_id && note.project_id !== query.project_id) continue;
      if (query.target_dir && note.target_dir !== query.target_dir) continue;
      if (query.tags && !query.tags.every((t) => note.tags.includes(t)))
        continue;
      if (query.after && note.created_at < query.after) continue;
      if (query.before && note.created_at > query.before) continue;
      const titleHit = note.title.toLowerCase().includes(needle);
      const bodyIdx = note.body.toLowerCase().indexOf(needle);
      if (!titleHit && bodyIdx < 0) continue;
      const occurrences =
        note.body.toLowerCase().split(needle).length - 1 + (titleHit ? 1 : 0);
      const score = Math.min(1, 0.3 * occurrences + (titleHit ? 0.4 : 0));
      let snippet: string;
      if (bodyIdx >= 0) {
        const start = Math.max(0, bodyIdx - 20);
        snippet = `${start > 0 ? "…" : ""}${note.body.slice(start, bodyIdx + needle.length + 30)}…`;
      } else {
        snippet = note.title;
      }
      hits.push({
        id: note.id,
        title: note.title,
        snippet,
        provenance: note.provenance,
        score,
      });
    }
    hits.sort((a, b) => b.score - a.score || a.id.localeCompare(b.id));
    return hits.slice(0, query.top_k ?? 10);
  }

  /** 分页列表（保持种子顺序 + 写操作追加序）。 */
  paginate<T>(
    items: T[],
    limit?: number,
    offset?: number,
  ): { items: T[]; page: { total: number; limit: number; offset: number } } {
    const l = limit ?? 50;
    const o = offset ?? 0;
    return {
      items: items.slice(o, o + l),
      page: { total: items.length, limit: l, offset: o },
    };
  }

  resetEventSeq(): void {
    this.seq = 0;
  }

  private clear(): void {
    this.projects.clear();
    this.stages.clear();
    this.tags.clear();
    this.notes.clear();
    this.events.clear();
    this.timelineGroups.clear();
    this.templates.clear();
    this.todos.clear();
    this.captures.clear();
    this.cards.clear();
    this.resetEventSeq();
  }
}

function putAll(
  map: Map<string, unknown>,
  entries: unknown[] | undefined,
): void {
  if (!entries) return;
  for (const e of entries) {
    const id = (e as { id: string }).id;
    map.set(id, e);
  }
}

export class NotFoundError extends Error {
  constructor(id: string) {
    super(`资源不存在：${id}`);
  }
}

export class ConflictError extends Error {}

function optionalString(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}
