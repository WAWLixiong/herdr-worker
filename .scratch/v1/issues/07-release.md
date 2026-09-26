# 07: 发布——README / LICENSE / git / marketplace 材料

**What to build:** 仓库发布态：README.md（英文，含 install/quickstart）+ README.zh-CN.md；LICENSE（MIT）；.gitignore（target/ 等）；`git init` + 初始提交（提交前须经用户确认）；GitHub topic `herdr-plugin` 上架说明；验证 `herdr plugin install <owner/repo>` 构建路径（`cargo build --release` + manifest 相对路径命令在全新 checkout 可用）。清理仅本地需要的构建补丁（unicode-segmentation vendor patch 视网络情况保留或移除，记录决定）。

**Blocked by:** 06

**Status:** done

- [x] 双语 README（英文为主 + 中文全译），含"个人草稿板 gitignore"说明
- [x] LICENSE MIT；.gitignore 完整
- [x] git 仓库建立 + 初始提交（fe13e7c，经用户放行后执行；工单 04 时因 worktree 编排需要而提前）
- [x] marketplace 上架清单（README 发布章节：topic `herdr-plugin` + 根目录 manifest；实际 push 与打 topic 由用户执行）
- [x] 全新 checkout 模拟：build 命令 + pane 命令相对路径可用（temp clone 14.9s 构建成功，exe/vendor/manifest 齐）

> vendor patch 决定：**保留**。unicode-segmentation 1.13.1 在 crates.io 被 yank，vendor 锁定 herdr 本体同版本——确定性构建且规避 yank 问题，README 已注明 clone 时保留 vendor/。
