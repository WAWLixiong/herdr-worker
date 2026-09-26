---
id: 0005
title: sync 事件大脑——状态回流与 blocked 通知
status: review
tags: [v1, orchestration]
blocked-by: [4]
order: 0
updated: 2026-09-26T12:26:59Z
---
[[events]] → sync 一次性进程：事件 JSON → 锁内更新 attempt → blocked 弹通知 → 更新 chip；idle≠完成；pane 消失无自报 → failed。
