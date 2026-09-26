# 07: 发布——README / LICENSE / git / marketplace 材料

**What to build:** 仓库发布态：README.md（英文，含 install/quickstart）+ README.zh-CN.md；LICENSE（MIT）；.gitignore（target/ 等）；`git init` + 初始提交（提交前须经用户确认）；GitHub topic `herdr-plugin` 上架说明；验证 `herdr plugin install <owner/repo>` 构建路径（`cargo build --release` + manifest 相对路径命令在全新 checkout 可用）。清理仅本地需要的构建补丁（unicode-segmentation vendor patch 视网络情况保留或移除，记录决定）。

**Blocked by:** 06

**Status:** ready-for-agent

- [ ] 双语 README（英文为主 + 中文全译），含"个人草稿板 gitignore"说明
- [ ] LICENSE MIT；.gitignore 完整
- [ ] git 仓库建立 + 初始提交（经用户确认后执行）
- [ ] marketplace 上架清单（topic、manifest 元数据齐全）
- [ ] 全新 checkout 模拟：build 命令 + pane 命令相对路径可用
