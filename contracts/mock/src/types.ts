// OpenAPI 3.1 文档的最小类型声明（仅覆盖 Mock 引擎消费的字段）。

export namespace OpenAPIV3_1 {
  export interface Document {
    openapi: string;
    info?: Record<string, unknown>;
    servers?: unknown[];
    paths: Record<string, PathItemObject>;
    components?: {
      schemas?: Record<string, SchemaObject>;
    };
  }

  export interface PathItemObject {
    get?: OperationObject;
    post?: OperationObject;
    patch?: OperationObject;
    put?: OperationObject;
    delete?: OperationObject;
  }

  export interface OperationObject {
    operationId?: string;
    tags?: string[];
    summary?: string;
    parameters?: ParameterObject[];
    requestBody?: RequestBodyObject;
    responses: Record<string, ResponseObject>;
  }

  export interface ParameterObject {
    name: string;
    in: string;
    required?: boolean;
    description?: string;
    schema?: SchemaObject;
  }

  export interface RequestBodyObject {
    required?: boolean;
    content: Record<string, { schema?: SchemaObject }>;
  }

  export interface ResponseObject {
    description?: string;
    content?: Record<string, { schema?: SchemaObject }>;
  }

  export type SchemaObject = Record<string, unknown>;
}
