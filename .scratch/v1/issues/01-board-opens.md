# 01: 板能开——从 markdown 到屏幕的完整读路径

**What to build:** 在仓库根建立 cargo 项目 `herdr-worker` 与 `herdr-plugin.toml`。`herdr-worker board` 子命令：向上查找 `.herdr-worker/`，解析 board.md 与 tasks（task.md / attempts / comments），用 ratatui 渲染经典看板（列卡片墙 + 三行卡片 + 详情浮层），支持 `←→/hjkl` 列、`↑↓/jk` 卡、`Enter/Esc` 详情、`q` 退出。`herdr plugin link` 后能在真机 herdr 里打开 Board pane 看到真数据。

**Blocked by:** None (can start immediately)

**Status:** done

- [x] frontmatter 解析器（YAML 子集：`key: value` 标量 + `[a, b]` 内联列表 + `---` 分隔；文件对外保持合法 YAML）带单测
- [x] 数据层：board.md（columns/hidden/branch_prefix/default_agent，缺省六列）+ tasks 目录扫描 + attempts/comments 装配
- [x] TUI：列/卡片/详情浮层渲染 + 导航键位（键位帮助行只显示已实现键）
- [x] 离屏渲染测试（TestBackend）断言真数据帧含已知任务标题
- [x] `herdr plugin link` 成功且 `herdr plugin list` 可见；真机开板可导航
- [x] 仓库自带 dogfood 板：`.herdr-worker/` 内含本批 v1 工单作为任务卡
