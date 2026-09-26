# 06: 人侧闭环——r / f / d / stop / 清理

**What to build:** 完成人在环的全部交互：`r` 自报审阅（review 列确认入 done）；`f` follow-up prompt（向该 attempt 的 agent 追加 prompt，自动携带未投递评论，投递后评论标记已投递）；`d` 在兄弟 pane 打开 diff（渲染器探测链 delta → difftastic → `git diff` 回退，两次 herdr CLI 调用）；stop-attempt（send-keys ctrl+c → release → 标 cancelled，worktree 保留）；任务完成后的"清理 worktree"动作（`worktree remove`，分支永不删）。

**Blocked by:** 05

**Status:** ready-for-agent

- [ ] `r`：review→done；`f`：agent prompt 携带未投递评论（file/hunk 定位转文本）
- [ ] `d`：兄弟 pane diff，渲染器探测链有回退；无 diff 时提示
- [ ] stop-attempt 全路径（ctrl+c → release → cancelled）
- [ ] 清理 worktree 动作（只清插件自建的，前缀/label 识别；分支保留）
- [ ] 一条龙演示：建卡 → s 启动 → agent 完成 → 自报进 review → r 确认 → d 看 diff → 清理
