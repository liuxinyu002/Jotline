// Ajv 校验层：按契约 Schema 校验请求体 / 查询参数 / 场景数据。
// OpenAPI 3.1 schema = JSON Schema 2020-12；$ref 经 addSchema 键注册解析（spike S7 验证）。
import { Ajv2020 } from "ajv/dist/2020.js";
import type { OpenAPIV3_1 } from "./types.ts";

export type ValidateFn = (data: unknown) => boolean;

export interface AjvError {
  instancePath: string;
  message?: string;
  params?: Record<string, unknown>;
  schemaPath?: string;
}

export class ContractValidator {
  private ajv: Ajv2020;
  private compiled = new Map<string, ValidateFn>();

  constructor(doc: OpenAPIV3_1.Document) {
    this.ajv = new Ajv2020({
      allErrors: true,
      strict: false,
      formats: { int32: true, int64: true, "date-time": true },
    });
    for (const [name, schema] of Object.entries(
      doc.components?.schemas ?? {},
    )) {
      this.ajv.addSchema(schema, `#/components/schemas/${name}`);
    }
  }

  /** 校验是否为某 components schema（场景数据校验用）。 */
  validateAgainst(schemaName: string, data: unknown): boolean {
    const fn = this.getCompiled(schemaName, {
      $ref: `#/components/schemas/${schemaName}`,
    } as never);
    return fn(data);
  }

  /** 字段定位错误（422 信封 detail 用）——读自校验函数自身的 errors。 */
  errorsOf(fn: ValidateFn): { field: string; message: string }[] {
    const errors = (fn as { errors?: AjvError[] }).errors ?? [];
    return errors.map((e) => {
      const missing = (e.params as { missingProperty?: string } | undefined)
        ?.missingProperty;
      const field =
        missing ??
        (e.instancePath.replace(/^\//, "").replace(/\//g, ".") || "(root)");
      return { field, message: e.message ?? "校验失败" };
    });
  }

  /** 编译请求体 schema（含 $ref，经 addSchema 键解析）。 */
  requestBodyValidator(
    operation: OpenAPIV3_1.OperationObject,
  ): ValidateFn | null {
    const schema =
      operation.requestBody?.content?.["application/json"]?.schema ?? null;
    if (!schema) return null;
    return this.getCompiled(
      `req:${operation.operationId ?? "anon"}`,
      schema as never,
    );
  }

  /** 编译响应体 schema（按操作 + 状态码；design D4 响应侧校验用）。 */
  responseBodyValidator(
    operation: OpenAPIV3_1.OperationObject,
    status: number,
  ): ValidateFn | null {
    const schema =
      operation.responses?.[String(status)]?.content?.["application/json"]
        ?.schema ?? null;
    if (!schema) return null;
    return this.getCompiled(
      `res:${operation.operationId ?? "anon"}:${status}`,
      schema as never,
    );
  }

  /** 编译查询参数 schema（由契约参数表合成；缓存键 = 操作 id）。 */
  queryValidator(
    operationId: string,
    properties: Record<string, unknown>,
    required: string[],
  ): ValidateFn {
    return this.getCompiled(`query:${operationId}`, {
      type: "object",
      properties,
      required: required.length > 0 ? required : undefined,
      additionalProperties: false,
    } as never);
  }

  private getCompiled(key: string, schema: never): ValidateFn {
    const cached = this.compiled.get(key);
    if (cached) return cached;
    const fn: ValidateFn = this.ajv.compile(schema);
    this.compiled.set(key, fn);
    return fn;
  }
}
