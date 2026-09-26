# herdr-worker

编码代理的看板 —— 一个 [herdr](https://herdr.dev) 插件。

把工作规划成看板上的任务，一键派给编码代理，在终端里审阅结果。板就是**你仓库里的 markdown**：任务、尝试、评论都是普通文件，人和 agent 读写同一个事实源。

> 定位：[Vibe Kanban](https://vibekanban.com)（已 sunset）的能力继任者，以纯 TUI 跑在 herdr 运行时上。

## 你会得到什么

- herdr pane 里的看板（ratatui TUI）：列、卡片、详情浮层
- 一个按键派活：**git worktree + agent + prompt，全托管**
- 状态实时回流：▶ running · ⛔ blocked（弹 herdr 通知）· ⏳ 空闲待确认
- 兄弟 pane 里审阅 diff（delta → difftastic → `git diff` 回退）
- Agent 驱动 API：agent 能建任务、报结果，甚至自己开工
- 评论（可锚定文件/hunk）在追问时批量投递给 agent
- 可选的 attempt 级用量自报

## 环境要求

- herdr ≥ 0.7.0
- Rust 工具链（插件从源码构建）
- git

## 安装

从 marketplace（发布后）：

```bash
herdr plugin install <owner>/herdr-worker
```

开发时链接本地检出：

```bash
herdr plugin link /path/to/herdr-worker
herdr plugin action invoke herdr-worker.open-board   # 打开 Board pane
```

> 说明：`unicode-segmentation` 已 vendor 在 `vendor/` 下（其发布的 1.13.1 被 yank；此处锁定 herdr 本体同版本）。clone 时请保留该目录。

## 快速上手

1. 在仓库里建板 —— `.herdr-worker/board.md`：

   ```yaml
   ---
   columns: [backlog, todo, in-progress, review, done, cancelled]
   hidden: [backlog, cancelled]
   branch_prefix: hw
   default_agent: claude
   diff_tool: auto        # auto（探测 delta→difft→纯文本）| none | 命令名（diff 从 stdin 喂入）
   ---
   ```

2. 建任务 —— `.herdr-worker/tasks/0001-fix-login/task.md`：

   ```yaml
   ---
   id: 0001
   title: 修复登录页
   status: todo
   ---
   任务正文**就是** agent 收到的 prompt。
   ```

3. 在 herdr 里打开板，光标移到卡片上按 `s`。worktree 出现、agent 在里面启动、卡片自动进 `in-progress` —— 全自动。

## 板的布局

```
.herdr-worker/
├── board.md                  # 列定义、隐藏列、分支前缀、默认 agent
└── tasks/
    └── 0042-fix-login/
        ├── task.md           # frontmatter + 正文（= prompt）
        ├── attempts/
        │   └── 1.md          # agent、分支、pane 锚点、状态、用量、摘要
        └── comments/
            └── 20260926-1430.md   # 作者、可选 file/hunk 锚点、内容
```

默认全部随 git 提交（clone 即得整块板；agent 在自己 worktree 里就能看到任务卡）。想要个人草稿板？把 `.herdr-worker/` 加进 `.gitignore`。

## 键位

| 键 | 动作 |
| --- | --- |
| `←→` / `h l` | 列间移动 |
| `↑↓` / `j k` | 卡片间移动 |
| `Enter` / `Esc` | 打开详情浮层 / 关闭（Esc 同时清除过滤） |
| `q` | 退出板（唯一退出键——overlay pane 启动期可能收到类 Esc 事件，故 Esc 永不退出） |
| `n` | 新建任务：输入标题，`#标签` 词自动收进 tags（落 `todo` 列） |
| `e` | 在 `$EDITOR` 里编辑任务正文（兄弟 pane） |
| `s` | 启动 attempt（worktree + agent + prompt） |
| `r` | review 列确认入 done |
| `f` | 追问：把未投递评论发给 agent |
| `d` | 全屏 overlay 打开 worktree diff：已跟踪改动 + 未跟踪文件清单；任意键关闭（有内容时不过期） |
| `x` | 停止进行中的 attempt（cancelled；worktree 保留） |
| `c` | 清理该任务插件自建的 worktree（分支永不删） |
| `H J K L` | 卡片移到相邻列 / 列内排序 |
| `/` | 标题子串过滤，或 `tag:xxx`（Esc 清除） |
| `q` | 退出 |

## 状态模型（两层）

- **活状态**：herdr 事件驱动 attempt 状态 —— `working → running`、`blocked → blocked`（通知 + 侧栏 chip）。`idle` **不是**完成：卡片显示 `⏳ 空闲待确认`，直到 agent 自报或人裁决。pane/worktree 无自报消失 → 保守标 `failed`。
- **终态**：agent 经 `report-result` 自报（`done` 自动进 `review`，人按 `r` 确认入 done），或被停止/取消。

## Agent API —— markdown 即 API

**读**：任何进程（包括 agent）直接读 markdown；任务 worktree 里就有自己的卡。

**写**：调用插件二进制。agent 通常会在 prompt 页脚自动收到全部坐标。发现方式：`herdr plugin list --json` → `plugin_root` → `<plugin_root>/target/release/herdr-worker`。

```bash
herdr-worker act create-task --title "加分页" [--status todo] [--tag api] [--body-file -]
herdr-worker act move-task --id N --to <列名>
herdr-worker act add-comment --id N [--file src/x.rs] [--body "内容"]
herdr-worker act report-result --id N --attempt M --status done|failed [--usage "45k tok"] --body "摘要"
herdr-worker act start-attempt --id N [--agent claude]
herdr-worker act stop-attempt --id N
herdr-worker act follow-up --id N
herdr-worker act clean-worktree --id N
```

所有写入都有板文件锁保护；手改文件是一等公民（板自动热重载）。

## 发布到 marketplace

herdr 的 marketplace 自动索引打了 `herdr-plugin` topic、根目录有可解析 `herdr-plugin.toml` 的公开 GitHub 仓库。给仓库打上 topic、push、分享 `herdr plugin install <owner>/herdr-worker` 即可。

## 非目标（v1）

不做 PR 创建/合并 · 不做多仓库 · 不做多板（用标签 + 过滤）· 不做协作/云同步 · 不做浏览器预览 · 不做 TUI 列编辑器（手改 `board.md`，板热重载）· 仅英文界面（无 i18n 框架）。

## 许可证

[MIT](LICENSE)
