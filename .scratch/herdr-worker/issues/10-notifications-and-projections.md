# 主动提醒：通知与视图投影要不要用

Type: grilling
Status: resolved

## Question

编排模型已定（见 05），但 herdr 还有三个周边能力没裁决（见 research/02 §8/§10）：

- `notification.show`——任务 agent 转 blocked 时，要不要弹 herdr 通知提醒人来答？
- `agent.view.set`——声明式投影 herdr 内置 Agents 视图（按状态/工作区过滤排序，`source: "plugin:..."`，不持久、startup hook 重放）——要不要投一块"我的看板 agent"视图？
- `workspace.report_metadata`——在 workspace 侧栏挂显示 token（≤16 键）——要不要给每个任务 worktree 挂任务号/状态 chip？

逐个裁决用 / 不用 / 缓一缓（进雾区）。

## Answer

（2026-09-26 与用户裁决）

| 能力 | 裁决 | 说明 |
| --- | --- | --- |
| `notification.show` | **要** | 任务 agent 转 blocked 时弹 herdr 通知——正是 herdr“永远不用找卡住的那个”的核心价值；频率策略（每 attempt 首次 blocked 提醒一次，避免反复轰炸）实现期定 |
| `agent.view.set` | **v1 不做** | 看板 pane + 侧栏 chip 已覆盖同一信息，第三展示位收益低；临时投影还需 startup hook 重放，维护成本不值 |
| `workspace.report_metadata` | **要** | 启动 attempt 时给任务 worktree 挂侧栏 chip（任务号+状态，如 `$hw:0042 ⛔`），attempt 状态变化时更新——在 herdr 原生侧栏一眼看出哪个 worktree 是哪个任务 |
