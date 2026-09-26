# herdr-worker 插件功能规格（map）

Label: wayfinder:map

> **地图完成**（2026-09-26）：10/10 票解决，雾区清空。终点交付物 = [SPEC.md](../../SPEC.md)，可交实现规划。

## Destination

一份可直接交给实现规划的**功能范围规格（spec）**：Vibe Kanban 能力逐项对照（采纳 / 放弃 / 改造 + 理由），加上 herdr-worker 插件形态草图（各能力分别落在 pane / action / event / 存储的哪里）。本图只做决定，不写实现代码。

## Notes

- 领域：herdr 插件——**编码代理任务看板**；对标 Vibe Kanban；**能力等价、纯 TUI 交付**（herdr 插件 v1 无非终端 UI）。
- 技术栈已定：**Rust + ratatui**。本仓 `E:\projects\herdr-worker` 是插件未来的家（当前为空）。
- herdr 源码与文档在相邻目录 `E:\projects\herdr`（研究票可读其 docs/next 与 src）。**不参考 tsk 的实现**。
- 每个会话开工前先读本图；grilling 票要同时调用 Skill 工具的 "grilling" 与 "domain-modeling"；research 票调用 "research"；prototype 票调用 "prototype"。
- 术语以 herdr 原生词汇为准（worktree / pane / tab / workspace / agent）。VK 的 "workspace"（每任务执行环境：分支+终端+dev server）不得与 herdr 的 workspace 混用；插件新概念在 spec 定稿阶段写入 `CONTEXT.md`。
- 交流用中文。
- 事实背景：Vibe Kanban 已宣布 sunset（vibekanban.com/blog/shutdown）；herdr-worker 定位其 TUI 继任者。
- 已定的产品级决定（详见各票）：任务数据 = 仓库内 markdown 文件；执行 = 全托管编排；agent 驱动 API = 一等公民。

## Decisions so far

<!-- 一行一票，closed 票的要点，详情在票里 -->

- [Vibe Kanban 完整能力清单](issues/01-vk-capability-inventory.md)：三层模型 Issue→Workspace→Session（issue 描述即 prompt，可扇出到多个 workspace）；非交互 API 是本地 stdio MCP server，能力完整（issue/tag/assignee/关系 CRUD + `start_workspace` + `run_session_prompt`/`get_execution` 提交-轮询原语）；审查评论批量投递非实时；搜索仅标题匹配；六列默认状态、完全可定制。全文：[research/01](research/01-vk-capability-inventory.md)
- [herdr 插件可用 API 能力盘点](issues/02-herdr-api-inventory.md)：插件可用全量 CLI + raw socket，无 SDK 无沙箱；**22 个可订阅事件**（含 `pane.agent_status_changed`、`worktree.created/opened/removed`，无 replay，需 `session.snapshot` 引导）；agent 状态由 lifecycle hook 或 TOML screen manifest 判定，`agent list/get/wait` 轮询 + 事件订阅皆可；编排闭环 `worktree.create → agent start（需已有 idle shell pane）→ agent prompt --wait → worktree.remove` 全支持；**用量 / token / 费用数据不暴露**；无插件存储 API（自理 `HERDR_PLUGIN_STATE_DIR`）；startup hook 一次性非守护，长活连接意味着 TUI pane 本身就是常驻进程；32 并发插件命令上限。全文：[research/02](research/02-herdr-api-inventory.md)
- [成熟的 TUI diff 方案调研](issues/03-tui-diff-options.md)：无成熟 ratatui diff 组件；推荐 **v1 = 兄弟 pane 外部渲染器**（delta → difftastic → `git diff` 回退，两次 herdr CLI 调用即可），零渲染代码、原生交互；需要 hunk 级任务操作时再基于 imara-diff / similar + syntect 自建（gitui 有先例）；不要库内嵌 delta/difft（无 API）、不要复用 lazygit 代码。全文：[research/03](research/03-tui-diff-options.md)
- [任务数据 schema：仓库内 markdown 文件格式](issues/04-task-data-schema.md)：一任务一目录 `.herdr-worker/tasks/NNNN-slug/`（task.md + attempts/ + comments/，原子追加零冲突）；task.md 放弃 assignee/related/duplicate、保留 parent+blocked-by、正文=描述=prompt；默认六列对齐 VK、board.md frontmatter 可配；attempt = 一个 worktree + 一段 agent 会话；板按 repo 路径绑定、任务文件默认随 git 提交（worktree 里 agent 可自发现任务卡）。
- [执行编排模型：任务如何跑起来](issues/05-orchestration-model.md)：启动三步 `worktree create`（分支 `{prefix}/{id}-{slug}`）→ root pane `agent start` → `agent prompt`，任务自动进 in-progress；状态两层——活状态订阅 `pane.agent_status_changed`（working→running、blocked→blocked），**idle≠完成**，终态只认 agent 自报或人确认（pane 消失无自报→failed）；自报 done 自动进 review；重跑/换 agent=新 attempt；取消保留 worktree、不做自动清理；插件只管自己建的 worktree。
- [Agent 驱动 API 设计](issues/06-agent-api-design.md)：双通道——查询直接读 markdown（markdown 即 API），变更走六件套 actions（create-task/move-task/add-comment/report-result/start-attempt/stop-attempt，`herdr plugin action invoke` 调用、文件锁保并发）；人类手改文件是一等公民；不做权限模型（herdr 信任模型）。
- [用量统计路线：出局、抓取还是自报](issues/09-usage-tracking-route.md)：**轻量自报**——report-result 带可选 `usage:` 字段（attempt 级、自由格式），TUI 有则显示；不做抓取、不做聚合面板。
- [看板 TUI 形态原型](issues/07-board-tui-prototype.md)：混搭裁决——主看板 = 经典看板（三行卡片：id+标题 / agent+状态徽章+次数 / 标签+usage），详情浮层沿用主从排版（meta→正文→attempts 时间线→comments）；B 泳道列表 v1 不做；交互词汇 `←→/hjkl · ↑↓/jk · Enter/Esc · s 启动 · r 自报审阅 · d diff→兄弟pane · / 标题搜索 · tag: 过滤 · H/J/K/L 移卡 · q`；v1 无列编辑器（手改 board.md + TUI 自动重载）。原型资产：prototype/07-board/（cargo run / --snap）
- [主动提醒：通知与视图投影要不要用](issues/10-notifications-and-projections.md)：`notification.show` **要**（agent 转 blocked 弹通知）；`agent.view.set` **v1 不做**（信息已有两处展示）；`workspace.report_metadata` **要**（任务 worktree 侧栏挂「任务号+状态」chip，attempt 状态变化时更新）。
- [汇总撰写 herdr-worker 功能规格](issues/08-write-spec.md)：**SPEC.md 定稿**（25 行 VK 对照表 + manifest 草案 + event-hooks-as-brain 架构分界）；装配补决：顺序 attempt（一任务至多一个活 attempt）+ `f` follow-up 键（携带未投递评论）；发布 = GitHub/marketplace/cargo build/MIT/英文 UI v1，雾区「发布与分发」「i18n」就此清空。

## Not yet specified

<!-- 雾区：还问不出精确问题的地带，前沿推进后逐步升格为票 -->

（无——地图完成，路已清）

## Out of scope

<!-- 超出终点的范围边界，不回头 -->

- **内置浏览器预览**（VK 的 app 预览 + devtools + 设备模拟）：TUI 无从承载，出局。
- **PR 创建 / 合并**：用户明确不需要。
- **多人协作 / 云同步**：herdr 是本地单用户运行时，协作属另一个产品。
- **多仓库支持**：v1 明确单仓（一个 herdr workspace 一块板）。
- **多板组织**：v1 一块板，用标签 / 过滤分组。
