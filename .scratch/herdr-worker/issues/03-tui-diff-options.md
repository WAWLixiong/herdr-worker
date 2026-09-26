# 成熟的 TUI diff 方案调研

Type: research
Status: resolved

## Question

调研市面上成熟的 TUI diff 方案，评估各自嵌入 herdr-worker（Rust + ratatui）的可行性：delta、difftastic、lazygit 的 diff 视图、ratatui 生态的 diff 组件 / crate、以及“spawn 一个外部 pager pane”的路线。给出每条路线的交互能力（滚动 / hunk 级操作 / 行内定位）、嵌入成本、维护活跃度。产出物写入 `.scratch/herdr-worker/research/03-tui-diff-options.md` 并在 Answer 中链接。

## Answer

五条路线里，delta 与 difftastic 都是纯 bin crate（crates.io `has_lib: false`）、流式 ANSI 渲染器且无 hunk 操作（difftastic 明确不产 patch），只能进程级或 ANSI 捕获式嵌入；lazygit 的 diff 视图是 Go 单体不可提取，但它的“stdin filter / ext-diff / raw git”三通道渲染器架构是成熟模式；ratatui 生态不存在成熟 diff 组件，真实形态是 imara-diff（Helix 同源）或 similar 这类算法库 + 自建视图，gitui（ratatui + hunk/行级 stage）已把这条路走通。**推荐 v1 走“外部渲染器独立 pane”路线**：板内动作经 herdr CLI `pane split --no-focus` + `pane run` 把 `git diff | delta`（发现链 delta → difft → 裸 git diff 兜底）spawn 到兄弟 pane——零渲染代码、零许可风险、完整原生交互且尊重用户已有配置；当后续产品需要“hunk 挂任务动作”时，再以 imara-diff/similar + ratatui + syntect 自建板内视图（gitui 先例，数天级）。不推荐：库级嵌入 delta/difftastic（无 API）、复用 lazygit 代码、tui-term 板内 PTY（与 herdr pane 重复）。完整调研报告见 research/03-tui-diff-options.md。
