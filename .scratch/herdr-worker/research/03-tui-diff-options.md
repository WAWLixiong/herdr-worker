# TUI diff 方案调研：herdr-worker 插件如何在终端里展示任务 diff

调研票：`issues/03-tui-diff-options.md` ｜ 状态：resolved ｜ 日期：2026-09-26

## 0. 背景与约束（决定评估口径）

herdr-worker 是 herdr 插件（Rust + ratatui），形态由 herdr 插件模型决定（以下均出自本机 herdr 文档，`E:\projects\herdr\docs\next\website\src\content\docs\`）：

- 插件就是 argv 命令（本插件 = 一个 ratatui 二进制），**整个 herdr CLI 就是插件 API**，经 `HERDR_BIN_PATH` / `HERDR_SOCKET_PATH` 回调（plugins.mdx:23-29, 253-261）。
- 插件在 manifest 里声明 `[[panes]]` entrypoint，placement 可为 `overlay` / `popup` / `split` / `tab` / `zoomed`；打开后就是普通 herdr pane（plugins.mdx:285-322）。
- herdr 原生 pane API：`pane split`（`--direction right --no-focus`，返回新 pane ID）、`pane run`（"Run a shell command and submit it"，即向目标 pane 提交一条 shell 命令）、`pane read`（可保留 ANSI）、`pane wait-output`、`pane focus/resize/zoom/move/swap`（agent-automation.mdx:26-34, 63-66, 101-120）。

因此"嵌入"有三层含义，本报告对每条路线分别评估：

1. **进程级**：把外部 diff 工具 spawn 到自己的 herdr pane（路线 5）。
2. **视图级**：把外部渲染器的 ANSI 输出捕获后画进自己的 ratatui 界面（lazygit 对 delta 的做法）。
3. **库级**：把 diff 计算/渲染当作 Rust 库链接进 ratatui 应用（路线 4）。

---

## 1. delta（dandavison/delta）— 语法高亮 pager / stdin filter

**是什么**：stdin→stdout 的 diff 美化过滤器。官方文档明确定位："git 会把输出送给 delta，delta 再把自己的输出交给一个『真正的 pager』（如 less）"——delta 自身**不是交互程序**（how-delta-works: https://dandavison.github.io/delta/how-delta-works.html）。可显式管道：`git diff | delta | ...`。

- **交互能力**：滚动由宿主 pager（less）提供；`--navigate` 启用 `n`/`N` 在文件（及 `log -p` 的 commit）间跳转（https://dandavison.github.io/delta/navigation-keybindings-for-large-diffs.html）；`--hyperlinks` 产出可点击行号链接（lazygit 支持点击）。**无 hunk 级操作**（不能 stage/discard），**无程序化行内定位 API**——输出就是给人看的 ANSI 文本（"delta's output is intended for humans, not machines"，how-delta-works）。
- **渲染**：行内（Levenshtein）高亮、**side-by-side（带换行）**、行号、语法高亮 + 主题、merge 冲突 / blame / grep 美化、`--color-moved`、diff-so-fancy 模拟（features: https://dandavison.github.io/delta/features.html）。
- **嵌入成本**：crates.io 上 `git-delta` 为**纯 bin crate**（`has_lib: false`，bin 名 `delta`），无库 API，不能链接嵌入。两种可行法：(a) 子进程 + PTY pane（路线 5 的标准形态）；(b) 视图级捕获：`delta --color=always` 输出 ANSI，ratatui 里再渲染——lazygit 正是这么干的（见 §3），代价是 pager 级交互（如 `--navigate`）会丢失。
- **维护/活跃**：Rust，32,346 stars，MIT；最近 push 2026-09-19，crates.io 0.19.2（2026-03-28）——非常活跃。（源：https://api.github.com/repos/dandavison/delta ；https://crates.io/crates/git-delta）
- **许可**：MIT。

## 2. difftastic（Wilfred/difftastic）— 结构化 diff

**是什么**：tree-sitter 语法树 diff，默认 side-by-side，展示真实行号；30+ 语言 + 结构化文本格式，解析失败/超大输入时回退行级 diff + 词高亮（manual: https://difftastic.wilfred.me.uk/print.html）。

- **交互能力**：**没有内置可滚动 TUI**——`src/display/` 只有 context/hunks/inline/json/side_by_side/style，`main.rs` 无任何 scroll/TTY 交互代码（源：repo `src/main.rs`、`src/display/` 目录，2026-09 检查）；翻页靠外置 pager（manual 的 git 配置建议 `[pager] difftool = true`）。无 hunk 级操作：README 明确列为 non-goal——"输出面向人类阅读，**不生成可后续应用的 patch**"；也**不提供工具集成 API**："Does difftastic integrate with my favourite tool? Probably not… consider writing a plugin"（README: https://github.com/Wilfred/difftastic）。
- **渲染**：默认 side-by-side（`--display=side-by-side|side-by-side-show-both|inline|json`）；管道时需 `--color=always`（jj 官方配置即如此）。
- **嵌入成本**：crates.io `difftastic` 为纯 bin（`has_lib: false`，bin 名 `difft`），MIT；只能进程级或 ANSI 捕获嵌入。重：tree-sitter C 依赖（需 C++14 编译），大变更时内存/性能差（README Known Issues 自认）。git 侧接入方式：`diff.external` / `GIT_EXTERNAL_DIFF` / `--ext-diff` / difftool（manual Git 章）。
- **维护/活跃**：Rust，25,934 stars，MIT（vendored parser 为 MIT/Apache 混合）；push 2026-09-22，crates.io 0.71.0（2026-09-18）——活跃。（源：https://api.github.com/repos/Wilfred/difftastic ；https://crates.io/crates/difftastic）
- **许可**：MIT（vendored parsers 混合 MIT/Apache）。

## 3. lazygit 的 diff 视图 — 集成式 TUI diff 的标杆（但不可提取）

**是什么**：Go + gocui 单体 TUI，diff 视图自己渲染、自己交互：空格 stage 单行、`v` 选行范围、`a` 选整个 hunk（README: https://github.com/jesseduffield/lazygit，"Press space on the selected line to stage it, or press `v`… press `a` to select the entirety of the current hunk"）。

- **可提取性：否**。Go 单体、无库边界，gocui 视图与应用状态深度耦合，对 Rust + ratatui 插件没有可复用代码。价值在于两个**模式**：
  1. **交互范式**：行/hunk 级 stage/discard 直接长在 diff 视图上（对 herdr-worker 未来"审阅任务产出"有直接参考价值）。
  2. **三通道渲染器架构**（docs/Custom_DiffRenderers.md: https://github.com/jesseduffield/lazygit/blob/master/docs/Custom_DiffRenderers.md）：
     - `stdinFilter`：GIT_PAGER 机制接 delta / diff-so-fancy（TUI 内嵌 filter 渲染）；
     - `extDiff`：`--ext-diff` 完全接管 diff 生成（difftastic）；
     - `rawGit`：原始 git + 自定义参数（如 `--color-words`）。
     用户用 `|` 键循环切换。关键实证：**"delta 的 `--navigate` 在 lazygit 里不工作（技术原因）"**——filter 式嵌入会丢 pager 级交互；delta 的 hyperlinks 则可用且可点击。side-by-side 需借 ydiff 等 filter（`ydiff -p cat` + `sidePanelWidth`）。
- **维护/活跃**：82,686 stars，MIT，push 2026-09-26（当日），极活跃。（源：https://api.github.com/repos/jesseduffield/lazygit）

## 4. ratatui 生态：diff 组件 / crate

**结论：不存在成熟的 ratatui diff 组件；生态的真实形态是"diff 算法库 + 自建视图"，且有两个大型先例。**

- **awesome-ratatui 官方 Widgets 清单无任何 diff widget**（查遍 2026-09 版列表，只有 textarea、scrollview、image、markdown 等；https://github.com/ratatui/awesome-ratatui）。
- **crates.io 搜 "tui diff"：全是小众独立应用**（tuicr、gandr、mutiny-diff、hewdiff、branchdiff、oyo、kizu、vigia…，下载量几十到几千），没有可复用库；唯一的引擎类小 crate `karet-diff`（histogram 行 diff + 行内高亮 + unified-diff 解析 + hunk staging）出自单一项目、尚早期。（源：https://crates.io/api/v1/crates?q=tui+diff）
- **diff 算法库（真正的积木）**：
  - `similar`（mitsuhiko）：Apache-2.0，v3.2.0（2026-08），总下载 2.04 亿；patience/myers/lcs + `inline`（词级）feature。（https://crates.io/crates/similar）
  - `imara-diff`（pascalkuthe，Helix 维护者）：Apache-2.0，v0.2.0（2025-06），总下载 3264 万；histogram/myers/patience，`unified_diff` 输出 feature。（https://crates.io/crates/imara-diff）
- **先例一：gitui**（gitui-org/gitui）：Rust + **ratatui 0.29** + asyncgit(git2) + syntect/two-face 高亮；功能含 "Stage, unstage, revert and reset files, **hunks and lines**"、async git API；22,523 stars、MIT、push 2026-08-04。证明"ratatui 内建 diff 视图 + hunk 级操作"是成熟路线（其 diff 直接来自 libgit2 输出）。（源：https://api.github.com/repos/gitui-org/gitui ；deps: https://deps.rs/crate/gitui/latest ；README features）
- **先例二：Helix**：`helix-core` 直接依赖 `imara-diff 0.2.0` 做编辑器内联 diff。（源：helix repo `helix-core/Cargo.toml`）
- **混合变体：`tui-term`**（MIT，v0.3.4，2026-04，下载 131 万）：ratatui 的伪终端 widget（vt100 解析，`unstable` portable-pty feature）。理论上可在板内嵌一个跑着 delta/difft 的子终端——但输入路由、resize、焦点管理复杂度高，且**与 herdr pane 能力重复**，v1 不值。

## 5. "外部 pager 单独 pane" 路线（herdr 原生路线）

**做法**：板内"查看 diff"动作 → `$HERDR_BIN_PATH pane split --current --direction right --no-focus` 取新 pane ID → `pane run <pane> "git -C <worktree> diff | delta"`（`pane run` 是向 pane 提交 shell 命令，管道天然走用户 shell）→ 需要时 `pane focus`。渲染器发现链：`delta` → `difft` → 裸 `git diff`（less）兜底。

- **交互能力**：完整原生体验——less 的滚动/搜索、delta 的 navigate/hyperlinks、difft 的结构化视图；herdr 侧免费获得 pane 生命周期（focus/resize/zoom/关闭）。**hunk 级操作不在 pager 里**，但板可以把 `git add -p` 同样 spawn 到 pane，或（后续）自己在板内实现（路线 4）。
- **嵌入成本**：**≈ 两次 CLI 调用，零渲染代码**。herdr 文档已给出同款配方（agent-automation.mdx 的 `pane split` + `pane run` 编排示例）。无许可问题（外部工具作为可选运行时依赖，MIT 均可商用分发或提示安装）。
- **代价**：diff 离开板的上下文（不能直接挂任务动作）；依赖用户装了 delta/difft（git 必装，兜底无虞）；Windows 下走 herdr 正常 pane launcher（plugins.mdx:326-329 有明确说明）。

---

## 对比表

| 路线 | 滚动 | hunk 级操作 | 行内定位/API | side-by-side | ratatui 嵌入成本 | 维护/活跃 | 许可 |
|---|---|---|---|---|---|---|---|
| 1. delta（进程/pager） | 宿主 pager + `n/N` 导航 | ❌ | ❌（ANSI-only；hyperlinks 例外） | ✅ | 低（spawn）；中（ANSI 捕获重绘，lazygit 模式，丢 navigate） | ★★★★★ 32.3k★ | MIT |
| 2. difftastic（进程） | 靠外置 pager | ❌（明确 non-goal：不产 patch） | ❌（`--color=always` 管道 / `--display=json` 可机读但无交互） | ✅ 默认 | 低（spawn）；高（嵌入捕获：tree-sitter 重、内存重） | ★★★★☆ 25.9k★ | MIT |
| 3. lazygit diff 视图 | ✅ 自绘 | ✅ 行/范围/hunk | ✅ | ⚠️ 需 ydiff filter | ❌ 不可提取（Go 单体）；仅作 UX 与三通道渲染器架构参考 | ★★★★★ 82.7k★ | MIT |
| 4. 自建（similar / imara-diff + ratatui） | ✅ 自控 | ✅ 全权（gitui 已证明） | ✅ 全权 | ✅ 可做（工作量×2） | 中（数天级；syntect 高亮、hunk 解析、滚动态） | 库本身极活跃（similar 2 亿下载 / imara-diff=Helix 同源） | Apache-2.0 |
| 5. 外部渲染器独立 pane | ✅ 原生 | ❌（可另 spawn `git add -p` 补） | ❌ | ✅（随渲染器） | **≈0**（两次 herdr CLI 调用） | 随所选工具 | 工具均 MIT |

## 推荐

**v1 首选路线 5（外部渲染器独立 pane），路线 4（imara-diff/similar 自建板内视图）作为后续增强储备。**

1. **路线 5 是 herdr 的"母语"**：herdr 的核心能力就是 pane 编排，而 delta/difftastic 都是 bin-only、无库 API 的流式渲染器——进程级嵌入零成本、零许可风险、完整原生交互，还天然尊重用户已有的 delta/difft 配置。渲染器发现链 delta → difft → `git diff` 兜底。lazygit 的三通道架构证明这是成熟模式；其"filter 嵌入丢 `--navigate`"的教训也说明视图级捕获（成本更高的那条路）并不划算。
2. **当产品确实需要"hunk 挂任务动作"**（如在 diff 上标记审阅意见、丢弃单个 hunk）时再上路线 4：`imara-diff`（Helix 同源、Apache-2.0、`unified_diff` 现成）或 `similar`（词级 inline diff 现成）计算 + ratatui 渲染 + syntect 高亮，gitui 已把这条路走通（ratatui + hunk/行级 stage），属于数天级工作量，无需发明新东西。
3. **明确不做**：把 delta/difftastic 当库嵌入（无 API）；复用 lazygit 代码（不可提取）；tui-term 板内 PTY（与 herdr pane 重复造轮子）。

## 来源清单

- delta：https://dandavison.github.io/delta/ （introduction / how-delta-works / features / navigation-keybindings-for-large-diffs）；https://api.github.com/repos/dandavison/delta ；https://crates.io/api/v1/crates/git-delta
- difftastic：https://difftastic.wilfred.me.uk/print.html （manual 全文）；https://github.com/Wilfred/difftastic （README、difft.1.md、src/main.rs、src/display/）；https://api.github.com/repos/Wilfred/difftastic ；https://crates.io/api/v1/crates/difftastic
- lazygit：https://github.com/jesseduffield/lazygit （README）；https://github.com/jesseduffield/lazygit/blob/master/docs/Custom_DiffRenderers.md ；https://api.github.com/repos/jesseduffield/lazygit
- ratatui 生态：https://github.com/ratatui/awesome-ratatui （Widgets 清单）；https://crates.io/api/v1/crates?q=tui+diff ；https://crates.io/crates/similar ；https://crates.io/crates/imara-diff ；https://crates.io/crates/tui-term ；https://api.github.com/repos/gitui-org/gitui ；https://deps.rs/crate/gitui/latest ；https://deps.rs/crate/asyncgit/0.28.1 ；helix repo `helix-core/Cargo.toml`（imara-diff 0.2.0）
- herdr（本机主源）：`E:\projects\herdr\docs\next\website\src\content\docs\plugins.mdx`、`agent-automation.mdx`、`configuration.mdx`、`concepts.mdx`
