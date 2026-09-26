# Vibe Kanban 完整能力清单

Type: research
Status: resolved

## Question

从 Vibe Kanban 的官方文档（vibekanban.com/docs）与仓库（github.com/BloopAI/vibe-kanban）提取一份**完整能力清单**，作为 herdr-worker spec 的对照基准。要求覆盖：任务 / 看板数据模型（字段、状态列、attempts、评论）、执行编排（worktree / 分支 / 终端 / dev server）、diff 审查与反馈、agent 类型接入方式、非交互 / agent 驱动 API、搜索过滤、用量统计、配置项。逐项给出出处链接。产出物写入 `.scratch/herdr-worker/research/01-vk-capability-inventory.md` 并在 Answer 中链接。

## Answer

Vibe Kanban（正在 sunset，转为开源社区维护）的能力面为：Issue→Workspace→Session 三层模型 —— issue 是规划单元（标题/富文本描述/优先级 urgent-high-medium-low/多 assignee/标签/Simple ID/评论+表情/一级子任务，6 个默认状态列 Backlog·To do·In progress·In review·Done·Cancelled，列可增删改排序），描述直接成为 agent prompt；workspace 是执行单元（每仓库一个 git worktree + 自动命名工作分支，支持多仓库、setup/cleanup/dev-server 脚本、集成终端、预览浏览器含点击定位组件与 Eruda devtools），一个 issue 可挂多个 workspace 并行；session 是 workspace 内的 agent 会话（共享文件但不共享上下文，支持队列消息、编辑重发分支、审批工作流）。Diff 审查为双栏/统一视图 + 行内评论（评论批量随下一条 chat 消息发给 agent 后即消耗入历史），PR 经 gh CLI 创建（AI 生成描述、draft、自动关联、状态徽标）。接入 10 种 agent（Claude Code、Codex、Copilot、Gemini、Amp、Cursor、OpenCode、Droid、CCR、Qwen），认证完全委托给各 agent 自己的 CLI，VK 侧用命名 profile/variant 配置（plan 模式、sandbox/approval、reasoning effort、每 profile 环境变量指向第三方 provider）+ 每 agent MCP server JSON。**非交互 API 是本地 stdio MCP server（`npx vibe-kanban --mcp`），无 REST**：issue/标签/assignee/关系（blocking/related/has_duplicate）全 CRUD、repo 脚本编辑、workspace/session 生命周期，`start_workspace` 省略 prompt 时自动用关联 issue 的标题+描述，`run_session_prompt`+`get_execution` 构成提交-轮询执行原语，且 agent 可在 workspace 内经 `get_context` 自我发现并用同一 API 建.issue（规划 agent 模式）。搜索仅标题（大小写不敏感部分匹配）+ 优先级/assignee/标签下拉过滤（类别内 OR、跨类别 AND）+ Manual/Priority/Created/Updated/Title 排序；用量统计仅每 session 上下文仪表（如 27K/200K）与 agent 原生 `/cost`、`/status` 斜杠命令，无聚合成本看板。注：github.com 与 raw.githubusercontent.com 在本环境持续不可达，全部内容取自 vibekanban.com/docs 官方文档（一手来源）。完整清单（逐项带出处链接，含"会令 spec 作者意外的 10 点"）见 research/01-vk-capability-inventory.md
