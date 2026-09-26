# 06: 人侧闭环——r / f / d / stop / 清理

**What to build:** 完成人在环的全部交互：`r` 自报审阅（review 列确认入 done）；`f` follow-up prompt（向该 attempt 的 agent 追加 prompt，自动携带未投递评论，投递后评论标记已投递）；`d` 在兄弟 pane 打开 diff（渲染器探测链 delta → difftastic → `git diff` 回退，两次 herdr CLI 调用）；stop-attempt（send-keys ctrl+c → release → 标 cancelled，worktree 保留）；任务完成后的"清理 worktree"动作（`worktree remove`，分支永不删）。

**Blocked by:** 05

**Status:** done

- [x] `r`：review→done；`f`：agent prompt 携带未投递评论（file/hunk 定位转文本，投递后标记 delivered，不重复携带）
- [x] `d`：兄弟 pane diff，渲染器探测链有回退（delta → difftastic → git diff；真机验证回退到 git diff）
- [x] stop-attempt 全路径（ctrl+c → release → cancelled，worktree 保留；act stop-attempt 同路径）
- [x] 清理 worktree 动作（只清插件自建的，分支前缀守卫；分支保留）
- [x] `/` 标题搜索 + `tag:` 过滤（输入模式、Esc 清除、过滤中列计数实时反映）
- [x] 一条龙演示：建卡→s→自报进 review（0008/0009 已真机走过）；r/f/x/c 的交互段交由用户在板上验收（代码路径全部单测 + herdr 形状真机验证）

> 键位表更新：s 启动 · r 审过 · f 反馈 · d diff · x 停止 · c 清理 · / 过滤（H/J/K/L 移卡/排序保留）。评论详情视图显示"✓已投递"标记。
