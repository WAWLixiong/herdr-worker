# 03: 六件套之文件四件套——agent API 的纯文件操作

**What to build:** `herdr-worker act <action>` 子命令实现 `create-task / move-task / add-comment / report-result` 四个纯文件操作（全部锁内），argv 传参（title/status/body 等用 flag + stdin）。

> 修正（2026-09-26，实现时发现）：`herdr plugin action invoke` 不支持传参（只注入 env 上下文），故四件套的调用方式为 **agent 直接调插件二进制**（路径经 `herdr plugin list --json` 的 plugin_root 可发现，即 `<root>/target/release/herdr-worker`）。manifest 注册的无参动作是 `open-board`（打开看板 pane，可作键位绑定目标）。SPEC §6 随之补充说明。

**Blocked by:** 02

**Status:** done

- [x] `create-task`：锁内扫描分配下一 id、建任务目录、写 task.md（title 来自参数，正文可来自 stdin）
- [x] `move-task`：改 status，校验必须取 board.md 声明列内（含隐藏列），非法列名报错
- [x] `add-comment`：追加评论文件（author 优先取 HERDR_PLUGIN_CONTEXT_JSON 的 focused_pane_agent，否则 human）
- [x] `report-result`：写指定 attempt 终态 + 摘要 + 可选 usage；自报 done 时任务自动入 review 列
- [x] manifest 注册 `open-board` action；`herdr plugin action invoke herdr-worker.open-board` 成功开板
- [x] 并发调用测试（锁正确性）；自报流转测试（done→review）
