---
id: 0002
title: 板是活的——文件监听与第一次写路径
status: review
tags: [v1, tui]
blocked-by: [1]
created: 2026-09-26T10:00:00+08:00
---
轮询监听 .herdr-worker/ 变化自动重绘；H/J/K/L 移卡写回 task.md；文件锁（create_new 锁文件 + 超时 + 陈旧破坏）。
