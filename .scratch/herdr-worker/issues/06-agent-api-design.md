# Agent 驱动 API 设计

Type: grilling
Status: resolved
Blocked by: 02, 04

## Question

已定：agent 驱动 API 是一等公民。设计：暴露哪些操作（建任务 / 移动 / 评论 / 查询状态 / 上报结果）、调用通道（`herdr plugin action invoke` vs agent 直接读写 markdown 文件 vs 两者并用）、操作与文件格式的关系、agent 自建任务的命名空间 / 权限约定。

## Answer

（2026-09-26 与用户敲定）

### 通道与分工：双通道，文件是唯一事实源

- **查询 = 直接读 markdown**（"markdown 即 API"）——agent 在自己的 worktree 里就能读到任务卡
- **变更 = manifest actions**（`herdr plugin action invoke herdr-worker.<action>`）——action 内用文件锁保证并发安全（id 分配、终态写入）
- **人类手改文件是合法一等公民**，TUI 重读即同步

比 VK 的 MCP-only 更简单也更透明；代价是 action 需自实现文件锁（v1 无 herdr 存储事务 API）。

### v1 操作清单（六件套）

| action | 行为 |
| --- | --- |
| `create-task` | 文件锁内分配递增 id，写任务目录 |
| `move-task` | 改 status（校验取值必须在 board.md 声明列内） |
| `add-comment` | 追加评论文件（author = 调用 agent） |
| `report-result` | 写 attempt 终态 + 结果摘要，可选 `usage:` 字段（见 09） |
| `start-attempt` / `stop-attempt` | agent 可自己开任务自己干（VK planning-agent 模式） |

### 权限与命名空间

**不做权限模型**（herdr 信任模型：进程即用户，插件与 agent 同权）；agent 身份只体现在 author 字段，不引入命名空间。
