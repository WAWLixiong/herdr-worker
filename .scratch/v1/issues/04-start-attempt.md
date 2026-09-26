# 04: 编排 `s` 键——全托管三步启动

**What to build:** 板上按 `s` 启动选中任务：`worktree create --branch {prefix}/{id}-{slug} --label …`（经 `HERDR_BIN_PATH`）→ 写 attempt N.md（running，记 pane/workspace 标识）→ root pane `agent start`（board.md default_agent，启动时可覆盖）→ `agent prompt`（任务正文即 prompt）→ 任务自动入 in-progress → `workspace.report_metadata` 挂侧栏 chip（任务号+状态）。`act start-attempt` 走同一编排路径（planning-agent 模式）。

**Blocked by:** 02, 03

**Status:** done

- [x] 三步编排 + 每步错误处理（herdr 不可达 / worktree 失败 / agent start 失败时回滚 attempt 文件）
- [x] attempt 文件记录 agent/branch/started/status + pane 与 workspace 标识（供 05 事件定位）；report-result 重写时保留锚点字段
- [x] 启动后任务自动入 in-progress；侧栏出现 chip
- [x] `act start-attempt` 同路径；重复启动（已有活 attempt）被拒绝并提示
- [x] 真机端到端：按 s 后 worktree 出现、agent 跑起来、卡片与 chip 正确

> e2e 发现并修复：agent start 的"就绪"早于 opencode 欢迎屏可接收输入，首条 prompt 会被吞成草稿——编排改为等 idle（上限 15s）再 prompt（commit 12b792e）。另：prompt 页脚携带 API 坐标（二进制绝对路径 + 主板 --path + id/attempt），弥补 worktree 内无 target/ 与 .herdr-worker 副本歧义。
