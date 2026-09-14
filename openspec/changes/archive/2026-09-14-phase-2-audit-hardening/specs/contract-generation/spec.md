## ADDED Requirements

### Requirement: 漂移防线覆盖契约源物理位置
契约漂移防线（本地 pre-commit 拦截）SHALL 覆盖契约注解的全部物理位置——包括契约 crate 内未实装域的 path stub，与已实装域 handler 所在的实现 crate（IDR-03 注解随实装迁移后，实现 crate 即契约源的主位置）；任一位置的注解源改动在未重新生成契约产物时，提交 MUST 被本地防线拦截，不得仅依赖 CI 外环兜底。

#### Scenario: 改动已实装域 handler 注解后直接提交
- **WHEN** 开发者修改实现 crate 内某已实装 handler 的 path 注解（如描述、参数或响应声明）后直接提交，未重新生成契约产物
- **THEN** pre-commit 契约检查触发，检出生成物与已提交版本的 diff，提交以非零退出被拦截

#### Scenario: 改动未实装域 stub 后直接提交
- **WHEN** 开发者修改契约 crate 内未实装域的 path stub 后直接提交，未重新生成契约产物
- **THEN** pre-commit 契约检查同样拦截（既有防线行为，作为回归保障）

### Requirement: 路由注册与文档注册端点集合一致
契约生成 SHALL 以机器可执行的守卫保证：服务路由注册的端点集合与 OpenAPI 文档注册的端点集合一致；一侧新增或遗漏端点导致集合不一致时，守卫 MUST 失败并指出差集，端点不得静默从契约产物中消失。

#### Scenario: 新增 handler 漏登记文档注册
- **WHEN** 开发者新增领域 handler 并完成路由注册，但未将其加入 OpenAPI 文档注册清单，运行一致性守卫
- **THEN** 守卫以非零退出失败，并指出该端点「路由已注册、文档未登记」的差集

#### Scenario: 两集合一致
- **WHEN** 路由注册与文档注册的端点集合完全一致
- **THEN** 一致性守卫通过（零退出），契约再生成产物的零 diff 检查不受影响
