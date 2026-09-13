# Tasks: phase-1-contracts-skeleton

## 1. 仓库引导与工作区骨架

- [ ] 1.1 git init + `.gitignore`（`._*` / `.DS_Store` / `target/` / `node_modules/` / `dist/`）+ 基线提交（准备阶段已完成则跳过，验证 AppleDouble 文件零入库）
- [ ] 1.2 根 `package.json` + `pnpm-workspace.yaml`（packages: app / sidecar / contracts/mock / contracts/generated）+ 根脚本占位（`contracts:build` / `contracts:check` / `mock`）；验证 `pnpm install` 零错误
- [ ] 1.3 工具链版本钉死：`rust-toolchain.toml`、`.node-version`、`package.json` engines + packageManager；验证各工具读取各自钉死版本
- [ ] 1.4 Biome 初始化：根 `biome.json`（三 TS 包共用，排除 `contracts/generated/**`）+ `pnpm lint` / `pnpm format` 脚本；验证 `pnpm lint` 对占位文件通过
- [ ] 1.5 app/ 与 sidecar/ 最小占位（package.json + tsconfig，声明对 `@jotline/contracts` 的 workspace 依赖）；验证 `pnpm --filter app typecheck` / `pnpm --filter sidecar typecheck` 空跑通过
- [ ] 1.6 README「本地开发」节 + `.env.example`（端口 4765/4766/1420、`dev-token`、`JOTLINE_DATA_DIR`、`DEEPSEEK_API_KEY` 占位）；验证按 README 从零可复现环境准备步骤

## 2. Rust 契约 crate 与 spike

- [ ] 2.1 根虚拟 `Cargo.toml`（members = ["src-tauri/crates/contracts"]）+ 契约 crate 骨架（仅 utoipa + serde + chrono + ulid 依赖，零 tauri）；验证 `cargo check -p contracts` 在 Linux 兼容路径下通过（无平台特定依赖）
- [ ] 2.2 通用类型基座：`Id<T>` newtype（ULID + 前缀常量表）、`DateTime<Utc>` 别名、错误信封、分页/列表信封；验证 cargo 单测（ULID 前缀、RFC3339 序列化）
- [ ] 2.3 执行 spike S1（tagged enum 含 `card.batch` 点号 tag → TS union narrowing）：写最小样例类型 + 临时生成脚本，跑 serde↔TS round-trip 类型测试；结论记入 spike 记录（IDR 落档见 8.x）
- [ ] 2.4 执行 spike S2（Option + skip_serializing_if → `field?: T` 无 `| null`）；不通过则按 design 退路调整标注方式并记录
- [ ] 2.5 执行 spike S5（生成确定性）与 S7（OpenAPI 3.0 vs 3.1 生态兼容：openapi-typescript / Ajv / openapi-fetch）；定稿输出版本

## 3. 首批契约域类型与 path stub

- [ ] 3.1 鉴权与错误域全量：401/404/409/422/500 错误码枚举 + 错误信封响应类型；验证 utoipa schema 生成含全部错误码
- [ ] 3.2 笔记基础域全量：create_note / read / search 的请求/响应类型（含 snippet + provenance 命中列表）+ path stub（`#[utoipa::path]` 空函数）；验证 gen 后 openapi.json 含三路径
- [ ] 3.3 SSE 信封与首批事件类型：信封（type + payload + 事件 id）+ 首批事件（notes/projects/todos 基础变更 + capture 状态）注册进 components；验证 TS 侧可导入信封 union
- [ ] 3.4 stdio JSON-RPC 域：请求/响应/通知/错误信封 + health 方法类型，注册进 components（`Ipc*` 命名）；验证 TS 侧可导入
- [ ] 3.5 实体域全量：projects / stages / tags（list / upsert）类型 + path stub；验证 PRD 字段齐备（含 sensitive 标记、阶段序列、标签计数）
- [ ] 3.6 捕获域全量：capture 提交输入矩阵（text/image/file tagged union）+ card schema（PRD §7.3：entries / event / todos[] / provenance.processing / group_name / at_source）+ path stub；验证样例卡片 JSON 可反序列化为契约类型（cargo 单测）
- [ ] 3.7 骨架域：events / timeline_groups / templates / todos（todos.status 为可扩展 enum，DEC-09 弹性）核心字段 + list path stub；验证生成 TS 类型不含未定决策字段
- [ ] 3.8 Wire 禁用清单不涉及：本组全部类型按 D4 机制编写（tagged enum / skip_serializing_if / snake_case）；验证 `contracts:check` 的 grep 检查脚本对故意注入的 `untagged` 样例显红（负向测试）

## 4. 生成管线与三端生成物

- [ ] 4.1 gen bin（`src/bin/gen.rs`）：组装 ApiDoc → 写 `contracts/openapi.json`（确定性键排序）；验证两次运行 diff 为空（S5 定稿后固化）
- [ ] 4.2 openapi-typescript 接入：`contracts/generated/` 组装为 `@jotline/contracts` workspace 包（package.json + 导出入口）；验证 `pnpm --filter app typecheck` 导入 HTTP / SSE / Ipc 类型零错误
- [ ] 4.3 工具注册表 + 工具 Schema 生成：Rust 侧 `ToolSpec` 静态表（list_projects / create_note / search_content 三个样例）→ deref $ref 自包含 JSON Schema 写 `contracts/tools/`；验证生成文件无外部 $ref
- [ ] 4.4 根脚本 `pnpm contracts:build`（cargo gen + TS 组装 + 场景校验编排）与 `pnpm contracts:check`（重新生成 + `git diff --exit-code` + Wire grep）；验证：改契约类型一个字段名后 check 显红，重新 build 后 check 零 diff

## 5. 协议文档 PROTOCOL.md

- [ ] 5.1 Wire 纪律细则 + ID 前缀注册表（15 实体）+ 时间格式细则；验证与契约源实现逐条对应（review 检查）
- [ ] 5.2 错误码表（信封格式 + 已实现错误码）+ 鉴权约定（Bearer / dev-token / 端口）+ SSE 事件信封与首批事件注册表 + stdio JSON-RPC 方法注册表（health 定稿，其余预留）；验证 Mock 实现与文档一致（401 信封、SSE 事件名抽查）

## 6. Mock 服务与场景数据集

- [ ] 6.1 引擎骨架：读 openapi.json 路由匹配（契约外 404）+ Bearer dev-token 鉴权（错 token 401 错误信封）；验证 `pnpm mock` 启动后 curl 无 token → 401
- [ ] 6.2 场景数据格式定稿 + 校验层：场景文件 schema（种子状态 + 剧本路由 + 延迟/错误注入声明）、启动时按契约 Schema 校验、固定 ULID；验证故意坏场景 → 启动失败并定位文件与字段
- [ ] 6.3 语义内核：内存状态播种 + 写操作变更 + 读反映状态 + SSE 广播（状态变更推事件 + 心跳）；验证创建一条笔记后列表可见、SSE 订阅端收到事件
- [ ] 6.4 剧本层与场景选择：延迟注入、错误注入（409 场景）、`X-Mock-Scenario` 请求级选择 + `/_mock/*` 控制端点（切换/重置，不在 openapi.json）；验证延迟注入场景实测生效、重置后状态回种子
- [ ] 6.5 首批场景数据集：default（projects/notes/search 数据，ULID+ISO-8601 抽查合格）/ auth-401 / stream-demo（3 条事件后正常关闭）/ search-oracle（含 snippet+provenance 命中）；验证 Roadmap 四条 curl 命令逐一通过

## 7. CI 布防

- [ ] 7.1 `.github/workflows/contracts.yml`：contracts job（build → diff --exit-code + 场景校验 + mock 冒烟 curl 断言）/ rust job（fmt + clippy）/ ts job（biome + typecheck），ubuntu-latest；验证在 GitHub 上首次运行三 job 全绿
- [ ] 7.2 pre-commit hook（contracts:check，倾向零依赖脚本）；验证改动契约源未重新生成时提交被拦截
- [ ] 7.3 仓库推送 GitHub 后开启 main 分支 required check（contracts job）+ 分支保护；验证故意推送未重生成的提交被拒（可用测试分支演练后删除）

## 8. 文档回写与 spike 结论归档

- [ ] 8.1 SPEC §3.2 工具链行修订（移除 tauri-specta 括注）+ 附录 B IDR 登记（工具链定稿 + S1–S7 结论 + D2 演进策略）；验证 SPEC 变更说明遵循「先修订再实施」纪律（本 change 即实施载体）
- [ ] 8.2 Roadmap Phase-1 勾选 + spec-driven 存档准备（`openspec validate --strict` 通过）

## 9. 最终验证（Roadmap Phase-1 完成标准）

- [ ] 9.1 依序执行 Roadmap 验证命令并全部通过：`pnpm contracts:build` 生成三端产物 → `pnpm contracts:check` 零 diff → `pnpm mock` 起服务 → `curl /api/projects`（200，ULID/ISO-8601 抽查）→ `curl /api/search?q=oracle`（snippet+provenance 命中）→ `curl -sN /api/stream`（3 条事件后关闭）→ 无 token curl → 401
