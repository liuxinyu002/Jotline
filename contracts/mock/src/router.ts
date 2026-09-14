// 路由层 + 剧本层 + 语义分发：读 openapi.json 匹配（契约外 404）+ Bearer 鉴权（401 信封）
// + 场景剧本（延迟 / 错误注入，优先于语义内核）+ 语义处理。
import type { IncomingMessage, ServerResponse } from "node:http";
import type { ContractValidator } from "./ajv.ts";
import type { RouteScript, Scenario } from "./scenarios.ts";
import type { SseHub } from "./sse.ts";
import { ConflictError, NotFoundError, type SemanticState } from "./state.ts";
import type { OpenAPIV3_1 } from "./types.ts";

export const DEV_TOKEN = process.env.JOTLINE_DEV_TOKEN ?? "dev-token";

interface CompiledRoute {
  method: string;
  path: string;
  regex: RegExp;
  paramNames: string[];
  operation: OpenAPIV3_1.OperationObject;
  templateOf: (pathname: string) => string | null;
}

export class MockEngine {
  private routes: CompiledRoute[];
  activeScenario: string;

  doc: OpenAPIV3_1.Document;
  scenarios: Map<string, Scenario>;
  private validator: ContractValidator;
  private state: SemanticState;
  private sse: SseHub;
  private log: (msg: string) => void;

  constructor(
    doc: OpenAPIV3_1.Document,
    scenarios: Map<string, Scenario>,
    validator: ContractValidator,
    state: SemanticState,
    sse: SseHub,
    log: (msg: string) => void,
  ) {
    this.doc = doc;
    this.scenarios = scenarios;
    this.validator = validator;
    this.state = state;
    this.sse = sse;
    this.log = log;
    this.routes = compileRoutes(doc);
    this.activeScenario = scenarios.has("default")
      ? "default"
      : (scenarios.keys().next().value ?? "default");
    this.reseed();
  }

  /** 全局切换场景（控制端点）或重置：状态回种子。 */
  reseed(scenario?: string): void {
    if (scenario) {
      if (!this.scenarios.has(scenario)) throw new NotFoundError(scenario);
      this.activeScenario = scenario;
    }
    const s = this.scenarios.get(this.activeScenario);
    if (s) this.state.seedFrom(s);
    this.log(`状态已播种：场景 ${this.activeScenario}`);
  }

  async handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
    const url = new URL(req.url ?? "/", "http://127.0.0.1");
    const method = (req.method ?? "GET").toLowerCase();

    // 控制端点（不在 openapi.json，非契约面）
    if (url.pathname.startsWith("/_mock/")) {
      this.handleControl(method, url, res);
      return;
    }

    // 契约路由匹配（契约外 404）
    const route = this.routes.find(
      (r) => r.method === method && r.regex.test(url.pathname),
    );
    if (!route) {
      this.sendJson(
        res,
        404,
        errorEnvelope(
          "not_found",
          `契约外路由：${method.toUpperCase()} ${url.pathname}`,
        ),
      );
      return;
    }

    // Bearer 鉴权（错 token 401 错误信封）；
    // SSE 端点（get_stream）同时接受等价的 query 参数 token（wire-protocol 鉴权约定）
    const bearerOk = req.headers.authorization === `Bearer ${DEV_TOKEN}`;
    const streamQueryOk =
      route.operation.operationId === "get_stream" &&
      url.searchParams.get("token") === DEV_TOKEN;
    if (!bearerOk && !streamQueryOk) {
      this.sendJson(
        res,
        401,
        errorEnvelope(
          "unauthorized",
          "未携带或错误的 Bearer token（经 JOTLINE_DEV_TOKEN 注入，缺省 dev-token）",
        ),
      );
      return;
    }

    // 场景选择：请求级（X-Mock-Scenario，仅剧本生效）→ 否则全局激活场景
    let scenarioName = this.activeScenario;
    const headerScenario = req.headers["x-mock-scenario"];
    if (typeof headerScenario === "string" && headerScenario.length > 0) {
      if (!this.scenarios.has(headerScenario)) {
        this.sendJson(
          res,
          422,
          errorEnvelope(
            "validation_failed",
            `X-Mock-Scenario 指定的场景不存在：${headerScenario}`,
          ),
        );
        return;
      }
      scenarioName = headerScenario;
    }
    const scenario = this.scenarios.get(scenarioName);
    if (!scenario) throw new Error(`场景缺失：${scenarioName}`);

    // 剧本层（优先于语义内核的默认行为）
    const script = this.routeScript(scenario, route, url.pathname);
    if (script?.delay_ms) {
      await sleep(script.delay_ms);
    }
    if (script?.error) {
      this.sendJson(
        res,
        script.error.status,
        errorEnvelope(script.error.code, script.error.message),
      );
      return;
    }

    // SSE：剧本流（预定义事件序列）或常驻订阅（广播 + 心跳）
    if (route.operation.operationId === "get_stream") {
      const streamScript = scenario.stream;
      if (streamScript?.events && streamScript.events.length > 0) {
        await this.sse.playScripted(
          res,
          streamScript.events,
          streamScript.close_after !== false,
        );
      } else {
        this.sse.subscribe(res);
      }
      return;
    }

    // 查询参数：按契约参数表收敛 + 校验
    const queryResult = this.coerceQuery(route.operation, url.searchParams);
    if (queryResult.errors) {
      this.sendJson(
        res,
        422,
        validationEnvelope("查询参数校验失败", queryResult.errors),
      );
      return;
    }

    // 请求体：按契约 Schema 校验（违反契约 → 422 信封含字段定位，状态不变）
    let body: unknown;
    const bodyValidator = this.validator.requestBodyValidator(route.operation);
    if (bodyValidator) {
      body = await readJsonBody(req);
      if (!bodyValidator(body)) {
        this.sendJson(
          res,
          422,
          validationEnvelope(
            "请求体校验失败",
            this.validator.errorsOf(bodyValidator),
          ),
        );
        return;
      }
    }

    // 语义分发
    const match = route.regex.exec(url.pathname);
    if (!match) throw new Error(`路由匹配失效：${url.pathname}`);
    const pathParams: Record<string, string> = {};
    for (let i = 0; i < route.paramNames.length; i++) {
      pathParams[route.paramNames[i]] = decodeURIComponent(match[i + 1]);
    }
    try {
      const result = this.dispatch(
        route,
        pathParams,
        queryResult.value as Record<string, unknown>,
        body as never,
      );
      if (result === null) {
        res.writeHead(204).end();
        return;
      }
      // 响应侧契约校验（design D4）：语义内核产出在发送前断言，
      // 违约不送出 → 500 错误信封（操作 / 状态码 / 违约字段）+ 日志显形
      const responseValidator = this.validator.responseBodyValidator(
        route.operation,
        result.status,
      );
      if (responseValidator && !responseValidator(result.body)) {
        const fields = this.validator.errorsOf(responseValidator);
        const operation = route.operation.operationId ?? "anon";
        this.log(
          `响应违反契约：${operation} ${result.status} 违约字段 ${JSON.stringify(fields)}`,
        );
        this.sendJson(
          res,
          500,
          errorEnvelope(
            "internal",
            `Mock 响应违反契约（${operation} ${result.status}）`,
            fields,
          ),
        );
        return;
      }
      this.sendJson(res, result.status, result.body);
    } catch (e) {
      if (e instanceof NotFoundError) {
        this.sendJson(res, 404, errorEnvelope("not_found", e.message));
      } else if (e instanceof ConflictError) {
        this.sendJson(res, 409, errorEnvelope("conflict", e.message));
      } else {
        throw e;
      }
    }
  }

  // -----------------------------------------------------------------------

  private handleControl(method: string, url: URL, res: ServerResponse): void {
    if (method === "get" && url.pathname === "/_mock/scenarios") {
      this.sendJson(res, 200, {
        active: this.activeScenario,
        scenarios: [...this.scenarios.values()].map((s) => ({
          name: s.name,
          description: s.description,
          active: s.name === this.activeScenario,
        })),
      });
      return;
    }
    const activate = url.pathname.match(
      /^\/_mock\/scenarios\/([^/]+)\/activate$/,
    );
    if (method === "post" && activate) {
      try {
        this.reseed(decodeURIComponent(activate[1]));
        this.sendJson(res, 200, { active: this.activeScenario });
      } catch {
        this.sendJson(
          res,
          404,
          errorEnvelope("not_found", `场景不存在：${activate[1]}`),
        );
      }
      return;
    }
    if (method === "post" && url.pathname === "/_mock/reset") {
      this.reseed();
      this.sendJson(res, 200, { active: this.activeScenario });
      return;
    }
    this.sendJson(
      res,
      404,
      errorEnvelope(
        "not_found",
        `未知控制端点：${method.toUpperCase()} ${url.pathname}`,
      ),
    );
  }

  /** 剧本路由匹配：具体路由键（METHOD + 契约模板）优先于 "*" 通配。 */
  private routeScript(
    scenario: Scenario,
    route: CompiledRoute,
    pathname: string,
  ): RouteScript | null {
    const routes = scenario.script?.routes;
    if (!routes) return null;
    const template = route.templateOf(pathname);
    if (template) {
      const specific = routes[`${route.method.toUpperCase()} ${template}`];
      if (specific) return specific;
    }
    return routes["*"] ?? null;
  }

  private coerceQuery(
    operation: OpenAPIV3_1.OperationObject,
    params: URLSearchParams,
  ): {
    value?: Record<string, unknown>;
    errors?: { field: string; message: string }[];
  } {
    const declared = (operation.parameters ?? []).filter(
      (p) => p.in === "query",
    );
    const value: Record<string, unknown> = {};
    const properties: Record<string, unknown> = {};
    const required: string[] = [];
    for (const p of declared) {
      const schema = p.schema ?? {};
      properties[p.name] = schema;
      if (p.required) required.push(p.name);
      if (schema.type === "array") {
        const all = params.getAll(p.name);
        if (all.length > 0) value[p.name] = all;
      } else {
        const raw = params.get(p.name);
        if (raw !== null) {
          value[p.name] = schema.type === "integer" ? toInt(raw) : raw;
        }
      }
    }
    const fn = this.validator.queryValidator(
      operation.operationId ?? "op",
      properties,
      required,
    );
    if (!fn(value)) {
      return { errors: this.validator.errorsOf(fn) };
    }
    return { value };
  }

  private dispatch(
    route: CompiledRoute,
    pathParams: Record<string, string>,
    query: Record<string, unknown>,
    body: unknown,
  ): { status: number; body: unknown } | null {
    const limit = query.limit as number | undefined;
    const offset = query.offset as number | undefined;
    const list = (items: unknown[]) =>
      this.state.paginate(items, limit, offset);

    switch (route.operation.operationId) {
      case "create_note": {
        const note = this.state.createNote(body as never);
        return { status: 201, body: note };
      }
      case "read_note": {
        const note = this.state.notes.get(pathParams.note_id);
        if (!note) throw new NotFoundError(pathParams.note_id);
        return { status: 200, body: note };
      }
      case "search_content":
        return { status: 200, body: this.state.search(query as never) };
      case "list_projects":
        return { status: 200, body: list([...this.state.projects.values()]) };
      case "upsert_project": {
        const r = this.state.upsertProject(body as never);
        return { status: r.created ? 201 : 200, body: r.project };
      }
      case "list_stages":
        return {
          status: 200,
          body: list(
            [...this.state.stages.values()].filter(
              (s) => s.project_id === query.project_id,
            ),
          ),
        };
      case "upsert_stage": {
        const r = this.state.upsertEntity("stages", "stg", body as never);
        return { status: r.created ? 201 : 200, body: r.entity };
      }
      case "list_tags":
        return { status: 200, body: list([...this.state.tags.values()]) };
      case "upsert_tag": {
        const r = this.state.upsertEntity("tags", "tag", body as never);
        return { status: r.created ? 201 : 200, body: r.entity };
      }
      case "list_events":
        return {
          status: 200,
          body: list(
            [...this.state.events.values()].filter(
              (e) => e.project_id === query.project_id,
            ),
          ),
        };
      case "list_timeline_groups":
        return {
          status: 200,
          body: list(
            [...this.state.timelineGroups.values()].filter(
              (g) => g.project_id === query.project_id,
            ),
          ),
        };
      case "list_templates":
        return { status: 200, body: list([...this.state.templates.values()]) };
      case "list_todos": {
        let todos = [...this.state.todos.values()];
        if (query.project_id)
          todos = todos.filter((t) => t.project_id === query.project_id);
        if (query.status)
          todos = todos.filter((t) => t.status === query.status);
        if (query.source)
          todos = todos.filter((t) => t.source === query.source);
        return { status: 200, body: list(todos) };
      }
      case "submit_capture":
        return { status: 202, body: this.state.submitCapture() };
      case "read_capture": {
        const capture = this.state.captures.get(pathParams.capture_id);
        if (!capture) throw new NotFoundError(pathParams.capture_id);
        return { status: 200, body: capture };
      }
      case "read_card": {
        const card = this.state.cards.get(pathParams.card_id);
        if (!card) throw new NotFoundError(pathParams.card_id);
        return { status: 200, body: card };
      }
      case "update_card":
        return {
          status: 200,
          body: this.state.patchCard(pathParams.card_id, body as never),
        };
      case "execute_card":
        return {
          status: 200,
          body: this.state.executeCard(pathParams.card_id),
        };
      case "discard_card":
        this.state.discardCard(pathParams.card_id);
        return null; // 204
      default:
        throw new Error(`未实现的契约操作：${route.operation.operationId}`);
    }
  }

  private sendJson(res: ServerResponse, status: number, body: unknown): void {
    res.writeHead(status, {
      "content-type": "application/json; charset=utf-8",
    });
    res.end(JSON.stringify(body, null, 2));
  }
}

function compileRoutes(doc: OpenAPIV3_1.Document): CompiledRoute[] {
  const routes: CompiledRoute[] = [];
  for (const [path, item] of Object.entries(doc.paths)) {
    for (const method of ["get", "post", "patch", "put", "delete"] as const) {
      const operation = item[method];
      if (!operation) continue;
      const paramNames = [...path.matchAll(/\{(\w+)\}/g)].map((m) => m[1]);
      const pattern = path.replace(/\{(\w+)\}/g, "([^/]+)");
      const regex = new RegExp(`^${pattern}$`);
      routes.push({
        method,
        path,
        regex,
        paramNames,
        operation,
        templateOf: (pathname: string) => {
          const m = pathname.match(regex);
          if (!m) return null;
          let template = path;
          for (let i = 0; i < paramNames.length; i++) {
            template = template.replace(`{${paramNames[i]}}`, m[i + 1]);
          }
          return template;
        },
      });
    }
  }
  return routes;
}

export function errorEnvelope(
  code: string,
  message: string,
  detail?: { field: string; message: string }[],
): unknown {
  return detail === undefined ? { code, message } : { code, message, detail };
}

export function validationEnvelope(
  message: string,
  detail: { field: string; message: string }[],
): unknown {
  return { code: "validation_failed", message, detail };
}

async function readJsonBody(req: IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(chunk as Buffer);
  const text = Buffer.concat(chunks).toString("utf8");
  if (text.length === 0) return {};
  try {
    return JSON.parse(text);
  } catch {
    // 交给 Schema 校验层报 422（invalid_json 不满足任何契约 Schema）
    return { __invalid_json__: text.slice(0, 100) };
  }
}

function toInt(raw: string): number {
  const n = Number(raw);
  return Number.isFinite(n) ? Math.trunc(n) : Number.NaN;
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
