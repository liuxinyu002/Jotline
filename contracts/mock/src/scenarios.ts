// 场景数据格式（design D5 + task 6.2 定稿）：
//   seed   —— 语义内核播种状态（各实体集合，ID 用固定 ULID，commit 稳定）
//   script —— 剧本层：路由级延迟 / 错误注入（"*" 匹配全部契约路由）
//   stream —— SSE 剧本：预定义事件序列，发完后关闭连接
import { readdirSync } from "node:fs";
import type { OpenAPIV3_1 } from "./types.ts";

/** 路由剧本键：`<METHOD> /api/...` 或 `"*"`（全部契约路由）。 */
export type RouteKey = string;

export interface RouteScript {
  /** 延迟注入（毫秒；响应耗时不低于此值） */
  delay_ms?: number;
  /** 错误注入（状态码 + 错误信封） */
  error?: {
    status: number;
    code: string;
    message: string;
  };
}

export interface Scenario {
  name: string;
  description: string;
  seed: {
    projects?: unknown[];
    stages?: unknown[];
    tags?: unknown[];
    notes?: unknown[];
    events?: unknown[];
    timeline_groups?: unknown[];
    templates?: unknown[];
    todos?: unknown[];
    captures?: unknown[];
    cards?: unknown[];
  };
  script?: {
    routes?: Record<RouteKey, RouteScript>;
  };
  stream?: {
    /** 预定义 SSE 事件序列（StreamEnvelope 形态） */
    events?: unknown[];
    /** 发完 events 后正常关闭连接 */
    close_after?: boolean;
  };
}

/** seed 集合名 → components schema 名（场景数据按契约 Schema 校验的映射表）。 */
export const SEED_SCHEMAS: Record<string, string> = {
  projects: "Project",
  stages: "Stage",
  tags: "Tag",
  notes: "NoteResponse",
  events: "Event",
  timeline_groups: "TimelineGroup",
  templates: "Template",
  todos: "Todo",
  captures: "CaptureStatusResponse",
  cards: "CardResponse",
};

export interface ScenarioLoadError {
  file: string;
  message: string;
}

/** 加载并校验全部场景；返回 [场景表, 错误列表]（有错时表为空）。 */
export function loadScenarios(
  dir: string,
  doc: OpenAPIV3_1.Document,
  readJson: (path: string) => unknown,
  validate: (schemaName: string, data: unknown) => boolean,
): [Map<string, Scenario>, ScenarioLoadError[]] {
  const errors: ScenarioLoadError[] = [];
  const scenarios = new Map<string, Scenario>();

  let files: string[];
  try {
    files = readdirSync(dir).filter(
      (f) => f.endsWith(".json") && !f.startsWith("."),
    );
  } catch {
    return [scenarios, [{ file: dir, message: `场景目录不存在：${dir}` }]];
  }

  for (const file of files) {
    const rel = `${dir}/${file}`;
    let raw: unknown;
    try {
      raw = readJson(rel);
    } catch (e) {
      errors.push({ file: rel, message: `JSON 解析失败：${e}` });
      continue;
    }
    const errs = validateScenario(rel, raw, doc, validate);
    if (errs.length > 0) {
      errors.push(...errs);
      continue;
    }
    const scenario = raw as Scenario;
    if (scenarios.has(scenario.name)) {
      errors.push({ file: rel, message: `场景名重复：${scenario.name}` });
      continue;
    }
    scenarios.set(scenario.name, scenario);
  }
  return [scenarios, errors];
}

/** 校验单个场景文件：结构 + seed 集合按契约 Schema + 剧本路由在契约内。 */
export function validateScenario(
  file: string,
  raw: unknown,
  doc: OpenAPIV3_1.Document,
  validate: (schemaName: string, data: unknown) => boolean,
): ScenarioLoadError[] {
  const errors: ScenarioLoadError[] = [];
  const fail = (message: string) => errors.push({ file, message });

  if (typeof raw !== "object" || raw === null) {
    return [{ file, message: "场景根必须为对象" }];
  }
  const s = raw as Record<string, unknown>;
  if (typeof s.name !== "string" || s.name.length === 0) fail("缺少 name 字段");
  if (typeof s.description !== "string") fail("缺少 description 字段");
  if (s.seed !== undefined && typeof s.seed !== "object")
    fail("seed 必须为对象");

  // seed 集合逐条按契约 Schema 校验（定位到 集合[下标]）
  const seed = (s.seed ?? {}) as Record<string, unknown>;
  for (const [collection, schemaName] of Object.entries(SEED_SCHEMAS)) {
    const entries = seed[collection];
    if (entries === undefined) continue;
    if (!Array.isArray(entries)) {
      fail(`seed.${collection} 必须为数组`);
      continue;
    }
    for (let i = 0; i < entries.length; i++) {
      if (!validate(schemaName, entries[i])) {
        fail(`seed.${collection}[${i}] 不符合契约 Schema ${schemaName}`);
      }
    }
  }
  for (const key of Object.keys(seed)) {
    if (!(key in SEED_SCHEMAS)) fail(`seed.${key} 不是已登记的实体集合`);
  }

  // 剧本路由键必须在契约内（或 "*"）
  const routes = (s.script as Record<string, unknown> | undefined)?.routes as
    | Record<string, unknown>
    | undefined;
  if (routes !== undefined) {
    for (const key of Object.keys(routes)) {
      if (key !== "*" && !routeKeyExists(key, doc)) {
        fail(`剧本路由 ${key} 不在 openapi.json 契约内`);
      }
    }
  }

  // stream 事件按 StreamEnvelope 校验
  const stream = s.stream as Record<string, unknown> | undefined;
  if (stream?.events !== undefined) {
    if (!Array.isArray(stream.events)) fail("stream.events 必须为数组");
    else {
      for (let i = 0; i < stream.events.length; i++) {
        if (!validate("StreamEnvelope", stream.events[i])) {
          fail(`stream.events[${i}] 不符合 StreamEnvelope 契约`);
        }
      }
    }
  }
  return errors;
}

function routeKeyExists(key: string, doc: OpenAPIV3_1.Document): boolean {
  const m = key.match(/^(\w+) (.+)$/);
  if (!m) return false;
  const [, method, path] = m;
  const item = doc.paths[path];
  return Boolean(
    item && (item as Record<string, unknown>)[method.toLowerCase()],
  );
}
