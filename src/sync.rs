// sync：事件大脑（SPEC §5 状态回流 + §8 通知与 chip）
// manifest [[events]] 触发的一次性短命进程：读 HERDR_PLUGIN_EVENT_JSON → 锁内更新
// attempt → 锁外执行 herdr 副作用（notification / chip）。快进快出。

use anyhow::{Context, Result};
use std::path::Path;

use crate::lock::with_lock;
use crate::model::{self, Attempt, Board, Task};
use crate::write;

#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// 事件与板无关（不是我们 pane/workspace 的）
    Ignored,
    AttemptStatus {
        task_id: u32,
        attempt_n: u32,
        from: String,
        to: String,
    },
    NotifyBlocked {
        task_id: u32,
        attempt_n: u32,
    },
    /// 活子状态翻转（idle ↔ working），attempt 终态未变
    AgentState {
        task_id: u32,
        attempt_n: u32,
        idle: bool,
    },
    Chip {
        workspace_id: String,
        task_id: u32,
        text: String,
    },
}

/// 活状态映射（SPEC §5）：working→running；blocked→blocked；
/// idle/done/unknown → running（idle ≠ 完成，attempt 保持 running，TUI 显示"空闲待确认"）
fn map_live_status(agent_status: &str) -> &'static str {
    match agent_status {
        "working" => "running",
        "blocked" => "blocked",
        _ => "running",
    }
}

fn badge(status: &str) -> &'static str {
    match status {
        "running" => "▶",
        "blocked" => "⛔",
        "done" => "✓",
        "failed" => "✗",
        "cancelled" => "–",
        _ => "·",
    }
}

fn find_active_by<'a>(
    board: &'a Board,
    pane_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Option<(&'a Task, &'a Attempt)> {
    board.tasks.iter().find_map(|t| {
        t.attempts.iter().find_map(|a| {
            let active = matches!(a.status.as_str(), "running" | "blocked");
            let pane_hit = pane_id.is_some() && a.pane.as_deref() == pane_id;
            let ws_hit = workspace_id.is_some() && a.workspace.as_deref() == workspace_id;
            if active && (pane_hit || ws_hit) {
                Some((t, a))
            } else {
                None
            }
        })
    })
}

/// 锁内：纯文件状态转移，返回副作用意图（测试直接调这个）
pub fn process_event(root: &Path, envelope: &serde_json::Value) -> Result<Vec<Outcome>> {
    let raw_event = envelope
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    // herdr 的事件名有序列化差异：钩子信封为 snake_case（pane_agent_status_changed），
    // 文档为点分（pane.agent_status_changed）。事件名内部本身含下划线，不能机械替换——两种字面形式都收。
    let event = raw_event;
    let data = envelope.get("data").cloned().unwrap_or(serde_json::Value::Null);
    let board = model::load_board(root)?;

    match event {
        "pane.agent_status_changed" | "pane_agent_status_changed" => {
            let pane_id = data.get("pane_id").and_then(|v| v.as_str());
            let agent_status = data
                .get("agent_status")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let Some((task, att)) = find_active_by(&board, pane_id, None) else {
                return Ok(vec![Outcome::Ignored]);
            };
            let to = map_live_status(agent_status).to_string();
            // 活子状态：idle/done → "⏳ 空闲待确认"；working/blocked 清除
            let state_action: Option<&str> = match agent_status {
                "idle" | "done" => Some("idle"),
                "working" | "blocked" => Some(""),
                _ => None,
            };
            let state_changed = match state_action {
                Some("idle") => att.agent_state.as_deref() != Some("idle"),
                Some("") => att.agent_state.is_some(),
                _ => false,
            };
            if att.status == to && !state_changed {
                return Ok(vec![Outcome::Ignored]);
            }
            let att_path = board
                .root
                .join("tasks")
                .join(&task.dir_name)
                .join("attempts")
                .join(format!("{}.md", att.n));
            if att.status != to {
                write::set_field(&att_path, "status", &to)?;
            }
            if let Some(sa) = state_action {
                if sa.is_empty() {
                    write::remove_field(&att_path, "agent_state")?;
                } else {
                    write::set_field(&att_path, "agent_state", sa)?;
                }
            }
            let from = att.status.clone();
            let mut out = Vec::new();
            if att.status != to {
                out.push(Outcome::AttemptStatus {
                    task_id: task.id,
                    attempt_n: att.n,
                    from: from.clone(),
                    to: to.clone(),
                });
                if to == "blocked" && from != "blocked" {
                    out.push(Outcome::NotifyBlocked {
                        task_id: task.id,
                        attempt_n: att.n,
                    });
                }
            }
            if state_changed {
                out.push(Outcome::AgentState {
                    task_id: task.id,
                    attempt_n: att.n,
                    idle: state_action == Some("idle"),
                });
            }
            if !out.is_empty() {
                if let Some(ws) = &att.workspace {
                    let mark = if state_action == Some("idle") {
                        "⏳"
                    } else {
                        badge(&to)
                    };
                    out.push(Outcome::Chip {
                        workspace_id: ws.clone(),
                        task_id: task.id,
                        text: format!("{:04} {mark}", task.id),
                    });
                }
            }
            Ok(out)
        }
        // pane 没了 / worktree 被移除：无终态自报 → 保守 failed
        "pane.exited" | "pane_exited" => {
            let pane_id = data.get("pane_id").and_then(|v| v.as_str());
            fail_active(&board, pane_id, None)
        }
        "worktree.removed" | "worktree_removed" => {
            let ws = data.get("workspace_id").and_then(|v| v.as_str());
            fail_active(&board, None, ws)
        }
        _ => Ok(vec![Outcome::Ignored]),
    }
}

fn fail_active(
    board: &Board,
    pane_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<Vec<Outcome>> {
    let Some((task, att)) = find_active_by(board, pane_id, workspace_id) else {
        return Ok(vec![Outcome::Ignored]);
    };
    let att_path = board
        .root
        .join("tasks")
        .join(&task.dir_name)
        .join("attempts")
        .join(format!("{}.md", att.n));
    write::set_field(&att_path, "status", "failed")?;
    write::set_field(&att_path, "ended", &write::utc_now_rfc3339())?;
    Ok(vec![Outcome::AttemptStatus {
        task_id: task.id,
        attempt_n: att.n,
        from: att.status.clone(),
        to: "failed".to_string(),
    }])
}

/// 入口：manifest [[events]] 调用
pub fn run() -> Result<()> {
    let env = std::env::var("HERDR_PLUGIN_EVENT_JSON")
        .context("sync 需要 HERDR_PLUGIN_EVENT_JSON（应由 herdr 事件钩子注入）")?;
    let envelope: serde_json::Value = serde_json::from_str(&env)?;
    let start = std::env::current_dir()?;
    let root = model::find_board(&start)
        .ok_or_else(|| anyhow::anyhow!("未找到 .herdr-worker/（从 {} 逐级向上）", start.display()))?;

    let outcomes = with_lock(&root, || process_event(&root, &envelope))?;

    // 一行摘要进插件日志（herdr 捕获 stderr）；同时兼容两种事件名形式
    let event_name = envelope
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let pane = envelope
        .get("data")
        .and_then(|d| d.get("pane_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let summary: Vec<String> = outcomes
        .iter()
        .map(|o| match o {
            Outcome::Ignored => "ignored".to_string(),
            Outcome::AttemptStatus { task_id, attempt_n, from, to } => {
                format!("task {task_id:04}#{attempt_n} {from}->{to}")
            }
            Outcome::NotifyBlocked { .. } => "notify-blocked".to_string(),
            Outcome::AgentState { idle: true, .. } => "agent-idle".to_string(),
            Outcome::AgentState { idle: false, .. } => "agent-working".to_string(),
            Outcome::Chip { .. } => "chip".to_string(),
        })
        .collect();
    eprintln!("sync [{event_name}] pane={pane}: {}", summary.join(", "));

    // 锁外副作用：通知 + chip（失败不影响状态回流）
    for o in &outcomes {
        match o {
            Outcome::NotifyBlocked { task_id, attempt_n } => {
                let _ = herdr(&[
                    "notification", "show",
                    &format!("hw {task_id:04} ⛔ blocked"),
                    "--body", &format!("任务 {task_id:04} 的 agent 等待你回答（attempt #{attempt_n}）"),
                    "--sound", "request",
                ]);
            }
            Outcome::Chip { workspace_id, task_id, text } => {
                let _ = herdr(&[
                    "workspace", "report-metadata", workspace_id,
                    "--source", "herdr-worker",
                    "--token", &format!("hw={text}"),
                ]);
                let _ = task_id;
            }
            _ => {}
        }
    }
    Ok(())
}

fn herdr(args: &[&str]) -> Result<()> {
    let bin = std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
    let out = std::process::Command::new(&bin)
        .args(args)
        .output()
        .with_context(|| format!("调用 {bin} 失败"))?;
    if !out.status.success() {
        anyhow::bail!(
            "herdr {} 失败：{}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture(tag: &str) -> PathBuf2 {
        let base = std::env::temp_dir().join(format!("herdr-worker-sync-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join(".herdr-worker");
        let t = root.join("tasks").join("0005-busy");
        fs::create_dir_all(t.join("attempts")).unwrap();
        fs::write(
            t.join("task.md"),
            "---\nid: 0005\ntitle: 忙\nstatus: in-progress\n---\n正文\n",
        )
        .unwrap();
        fs::write(
            t.join("attempts").join("1.md"),
            "---\nn: 1\nagent: opencode\nbranch: hw/0005-busy\npane: w2:p1\nworkspace: w2\nagent_name: hw0005-1\nstatus: running\n---\n",
        )
        .unwrap();
        root
    }
    type PathBuf2 = std::path::PathBuf;

    fn envelope(event: &str, json: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "event": event, "data": json })
    }

    fn attempt_status(root: &Path) -> String {
        let src = fs::read_to_string(
            root.join("tasks").join("0005-busy").join("attempts").join("1.md"),
        )
        .unwrap();
        src.lines()
            .find(|l| l.starts_with("status:"))
            .unwrap()
            .trim_start_matches("status: ")
            .to_string()
    }

    #[test]
    fn snake_case_envelope_also_matches() {
        let root = fixture("snake");
        let ev = serde_json::json!({
            "event": "pane_agent_status_changed",
            "data": {"pane_id": "w2:p1", "agent_status": "blocked", "workspace_id": "w2"}
        });
        let out = process_event(&root, &ev).unwrap();
        assert_eq!(attempt_status(&root), "blocked");
        assert!(out.contains(&Outcome::NotifyBlocked { task_id: 5, attempt_n: 1 }));
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn blocked_notifies_once_and_updates_chip() {
        let root = fixture("blocked");
        let ev = envelope(
            "pane.agent_status_changed",
            serde_json::json!({"pane_id": "w2:p1", "agent_status": "blocked", "workspace_id": "w2"}),
        );
        let out = process_event(&root, &ev).unwrap();
        assert_eq!(attempt_status(&root), "blocked");
        assert!(out.contains(&Outcome::NotifyBlocked { task_id: 5, attempt_n: 1 }));
        assert!(out.iter().any(|o| matches!(o, Outcome::Chip { text, .. } if text == "0005 ⛔")));

        // 再次 blocked：状态未变 → 无通知、无 chip
        let out2 = process_event(&root, &ev).unwrap();
        assert_eq!(out2, vec![Outcome::Ignored]);
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn idle_is_not_done() {
        let root = fixture("idle");
        for st in ["idle", "done", "unknown"] {
            let ev = envelope(
                "pane.agent_status_changed",
                serde_json::json!({"pane_id": "w2:p1", "agent_status": st}),
            );
            process_event(&root, &ev).unwrap();
        }
        assert_eq!(attempt_status(&root), "running", "idle/done/unknown 不得改 attempt 终态");
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn unknown_pane_ignored() {
        let root = fixture("other");
        let ev = envelope(
            "pane.agent_status_changed",
            serde_json::json!({"pane_id": "w9:p9", "agent_status": "blocked"}),
        );
        assert_eq!(process_event(&root, &ev).unwrap(), vec![Outcome::Ignored]);
        assert_eq!(attempt_status(&root), "running");
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn pane_exit_and_worktree_remove_fail_active_attempt() {
        let root = fixture("gone");
        let ev = envelope("pane.exited", serde_json::json!({"pane_id": "w2:p1"}));
        let out = process_event(&root, &ev).unwrap();
        assert_eq!(attempt_status(&root), "failed");
        assert!(matches!(&out[0], Outcome::AttemptStatus { to, .. } if to == "failed"));

        // 已 failed → 再来 worktree.removed 也 Ignored
        let ev = envelope("worktree.removed", serde_json::json!({"workspace_id": "w2"}));
        assert_eq!(process_event(&root, &ev).unwrap(), vec![Outcome::Ignored]);
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn working_maps_to_running_unchanged() {
        let root = fixture("working");
        let ev = envelope(
            "pane.agent_status_changed",
            serde_json::json!({"pane_id": "w2:p1", "agent_status": "working"}),
        );
        assert_eq!(process_event(&root, &ev).unwrap(), vec![Outcome::Ignored], "running→running 无变化");
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }
}
