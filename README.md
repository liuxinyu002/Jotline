# Jotline（项目随手记）

实施顾问的项目记忆库：截图 / 粘贴 / 文件随手丢入，Agent 结构化为事件、待办与笔记。

- 产品需求：`docs/PRD.refined.md` ｜ 技术规范：`docs/SPEC.md` ｜ 分期大纲：`docs/Roadmap.md`
- 接口契约：Rust 契约源（`src-tauri/crates/contracts`）为唯一事实源，生成物入库于 `contracts/`，协议细则见 `contracts/PROTOCOL.md`

## 本地开发

### 前置依赖

| 工具 | 版本（钉死于仓库） | 安装方式 |
|---|---|---|
| Rust | 1.95.0（`rust-toolchain.toml`） | [rustup](https://rustup.rs)（进入仓库自动切版本） |
| Node.js | 24.14.1（`.node-version`） | [nvm](https://github.com/nvm-sh/nvm)：`nvm use` |
| pnpm | 12.4.1（`package.json` packageManager） | `corepack enable`（进入仓库自动锁定） |

### 环境准备（从零开始）

```bash
git clone <repo-url> && cd Jotline

# 1. 启用 pnpm（corepack 按 packageManager 字段锁定版本）
corepack enable

# 2. 安装依赖（CI 同款：按 lockfile 冻结安装）
pnpm install --frozen-lockfile

# 3. 安装 pre-commit hook（契约防漂移内环；见 scripts/pre-commit.sh）
cp scripts/pre-commit.sh .git/hooks/pre-commit 2>/dev/null || true

# 4. 校验 Rust 工具链与契约 crate 编译（按 rust-toolchain.toml 锁定 1.95.0）
cargo check -p contracts --locked

# 5. 生成契约三端产物（openapi.json / 前端 TS 类型 / 工具 Schema）
pnpm contracts:build

# 6. 契约防漂移检查（重新生成 + 零 diff 校验 + Wire 纪律 grep）
pnpm contracts:check

# 7. 复制环境变量占位（可选：Phase-1/2 仅 Mock 需要 token，其余为后续阶段占位）
cp .env.example .env
```

### 常用命令

```bash
pnpm mock        # 启动契约 Mock 服务（127.0.0.1:4766，Bearer dev-token）
pnpm lint        # Biome 检查（app / sidecar / mock）
pnpm format      # Biome 格式化
pnpm typecheck   # 全部 TS 包类型检查
```

### 冒烟验证（Mock）

```bash
pnpm mock &      # 后台启动契约 Mock
curl -s http://127.0.0.1:4766/api/projects -H "Authorization: Bearer dev-token"
curl -s "http://127.0.0.1:4766/api/search?q=oracle" -H "Authorization: Bearer dev-token"
curl -s http://127.0.0.1:4766/api/projects    # 无 token → 401
```

### 开发环境约定（Roadmap 总则 4）

| 项 | 值 |
|---|---|
| 领域 API（真实后端） | `127.0.0.1:4765`（Phase-2 起） |
| 契约 Mock 服务 | `127.0.0.1:4766`（`MOCK_PORT` 可覆盖） |
| 前端 dev server | `127.0.0.1:1420`（Phase-11 起） |
| dev Bearer token | `dev-token`（固定） |
| 数据目录 | `JOTLINE_DATA_DIR`，默认 `./vault` |

### 契约变更流程（铁律）

接口一律先改 Rust 契约源（`src-tauri/crates/contracts`）→ `pnpm contracts:build` 重新生成并提交全部生成物 → `pnpm contracts:check` 零 diff 通过 → 再实施消费侧代码（app / sidecar / mock 场景）。禁止手写第二份接口定义。
