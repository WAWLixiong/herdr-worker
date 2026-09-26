# 用量统计路线：出局、抓取还是自报

Type: grilling
Status: resolved

## Question

研究结论（见「herdr 插件可用 API 能力盘点」）：herdr 不暴露 agent 的 token / 费用数据，`AgentInfo` 无用量字段。VK 也只有会话内 context 仪表 + agent 原生 slash 命令。三条路选一：

- **(a) v1 出局**：spec 记为 non-goal，用量交给各 agent 自己的命令（`/cost` 等）
- **(b) 输出抓取**：插件解析 agent 终端输出中的用量报告——脆弱、依赖各 agent 输出格式
- **(c) agent 自报**：借助已定的 agent 驱动 API——agent 完成任务时把用量写入任务文件 / 调用 API 上报

需要权衡：值不值得做、做到什么粒度（每 attempt / 每任务累计）。

## Answer

（2026-09-26）选 **(c) 轻量自报**。

- 机制：`report-result` 携带可选 `usage:` 字段（attempt 级、自由格式，如 `tokens: 45k, $0.12`），TUI 有则显示无则略
- 明确不做：输出抓取（脆弱、各家格式不同）、聚合费用面板（VK 也没有）
- 理由：herdr 不暴露用量数据（[research/02](../research/02-herdr-api-inventory.md) §5 是硬缺口）；report-result 本就要写 attempt 文件，加一个可选字段边际成本近零。粒度 = 每 attempt；任务级累计（若有需要）由 TUI 聚合显示，不另存数据。
