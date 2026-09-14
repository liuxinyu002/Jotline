# vault-storage Specification

## Purpose

定义数据目录（vault）的存储行为：布局初始化、文件真相源落盘、SQLite 索引与真相源分离、dev 首启 seed——支撑「文件为真相源、库可重建」（ADR-5）与 Roadmap Phase-2 验证命令的文件级核对。

## Requirements

### Requirement: 数据目录全布局初始化

主进程启动时 SHALL 在 `JOTLINE_DATA_DIR` 指定的位置初始化完整数据目录布局（notes / attachments / inbox / projects / memory / templates 子目录、audit.jsonl 占位文件、index.sqlite 索引库）；`JOTLINE_DATA_DIR` 的语义 SHALL 为 vault 根目录（非其父目录），默认值 `./vault`（相对启动目录）。布局已存在时不破坏既有内容。

#### Scenario: 首次启动目录就绪

- **WHEN** 主进程在指向空目录的 `JOTLINE_DATA_DIR` 下启动
- **THEN** 该目录内出现 SPEC §3.1 全布局的子目录与文件（含 audit.jsonl 占位与 index.sqlite），可被 `ls` 与 `sqlite3` 直接核对

#### Scenario: 重复启动不破坏数据

- **WHEN** 主进程在已有笔记文件与索引行的数据目录上再次启动
- **THEN** 既有文件与索引内容保持不变

### Requirement: 文件真相源落盘

创建笔记成功时，笔记正文 MUST 以 Markdown 文件落盘于 `<vault>/notes/<item_id>.md`；文件是笔记内容的真相源，SQLite 行仅为索引。

#### Scenario: 创建后文件可见

- **WHEN** 经 API 创建一条笔记成功
- **THEN** `<vault>/notes/` 下出现以返回的笔记 ID 命名的 `.md` 文件，内容包含请求正文

### Requirement: SQLite 索引与真相源同步

创建笔记成功时，索引库中 MUST 存在对应行（可经 `sqlite3` 查询 items 表命中该 ID）；索引行缺失或损坏不构成数据丢失（真相源在文件），但正常路径下二者同步写入。

#### Scenario: 创建后索引行存在

- **WHEN** 经 API 创建一条笔记成功
- **THEN** `sqlite3 <vault>/index.sqlite "select id from items;"` 包含该笔记 ID

### Requirement: dev 首启 seed 固定项目

数据目录中不存在任何项目时，主进程启动 SHALL 自动创建一个示例项目，其 ID 为固定 ULID 字面量（写在实现与文档中，不随启动变化），保证验证命令与冒烟脚本可复述。

#### Scenario: 空库首启自动 seed

- **WHEN** 主进程在无任何项目的空数据目录上启动
- **THEN** `GET /api/projects` 返回包含该固定 ID 示例项目的列表

#### Scenario: 已有项目不重复 seed

- **WHEN** 主进程在已含项目的数据目录上启动
- **THEN** 不再新增示例项目，既有项目列表不变
