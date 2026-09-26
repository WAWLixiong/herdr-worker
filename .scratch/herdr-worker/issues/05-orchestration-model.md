# 执行编排模型：任务如何跑起来

Type: grilling
Status: resolved
Blocked by: 02

## Question

已定：全托管——看板上一个动作 = 建 worktree + 开 pane + 派 agent + 状态自动回流到卡片。细化：启动任务的确切流程（调哪些 herdr API）、agent 如何选定、状态回流机制（事件订阅 vs 轮询 vs 两者并用）、blocked / idle 时卡片如何呈现、attempts 重跑语义（复用 worktree 还是新开）、取消与清理、与 herdr 原生 worktree 管理的边界（谁拥有 worktree 的生命周期）。以「herdr 插件可用 API 能力盘点」的事实为准绳。

## Answer

（2026-09-26 与用户敲定；事实准绳 [research/02](../research/02-herdr-api-inventory.md)）

### 启动流程（三步）

1. `worktree create`——分支名 `{prefix}/{id}-{slug}`（前缀来自 board.md，对齐 VK 命名法），label 标识插件所有权；一步返回 workspace + tab + root pane
2. 在返回的 root pane 里 `agent start`（root pane 即天然满足"已有空闲 shell pane"前提）
3. `agent prompt`——任务正文（task.md body）即 prompt

启动后**任务自动移入 in-progress 列**（对齐 VK 的状态自动流转）。

### Agent 选定

board.md 定 `default_agent`（herdr agent kind，如 claude-code）；每次启动 attempt 时可在 TUI 覆盖。VK 的 profiles/variants（env/args 变体）**v1 出局**——herdr 各 agent 自带配置。

### 状态回流（两层，关键设计）

- **活状态**：订阅 `pane.agent_status_changed`（无 replay，`session.snapshot` 引导）+ `worktree.*` 事件；映射 working→running、blocked→blocked。**idle ≠ 完成**——只是"本轮说完话"，attempt 仍算 running，TUI 显示"agent 空闲待确认"
- **终态**（done/failed/cancelled）：**不由 idle 推断**——由 agent 经 `report-result` 自报，或人在 TUI 确认；pane 消失且无自报 → 保守标 failed
- 工作流：agent 自报 done → 任务自动进 **review** 列；人审通过 → done

### attempt 边界

失败重跑 / 换 agent / 推倒重来 = **新 attempt（新 worktree）**；同一 worktree 内 follow-up prompt = 原 attempt 继续。

### 取消、清理、所有权

- 取消 = send-keys ctrl+c → release，attempt 标 cancelled，worktree 保留
- 任务完成后 TUI 提供"清理 worktree"动作（`worktree remove`，分支永不删）；**不做自动清理**（不学 VK 的 orphan reaper）
- 插件只管理自己创建的 worktree（按分支前缀 / label 识别）；用户手工建的 worktree 不进板
