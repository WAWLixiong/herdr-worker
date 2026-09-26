# 05: sync 事件大脑——状态回流与 blocked 通知

**What to build:** manifest `[[events]]`（pane.agent_status_changed / pane.exited / worktree.created / worktree.opened / worktree.removed）触发 `herdr-worker sync` 一次性进程：读 `HERDR_PLUGIN_EVENT_JSON` → 定位对应 attempt（04 记录的 pane/workspace 标识）→ 锁内更新 attempt 状态 → 转 blocked 时 `notification.show` → 更新侧栏 chip。呈现"⏳ 空闲待确认"（agent idle ≠ 完成）；pane 消失且无自报 → attempt 保守标 failed。

**Blocked by:** 04

**Status:** done

- [x] 事件 JSON → attempt 定位（pane_id / workspace_id 映射）单测（样例 EventEnvelope）
- [x] working→running、blocked→blocked 映射；blocked 首次转换弹通知（避免重复轰炸）
- [x] idle 呈现为"⏳ 空闲待确认"（attempt 保持 running；sync 写 agent_state 子状态，TUI 徽章消费）
- [x] pane.exited / worktree.removed 且无终态自报 → failed
- [x] chip 随状态变化更新
- [x] sync 快进快出（无长驻）；真机端到端：真跑一个 agent 观察状态流动

> 偏差记录：①钩子信封的事件名是 snake_case（`pane_agent_status_changed`），与文档点分形式并存——两种都收；②worktree.created/opened 未注册（它们是编排自身动作，注册=每次建 worktree 白起一个进程）；③fresh opencode 吞首条粘贴的落袋重试放在了编排侧（orchestrate.rs），连带进本票验证。
