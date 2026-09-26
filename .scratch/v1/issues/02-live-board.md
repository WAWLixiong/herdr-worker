# 02: 板是活的——文件监听与第一次写路径

**What to build:** 板成为"读者"：`.herdr-worker/` 变化（外部编辑器/agent 手改）时 TUI 自动重绘（轮询实现，~500ms，无 notify 依赖）；`H/J/K/L` 把选中卡移到相邻列——第一次写回 task.md（status + order），含文件锁（`create_new` 锁文件 + 超时 + 陈旧破坏，无 fs2 依赖）。

**Blocked by:** 01

**Status:** done

- [x] 轮询 watcher：task.md/board.md 的 mtime+集合快照对比，变化即重载重绘
- [x] 外部改 task.md 状态 → 卡片自动跳列（演示：编辑器改文件，板即时反映）
- [x] `H/J/K/L` 移卡写回：status 变更 + order 重排，文件 round-trip 稳定（含 id 零填充保持）
- [x] 文件锁：并发写者（两个进程同时移卡）不冲突、不丢更新；锁文件陈旧可破坏
- [x] 写路径单测：round-trip（读→改→写→读语义等价）
