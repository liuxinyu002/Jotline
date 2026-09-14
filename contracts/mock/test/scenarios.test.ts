// 场景装载单测（design D7）：loadScenarios 的 DI 面——注入 readJson
// 与真实契约校验器，锚定装载纪律（Schema 校验 / 契约内路由键 / 重名拒绝）。
// 目录清单不可注入（readdirSync 直读），用临时目录承载 fixture。
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { ContractValidator } from "../src/ajv.ts";
import { OPENAPI_PATH, readJson } from "../src/bootstrap.ts";
import { loadScenarios } from "../src/scenarios.ts";
import type { OpenAPIV3_1 } from "../src/types.ts";

const doc = readJson(OPENAPI_PATH) as OpenAPIV3_1.Document;
const validator = new ContractValidator(doc);
const validate = (schemaName: string, data: unknown) =>
  validator.validateAgainst(schemaName, data);

/** 临时场景目录：文件名 → JSON 内容。 */
function scenarioDir(files: Record<string, unknown>): string {
  const dir = mkdtempSync(join(tmpdir(), "jotline-mock-test-"));
  for (const [name, content] of Object.entries(files)) {
    writeFileSync(join(dir, name), JSON.stringify(content));
  }
  return dir;
}

const VALID_PROJECT = {
  id: "prj_01J8Z3TEST0000000000000000",
  name: "测试项目",
  sensitive: false,
  created_at: "2025-06-01T02:00:00Z",
  updated_at: "2025-06-01T02:00:00Z",
};

test("loadScenarios DI：合法场景装载（种子按契约 Schema 校验通过）", () => {
  const dir = scenarioDir({
    "ok.json": {
      name: "ok",
      description: "合法场景",
      seed: { projects: [VALID_PROJECT] },
    },
  });
  const [scenarios, errors] = loadScenarios(dir, doc, readJson, validate);
  assert.deepEqual(errors, []);
  assert.equal(scenarios.size, 1);
  assert.ok(scenarios.get("ok"));
});

test("loadScenarios DI：seed 不符合契约 Schema 被拒（缺必填字段）", () => {
  const broken = { ...VALID_PROJECT } as Record<string, unknown>;
  delete broken.name;
  const dir = scenarioDir({
    "bad.json": {
      name: "bad",
      description: "种子违约",
      seed: { projects: [broken] },
    },
  });
  const [scenarios, errors] = loadScenarios(dir, doc, readJson, validate);
  assert.equal(scenarios.size, 0);
  assert.equal(errors.length, 1);
  assert.match(
    errors[0].message,
    /seed\.projects\[0\] 不符合契约 Schema Project/,
  );
});

test("loadScenarios DI：剧本路由键不在契约内被拒", () => {
  const dir = scenarioDir({
    "bad-route.json": {
      name: "bad-route",
      description: "契约外剧本路由",
      script: { routes: { "GET /api/nonexistent": { delay_ms: 10 } } },
    },
  });
  const [, errors] = loadScenarios(dir, doc, readJson, validate);
  assert.equal(errors.length, 1);
  assert.match(
    errors[0].message,
    /剧本路由 GET \/api\/nonexistent 不在 openapi/,
  );
});

test("loadScenarios DI：场景名重复被拒", () => {
  const dir = scenarioDir({
    "a.json": { name: "dup", description: "重复" },
    "b.json": { name: "dup", description: "重复" },
  });
  const [, errors] = loadScenarios(dir, doc, readJson, validate);
  assert.equal(errors.length, 1);
  assert.match(errors[0].message, /场景名重复：dup/);
});
