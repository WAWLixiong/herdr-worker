# herdr-worker

一个 herdr 插件：用看板管理编码代理的工作。以仓库内 markdown 文件为数据源，herdr 提供运行时（panes、worktrees、agents）。

## Language

**Board（板）**:
仓库内 `.herdr-worker/` 目录本身——板级配置与全部任务数据的物理载体，按仓库路径绑定。
_Avoid_: 项目（project）、VK 的 workspace

**Task（任务）**:
规划的原子单元，一张看板卡片；正文描述同时是派发给 agent 的 prompt。
_Avoid_: issue、ticket

**Attempt（尝试）**:
一个任务的执行单元：一个 git worktree 加一段 agent 会话。追加对话不产生新尝试。
_Avoid_: run、session、VK 的 workspace

**Comment（评论）**:
任务下的一条讨论或 diff 审查意见，一评论一文件。
_Avoid_: note

**Column（列）**:
任务在看板上的状态位置，取值由 board.md 声明；默认六列对齐 Vibe Kanban。
_Avoid_: status（status 只是列名在 frontmatter 里的字段名）
