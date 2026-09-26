# herdr-worker 功能规格

> 状态：定稿审批中 · 2026-09-26
> 决议来源：wayfinder 地图 `.scratch/herdr-worker/`（10 张决议票 + 3 篇调研，溯源见附录 A）
> 对标基线：Vibe Kanban（已 sunset）；术语见 [CONTEXT.md](CONTEXT.md)

## 0. 一句话定位

herdr-worker 是一个 herdr 插件：**用看板管理编码代理的工作**。以仓库内 markdown 文件为任务数据源（markdown 即 API），herdr 提供运行时（panes、worktrees、agents）。目标：**能力对标 Vibe Kanban，能力等价、纯 TUI 交付**。

## 1. 范围与非目标

- **范围**：单仓库一块板；任务规划（看板）+ 执行编排（worktree+agent 全托管）+ 状态回流 + diff 审查 + agent 驱动 API。
- **非目标**（v1 明确出局）：内置浏览器预览、PR 创建/合并、多人协作/云同步、多仓库、多板、用量抓取/聚合面板、agent profiles/variants、TUI 列编辑器、B 泳道列表视图。

## 2. Vibe Kanban 能力对照表

| VK 能力 | 裁决 | herdr-worker 形态 |
| --- | --- | --- |
| 三层模型 Issue→Workspace→Session | **改造** | 两级：Task→Attempt（attempt = 一个 worktree + 一段 agent 会话；**顺序执行**，一任务同时至多一个活 attempt——并行靠多任务，不靠单任务扇出） |
| 描述即 agent prompt | **采纳** | task.md 正文就是 prompt |
| 六默认列（Backlog/Cancelled 隐藏） | **采纳** | `backlog/todo/in-progress/review/done/cancelled`，board.md frontmatter 可配 |
| 自定义列界面（增删改排/着色） | **改造** | v1 手改 board.md，TUI 监听文件变化自动重载 |
| 优先级 / 标签 | **采纳** | priority 四档；tags 过滤用（无颜色自定义） |
| Assignee 多人指派 | **放弃** | 本地单用户；执行者记在 attempt |
| 子任务（一层） | **采纳** | `parent` 字段 |
| 关系 blocking/related/duplicate | **改造** | 只留 `blocked-by`（related/duplicate YAGNI） |
| 一 issue 扇出多 workspace 并行 | **放弃** | 见三层模型行 |
| 一 workspace 多 session | **改造** | 同 attempt 内 follow-up prompt（不新开文件） |
| 内置终端 | **天然拥有** | herdr pane 即终端 |
| 内置浏览器预览 / devtools / 设备模拟 | **放弃** | TUI 无从承载 |
| dev server 生命周期 + URL 探测 | **放弃 v1** | 需要时在 pane 里手动起 |
| Diff 审查视图（语法高亮/双栏） | **改造** | 兄弟 pane 外部渲染器：delta → difftastic → `git diff` 回退（两次 herdr CLI 调用，零渲染代码）；hunk 级操作留给未来（imara-diff/similar 路线） |
| 行内评论批量投递给 agent | **改造** | 评论是文件（可带 file/hunk 定位）；详情页 follow-up prompt 自动携带未投递评论（VK 批量投递的等价物） |
| Agent profiles / variants / env 注入 | **放弃** | herdr 各 agent 自带配置；board.md 定 `default_agent`，每次 attempt 可覆盖 |
| Agent 认证 | **采纳同思路** | 完全委托各 agent CLI，插件不碰凭证 |
| 非交互 API（本地 MCP） | **改造** | 双通道：读 = 直接读 markdown；写 = 六件套 plugin actions（见 §6） |
| 搜索（仅标题）+ 过滤器 | **改造** | `/` 标题子串 + `tag:` 前缀，v1 两样 |
| 用量（context 仪表/`/cost`） | **改造** | `report-result` 可选 `usage:` 字段（attempt 级自由格式），卡片/详情有则显示 |
| 状态自动流转（开工/PR 触发） | **改造** | 启动→in-progress；agent 自报 done→review；人按 `r` 确认→done |
| 通知（浏览器推送） | **改造** | `notification.show`：attempt 转 blocked 弹 herdr 通知 |
| （VK 无）worktree 侧栏标识 | **新增** | `workspace.report_metadata` 给任务 worktree 挂 chip（任务号+状态），状态变化时更新 |
| PR 创建（AI 描述）/合并/rebase 冲突对话框 | **放弃** | 用户明确不需要 |
| worktree 孤儿回收 / 静默重建 | **放弃** | 不做自动清理；完成后手动"清理 worktree"（分支永不删） |
| 云同步 / 团队 / 多项目 | **放弃** | herdr 是本地单用户运行时 |

## 3. 插件形态草图

单二进制 `herdr-worker`（Rust + ratatui），三个入口角色；manifest 草图：

```toml
id = "herdr-worker"
name = "herdr-worker"
version = "0.1.0"
min_herdr_version = "0.7.0"
platforms = ["linux", "macos", "windows"]
license = "MIT"

[[build]]
command = ["cargo", "build", "--release"]

# TUI：读者 + 发令者（不做状态回流的大脑）
[[panes]]
id = "board"
title = "Board"
placement = "overlay"                    # 临时放大层：q 关闭后焦点/缩放自动还原；打开时可覆盖 split/tab
command = ["herdr-worker", "board"]

# 六件套（agent 与人经 herdr plugin action invoke 调用）
[[actions]]
id = "create-task"                       # 同型：move-task / add-comment /
contexts = ["workspace"]                 #       report-result / start-attempt / stop-attempt
command = ["herdr-worker", "act", "create-task"]

# 状态回流的大脑：一次性短命进程
[[events]]
on = "pane.agent_status_changed"         # 同型：pane.exited / worktree.created /
command = ["herdr-worker", "sync"]       #        worktree.opened / worktree.removed
```

**架构分界（地图 08 装配决定）**：状态回流的**大脑是 event hooks**（`sync` 子命令：读 `HERDR_PLUGIN_EVENT_JSON` → 文件锁内更新 attempt → blocked 弹通知 → 更新侧栏 chip）。TUI 只是**读者+发令者**：监听 `.herdr-worker/` 文件变化重绘（notify），按键调 herdr CLI（`HERDR_BIN_PATH`）。板没开着，状态照常流动。已知约束：事件风暴可能触 32 并发插件命令上限（单用户规模罕见，接受）；sync 必须快进快出。

**存储**：任务数据 = 仓库内 `.herdr-worker/`（随 git 提交）；运行时状态（过滤器等）= `HERDR_PLUGIN_STATE_DIR`（不进仓库）。无 herdr 存储事务 API——所有写入走插件自实现的**文件锁**。

## 4. 数据 schema

```
<repo>/
└── .herdr-worker/
    ├── board.md          # 板级配置：columns（列名/顺序/隐藏）、branch_prefix、default_agent、diff_tool
    └── tasks/
        └── 0042-fix-login/
            ├── task.md
            ├── attempts/
            │   └── 1.md
            └── comments/
                └── 20260926-1430.md
```

- **task.md** frontmatter：`id`（4 位递增）/ `title` / `status`（必须在 board.md 列内）/ `priority` / `tags[]` / `parent`（一层）/ `blocked-by[]` / `order`（列内手动排序）/ `created` / `updated`；**正文 = 描述 = prompt**
- **attempts/N.md**：`n` / `agent` / `branch` / `started` / `ended` / `status`（running/blocked/done/failed/cancelled）；正文 = 结果摘要；可选 `usage:`
- **comments/**：一评论一文件：`author`（human 或 agent id）/ `time` / 可选 `file`/`hunk`（diff 审查评论）/ 正文
- **板绑定**：按 repo 路径，无注册配置；同仓多 workspace 见同一块板
- **git**：默认提交（clone 即得；worktree 里 agent 自发现任务卡）；个人草稿板可 gitignore（README 注明）

## 5. 编排模型

- **启动三步**：`worktree create --branch {prefix}/{id}-{slug} --label …`（一步返回 workspace+tab+root pane）→ root pane `agent start`（agent 取 board.md default_agent，可覆盖）→ `agent prompt`（任务正文）。启动后任务自动入 in-progress
- **状态两层**：活状态 = 订阅 `pane.agent_status_changed`（`session.snapshot` 引导，无 replay）：working→running、blocked→blocked；**idle ≠ 完成**（"空闲待确认"，attempt 仍 running）。终态 = agent 自报（report-result）或人确认；pane 消失且无自报 → 保守 failed
- **attempt 边界**：失败重跑 / 换 agent = 新 attempt（新 worktree）；同 worktree follow-up = 原 attempt 继续；顺序执行
- **取消**：send-keys ctrl+c → release → 标 cancelled，worktree 保留
- **清理**：手动动作 `worktree remove`（分支永不删）；插件只管自己建的 worktree（前缀/label 识别）

## 6. Agent 驱动 API

**双通道，文件是唯一事实源**：

- **读**：直接读 markdown（"markdown 即 API"；agent 在自己 worktree 里就能读任务卡）
- **写**：六件套（文件锁保并发）——`create-task`（锁内分配 id）/ `move-task`（校验列名）/ `add-comment` / `report-result`（终态+摘要+可选 usage）/ `start-attempt` / `stop-attempt`（planning-agent 模式：agent 自己开任务自己干）
- **调用方式（实现期修正）**：`herdr plugin action invoke` 不支持传参，故六件套 = **直接执行插件二进制** `herdr-worker act <action> --flags`（路径可经 `herdr plugin list --json` 的 plugin_root 发现，即 `<root>/target/release/herdr-worker`）；manifest 只注册无参动作（如 `open-board`，兼作键位绑定目标）
- **人改文件 = 一等公民**，TUI 重读即同步
- **无权限模型**（herdr 信任模型：进程即用户）；agent 身份只体现在 author

## 7. TUI（原型裁决，资产 `.scratch/herdr-worker/prototype/07-board/`）

- **主看板 = 经典看板**：列卡片墙；卡片三行（id+标题 / agent+状态徽章+attempt 数 / 标签+usage）
- **详情 = Enter 浮层，主从排版**：meta → 正文（prompt）→ attempts 时间线 → comments（人绿 agent 蓝）→ `d` 提示
- **键位**：`←→/hjkl` 列 · `↑↓/jk` 卡 · `Enter/Esc` 详情 · `n` 新建（标题+内联 `#tag`，落 todo，建后跳选） · `e` 编辑正文（兄弟 pane 开 `$EDITOR`） · `s` 启动 · `r` 自报审阅 · `d` diff→兄弟 pane · `f` follow-up prompt（自动携带未投递评论）· `/` 标题搜索 · `tag:` 过滤 · `H/J/K/L` 移卡到相邻列 · `q`（唯一退出键，Esc 只关详情/清过滤）
- 鼠标交互不作 v1 承诺（实现期验证 herdr pane 鼠标事件能力）

## 8. 主动提醒

| 能力 | 裁决 |
| --- | --- |
| `notification.show` | **要**：attempt 转 blocked 弹通知（频率策略实现期定，避免轰炸） |
| `agent.view.set` 视图投影 | **v1 不做**（信息已有看板+chip 两处展示） |
| `workspace.report_metadata` chip | **要**：任务 worktree 侧栏挂「任务号+状态」，sync 在状态变化时更新 |

## 9. 发布与分发

- GitHub 公开仓库 + topic `herdr-plugin` + 根目录 `herdr-plugin.toml`（marketplace 自动索引）
- `[[build]]` = `cargo build --release`；**v1 要求用户有 Rust 工具链**，不发预编译
- platforms = linux / macos / windows；**License = MIT**
- UI 语言：**v1 英文**，配中文 README；不建 i18n 框架

## 10. 实现期待办（不阻塞规格）

文件锁的具体机制（lock 文件 vs fs2）· 通知频率策略 · 鼠标能力验证 · 32 并发上限的实际观测 · chip 的 TTL/重挂策略 · `order` 字段在多 writer 下的合并 · board.md 变更的冲突提示

## 附录 A：决议溯源

| 票 | 决议 |
| --- | --- |
| [01 VK 能力清单](.scratch/herdr-worker/issues/01-vk-capability-inventory.md) | 对照基线（[research/01](.scratch/herdr-worker/research/01-vk-capability-inventory.md)） |
| [02 herdr API 盘点](.scratch/herdr-worker/issues/02-herdr-api-inventory.md) | 能力面 + 13 条 gap（[research/02](.scratch/herdr-worker/research/02-herdr-api-inventory.md)） |
| [03 TUI diff 方案](.scratch/herdr-worker/issues/03-tui-diff-options.md) | 兄弟 pane 外部渲染器（[research/03](.scratch/herdr-worker/research/03-tui-diff-options.md)） |
| [04 数据 schema](.scratch/herdr-worker/issues/04-task-data-schema.md) | §4 |
| [05 编排模型](.scratch/herdr-worker/issues/05-orchestration-model.md) | §5 |
| [06 Agent API](.scratch/herdr-worker/issues/06-agent-api-design.md) | §6 |
| [07 TUI 原型](.scratch/herdr-worker/issues/07-board-tui-prototype.md) | §7 |
| [08 本票] | 装配 + §3 架构分界 + §2 顺序 attempt + §7 `f` 键 |
| [09 用量路线](.scratch/herdr-worker/issues/09-usage-tracking-route.md) | §2 用量行 |
| [10 通知与投影](.scratch/herdr-worker/issues/10-notifications-and-projections.md) | §8 |

## 附录 B：术语表

见 [CONTEXT.md](CONTEXT.md)：Board / Task / Attempt / Comment / Column。
