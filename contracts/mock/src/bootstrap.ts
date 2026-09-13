// 引擎组装（server 与 validate CLI 共用）：契约 + 场景加载与 Ajv 校验。
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { ContractValidator } from "./ajv.ts";
import { MockEngine } from "./router.ts";
import { loadScenarios, type ScenarioLoadError } from "./scenarios.ts";
import { SseHub } from "./sse.ts";
import { SemanticState } from "./state.ts";
import type { OpenAPIV3_1 } from "./types.ts";

const here = path.dirname(fileURLToPath(import.meta.url));
export const OPENAPI_PATH = path.join(here, "..", "..", "openapi.json");
export const SCENARIOS_DIR = path.join(here, "..", "scenarios");

export function readJson(file: string): unknown {
  return JSON.parse(readFileSync(file, "utf8"));
}

export function buildEngine(log: (msg: string) => void): {
  engine: MockEngine | null;
  errors: ScenarioLoadError[];
} {
  const doc = readJson(OPENAPI_PATH) as OpenAPIV3_1.Document;
  const validator = new ContractValidator(doc);
  const validate = (schemaName: string, data: unknown) =>
    validator.validateAgainst(schemaName, data);
  const [scenarios, errors] = loadScenarios(
    SCENARIOS_DIR,
    doc,
    readJson,
    validate,
  );
  if (errors.length > 0) return { engine: null, errors };

  const sse = new SseHub();
  const state = new SemanticState((e) => sse.broadcast(e));
  const engine = new MockEngine(doc, scenarios, validator, state, sse, log);
  return { engine, errors: [] };
}
