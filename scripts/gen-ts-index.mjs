#!/usr/bin/env node
// 从 contracts/openapi.json 的 components 清单生成 @jotline/contracts 命名导出入口。
// 确定性：openapi.json 本身键排序（gen.rs BTreeMap），此处按文件序遍历即字典序。
// 本脚本是「TS 组装」步骤的一部分（pnpm contracts:build 调用），产物 commit 入库。
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const openapiPath = path.join(scriptDir, "..", "contracts", "openapi.json");
const outPath = path.join(scriptDir, "..", "contracts", "generated", "src", "index.ts");

const doc = JSON.parse(readFileSync(openapiPath, "utf8"));
const schemaNames = Object.keys(doc.components?.schemas ?? {});
if (schemaNames.length === 0) {
  console.error("openapi.json 无 components.schemas，拒绝生成空入口");
  process.exit(1);
}

const lines = [
  "// 本文件由 pnpm contracts:build 生成（scripts/gen-ts-index.mjs）——禁止手写修改。",
  'import type { components } from "./schema.js";',
  "",
  'export type { components, paths, operations } from "./schema.js";',
  "",
];
for (const name of schemaNames) {
  lines.push(`export type ${name} = components["schemas"]["${name}"];`);
}
lines.push("");

writeFileSync(outPath, lines.join("\n"));
console.log(`生成 ${outPath}（${schemaNames.length} 个命名类型导出）`);
