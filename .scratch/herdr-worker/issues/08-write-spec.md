# 汇总撰写 herdr-worker 功能规格

Type: grilling
Status: resolved
Blocked by: 04, 05, 06, 07, 09, 10

## Question

把全部已决事项装配成终稿 spec：VK 能力逐项对照表（采纳 / 放弃 / 改造 + 理由）、herdr-worker 插件形态草图（pane / action / event / 存储各承载什么）、数据 schema、编排模型、agent API、TUI 形态、发布注意事项、`CONTEXT.md` 词汇表。与用户过一遍定稿。这是本图的终点交付物。

## Answer

（2026-09-26 定稿，用户确认）

**SPEC 已装配至仓库根 [SPEC.md](../../SPEC.md)**：25 行 VK 能力对照表（采纳/改造/放弃 + herdr-worker 形态）· 插件形态草图（manifest 草案 + "event hooks 是状态回流大脑、TUI 只是读者+发令者"的架构分界）· 数据 schema · 编排模型 · Agent API（六件套）· TUI · 主动提醒 · 发布与分发（GitHub + marketplace / `cargo build --release` 要求 Rust 工具链 / **MIT** / **v1 英文 UI** + 中文 README）· 实现期待办 · 决议溯源附录。

装配补决（用户已确认）：

1. **顺序 attempt**——一任务同时至多一个活 attempt；并行靠多任务，不靠单任务扇出（VK 的扇出模式放弃）
2. **`f` follow-up 键**——详情页向该 attempt 的 agent 追加 prompt，自动携带未投递评论（VK“行内评论批量投递”的等价物）

雾区「发布与分发」「i18n」随本票裁决并入 spec §9，**雾区清空**。全部 10 票解决，地图完成。
