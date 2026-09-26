# herdr 插件可用 API 能力盘点

Type: research
Status: resolved

## Question

盘点 herdr 暴露给插件的全部能力面（读 `E:\projects\herdr\docs\next\website\src\content\docs\` 下的 cli-reference、socket-api、agent-automation、concepts、plugins 等文档，必要时对照 `E:\projects\herdr\src\` 源码）：worktree 生命周期（创建 / 清理 / 事件）、pane 与 tab 操作、agent 启动 / 提示 / 状态查询（working / blocked / idle 如何判定、如何轮询或订阅）、插件可订阅的事件清单、agent 用量 / token / 费用数据是否可查（这决定雾区「用量统计」能否升格为票）、startup hook、键位绑定、popup / overlay / split 各形态 pane 的约束。产出物写入 `.scratch/herdr-worker/research/02-herdr-api-inventory.md` 并在 Answer 中链接。

## Answer

herdr 的插件 API 就是完整 CLI + 原生 socket（newline-JSON，Windows 命名管道），核心能力面齐全：worktree 全生命周期（`worktree.list/create/open/remove` + `worktree.created/opened/removed` 事件，均可用 `[[events]]` 钩子）、完整 pane/tab/workspace 拓扑操作（split/swap/move/zoom/resize、`layout.apply` 声明式布局）、agent 启动（`agent start` 需已有空闲 shell pane，22 种 kind）、提示与消息（`agent prompt [--wait]`、`agent send-keys`，agent 间通信即互相调 CLI）、状态判定（集成 lifecycle hook 权威，否则 bottom-buffer screen manifest 检测；idle/done 区别仅在被看见与否）、轮询（`agent list/get/wait`）与订阅（`events.subscribe` 的 `pane.agent_status_changed` 等 + 插件事件钩子共 22 个可挂钩事件名，排除 pane.updated/layout.updated/metadata 等高频事件）。两个硬缺口决定设计边界：(1) **用量/token/费用完全不可查** —— AgentInfo 无任何 usage 字段，"tokens" 仅为插件自报的 display-only 元数据，用量统计功能只能靠 pane 输出抓取或读 agent 本地文件，不能作为原生票；(2) 无 herdr 托管的插件存储与常驻后台进程（startup hook 是一次性的），看板状态须自管于 `HERDR_PLUGIN_STATE_DIR`，超出 22 个钩子事件的实时刷新需要长连 socket 的 TUI pane 进程承载。full inventory at research/02-herdr-api-inventory.md
