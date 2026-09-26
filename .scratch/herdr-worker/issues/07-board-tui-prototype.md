# 看板 TUI 形态原型

Type: prototype
Status: resolved
Blocked by: 03, 04, 05

## Question

做一个廉价原型回答“看板在 ratatui pane 里长什么样、怎么操作”：列布局、卡片信息密度、键盘 / 鼠标交互、任务详情视图、diff 视图的位置（参照「成熟的 TUI diff 方案调研」的结论）、搜索 / 过滤的交互形态、板级配置（board.md 的列定义等）在 TUI 里是否提供编辑入口还是 v1 手改文件。产出可运行的粗原型或静态线框，链接为资产。

## Answer

（2026-09-26 与用户对着原型裁决；原型资产：[prototype/07-board/](../prototype/07-board/)，`cargo run` 交互 / `--snap` 出静态帧，四帧已存 `snap-*.txt`）

### 布局裁决：混搭

- **主看板 = A 经典看板**：可见列横向排布（默认 todo/in-progress/review/done），列头带计数与选中高亮；卡片三行——①id+标题 ②agent+attempt 状态徽章（▶ running / ⛔ blocked / ⏳ 空闲待确认 / ✓ done / ✗ failed，对应 05 的两层状态模型）+ 次数 ③标签+usage
- **详情 = Enter 呼出浮层，排版沿用 C 的主从布局**：meta 行（列/agent/attempt 数/优先级）→ 标签行 → 任务正文（即 prompt）→ attempts 时间线 → comments（作者着色，human 绿 / agent 蓝）→ 底部 `d` 提示
- **B 泳道列表**：后续可选视图，v1 不做

### 交互词汇（已采纳）

`←→/hjkl` 列间移动 · `↑↓/jk` 卡片 · `Enter` 详情 · `Esc` 返回 · `s` 启动 attempt · `r` 自报审阅（agent 自报 done 的任务在 review 列，人按 r 确认入 done）· `d` 兄弟 pane 开 diff（delta → difftastic → git diff 回退，落实 research/03）· `/` 标题子串搜索 · `tag:` 前缀过滤 · **大写 `H/J/K/L` 把卡片移到相邻列**（拖拽的键盘等价物）· `q` 退出。鼠标交互不在 v1 键位承诺内（herdr pane 内鼠标事件能力待实现期验证）。

### 列配置

**v1 无 TUI 列编辑器**：手改 `.herdr-worker/board.md`，TUI 监听文件变化自动重载（重载本身必须实现）。
