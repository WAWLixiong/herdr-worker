# 任务数据 schema：仓库内 markdown 文件格式

Type: grilling
Status: resolved
Blocked by: 01

## Question

已定：任务数据是仓库内 markdown 文件（agent 与人可直接读写）。设计具体格式：目录布局（如 `.herdr-worker/`）、frontmatter 字段（标题 / 描述 / 状态列 / 标签 / assignee）、任务 id 方案、attempts（多次尝试）如何表达、评论如何表达、板与 herdr workspace 的绑定关系、文件是否随 git 提交。以「Vibe Kanban 完整能力清单」的任务模型为参照系，逐字段决定采纳 / 改造 / 放弃。

## Answer

（2026-09-26 与用户逐项敲定；参照系见 [research/01](../research/01-vk-capability-inventory.md)）

**术语**：**任务 Task**（规划单元，取代 VK 的 Issue）、**尝试 Attempt**（执行单元）、**板 Board**、**评论 Comment**。

### 目录布局：一任务一目录

```
<repo>/
└── .herdr-worker/
    ├── board.md                  # 板级配置（frontmatter）：列定义、隐藏列、分支前缀等
    └── tasks/
        └── 0042-fix-login/       # id = 4 位递增序号 + slug
            ├── task.md
            ├── attempts/
            │   └── 1.md
            └── comments/
                └── 20260926-1430.md
```

选任务目录而非单文件：agent 追加 attempt / 评论 = 新建文件（原子写，避免读-改-写冲突）。

### task.md

frontmatter 字段：`id`、`title`、`status`（必须取 board.md 声明的列名）、`priority`（urgent/high/medium/low，可空）、`tags[]`、`parent`（可选，子任务仅一层）、`blocked-by[]`（可选）、`order`（可选，列内手动排序）、`created`、`updated`。
**正文 = 描述 = agent prompt**（VK 核心设定，采纳）。

对照 VK 的裁决：**放弃 assignee**（VK 是多人指派，本地单用户无意义，执行者记在 attempt）；**放弃 related / has_duplicate**（VK 自己都只在 API 暴露，YAGNI）；**保留 parent + blocked-by**（看板直接有用）。

### 状态列：默认六列对齐 VK

`backlog / todo / in-progress / review / done / cancelled`；backlog、cancelled 默认隐藏。board.md frontmatter 可改名、增列、隐藏列。TUI 侧是否提供列编辑入口由「看板 TUI 形态原型」决定。

### attempts/N.md

frontmatter：`n`、`agent`（如 claude-code）、`branch`、`started`、`ended`、`status`（running/blocked/done/failed/cancelled，与 herdr agent 状态对齐）；正文 = 结果摘要（agent 收尾时写）。
语义边界：**attempt = 一个 worktree + 一段 agent 会话**；同一 attempt 内的 follow-up prompt 不新开文件；何时开新 attempt 由「执行编排模型」（05）定。

### comments/

一评论一文件：frontmatter 记 `author`（`human` 或 agent id）、`time`、可选 `file`/`hunk`（区分普通讨论与 diff 审查评论，对应 VK 的两套评论机制）；正文 = 内容。

### 板的绑定与 git

板 = 仓库内 `.herdr-worker/` 目录本身，**按 repo 路径绑定**，无 workspace↔板注册配置——同一仓库的多个 herdr workspace 看到同一块板。运行时状态（过滤器等）存 `HERDR_PLUGIN_STATE_DIR`，不进仓库。
任务文件**默认随 git 提交**：任务历史随代码走、clone 即得整块板、worktree 检出后 agent 可直接读到自己的任务卡（自发现）。个人草稿板场景在 README 注明可 gitignore。
