# 04: 编排 `s` 键——全托管三步启动

**What to build:** 板上按 `s` 启动选中任务：`worktree create --branch {prefix}/{id}-{slug} --label …`（经 `HERDR_BIN_PATH`）→ 写 attempt N.md（running，记 pane/workspace 标识）→ root pane `agent start`（board.md default_agent，启动时可覆盖）→ `agent prompt`（任务正文即 prompt）→ 任务自动入 in-progress → `workspace.report_metadata` 挂侧栏 chip（任务号+状态）。`act start-attempt` 走同一编排路径（planning-agent 模式）。

**Blocked by:** 02, 03

**Status:** ready-for-agent

- [ ] 三步编排 + 每步错误处理（herdr 不可达 / worktree 失败 / agent start 失败时回滚 attempt 文件）
- [ ] attempt 文件记录 agent/branch/started/status + pane 与 workspace 标识（供 05 事件定位）
- [ ] 启动后任务自动入 in-progress；侧栏出现 chip
- [ ] `act start-attempt` 同路径；重复启动（已有活 attempt）被拒绝并提示
- [ ] 真机端到端：按 s 后 worktree 出现、agent 跑起来、卡片与 chip 正确
