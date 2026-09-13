// 场景校验 CLI（pnpm --filter @jotline/mock run validate）：
// 与服务启动同一校验层——contracts:build 第 3 步调用，坏场景 → 非零退出并定位文件与字段。
import { buildEngine, SCENARIOS_DIR } from "./bootstrap.ts";

const { engine, errors } = buildEngine(() => {});
if (errors.length > 0 || !engine) {
  console.error("✗ 场景数据校验失败：");
  for (const e of errors) console.error(`  ${e.file}: ${e.message}`);
  process.exit(1);
}
console.log(
  `✓ 场景数据校验通过（${SCENARIOS_DIR}，激活：${engine.activeScenario}）`,
);
