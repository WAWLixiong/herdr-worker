// 编排：s 键三步全托管（SPEC §5）
// worktree create → attempt 文件 → agent start → agent prompt → 自动 in-progress → 侧栏 chip
// agent prompt = 任务正文 + API 坐标页脚（二进制绝对路径 + 主板 --path + 本任务 id/attempt）

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

use crate::lock::with_lock;
use crate::model::{self, Board};
use crate::write;

#[derive(Debug)]
pub struct StartedAttempt {
    pub n: u32,
    pub pane_id: String,
    pub workspace_id: String,
    pub agent_name: String,
}

fn herdr_bin() -> String {
    std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string())
}

/// 查询 agent 当前状态（轻量容错：失败返回 None）
fn agent_status_of(name: &str) -> Option<String> {
    let out = Command::new(herdr_bin())
        .args(["agent", "get", name])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).ok()?;
    v.pointer("/result/agent_status")
        .or_else(|| v.pointer("/result/agent/agent_status"))
        .and_then(|s| s.as_str())
        .map(String::from)
}

fn herdr_json(args: &[&str]) -> Result<serde_json::Value> {
    let out = Command::new(herdr_bin())
        .args(args)
        .output()
        .with_context(|| format!("调用 {} 失败", herdr_bin()))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        bail!("herdr {} 失败（{}）：{}", args.first().unwrap_or(&""), out.status, stderr.trim());
    }
    serde_json::from_str(&stdout).with_context(|| format!("解析 herdr 输出失败：{stdout}"))
}

/// 宽容取值：多路径尝试
fn pick<'a>(v: &'a serde_json::Value, paths: &[&str]) -> Option<&'a str> {
    for p in paths {
        if let Some(s) = v.pointer(p).and_then(|x| x.as_str()) {
            return Some(s);
        }
    }
    // 有些响应把 result 展平或嵌套不同，做一次模糊搜索：字段名匹配
    None
}

/// prompt 页脚：给 agent 的 API 坐标（worktree 里没有 target/，必须给绝对路径与主板路径）
pub fn prompt_footer(board_root: &Path, task_id: u32, n: u32) -> String {
    let repo = board_root
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "herdr-worker".to_string());
    format!(
        "\n\n---\nherdr-worker 板 API（用绝对路径操作主板，勿写你所在 worktree 的 .herdr-worker 副本）：\n\
         报告结果：{exe} act report-result --path \"{repo}\" --id {task_id} --attempt {n} --status done|failed [--usage \"45k tok\"] --body \"摘要\"\n\
         评论：{exe} act add-comment --path \"{repo}\" --id {task_id} --body \"内容\"\n\
         完成（或失败）前请务必 report-result。"
    )
}

fn attempt_file_content(
    n: u32,
    agent: &str,
    branch: &str,
    pane_id: &str,
    workspace_id: &str,
    agent_name: &str,
) -> String {
    let now = write::utc_now_rfc3339();
    format!(
        "---\nn: {n}\nagent: {agent}\nbranch: {branch}\npane: {pane_id}\nworkspace: {workspace_id}\nagent_name: {agent_name}\nstarted: {now}\nstatus: running\n---\n"
    )
}

pub fn start_attempt(
    board_root: &Path,
    task_id: u32,
    agent_override: Option<&str>,
) -> Result<StartedAttempt> {
    let repo = board_root
        .parent()
        .context("板根目录必须有父级（仓库根）")?
        .display()
        .to_string();

    with_lock(board_root, || {
        let board = model::load_board(board_root)?;
        let task = board
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 不存在"))?;

        // 重复启动拒绝：已有 running/blocked 的 attempt
        if let Some(a) = task.latest_attempt() {
            if matches!(a.status.as_str(), "running" | "blocked") {
                bail!("任务 {task_id:04} 已有进行中的 attempt #{}（先 stop 或等它结束）", a.n);
            }
        }
        let n = (task.attempts.len() + 1) as u32;
        let agent_kind = agent_override.unwrap_or(&board.default_agent).to_string();
        let branch = format!("{}/{}", board.branch_prefix, task.dir_name);
        let agent_name = format!("hw{task_id:04}-{n}");
        let label = format!("hw:{task_id:04}");

        // 1) worktree create（root pane 即天然空闲 shell pane）
        let out = herdr_json(&[
            "worktree", "create",
            "--cwd", &repo,
            "--branch", &branch,
            "--label", &label,
            "--no-focus",
        ])?;
        let workspace_id = pick(&out, &["/result/workspace/workspace_id", "/result/workspace/id"])
            .context("worktree create 响应缺 workspace_id")?
            .to_string();
        let pane_id = pick(&out, &["/result/root_pane/pane_id", "/result/root_pane/id"])
            .context("worktree create 响应缺 pane_id")?
            .to_string();

        // 2) attempt 文件（先落盘，事件大脑 T05 靠 pane/workspace 定位）
        let att_dir = board_root
            .join("tasks")
            .join(&task.dir_name)
            .join("attempts");
        std::fs::create_dir_all(&att_dir)?;
        let att_path = att_dir.join(format!("{n}.md"));
        std::fs::write(
            &att_path,
            attempt_file_content(n, &agent_kind, &branch, &pane_id, &workspace_id, &agent_name),
        )?;

        // 3) agent start；失败则回滚 attempt 文件（worktree 留给用户处置）
        if let Err(e) = herdr_json(&[
            "agent", "start", &agent_name, "--kind", &agent_kind, "--pane", &pane_id,
        ]) {
            let _ = std::fs::remove_file(&att_path);
            bail!("agent start 失败（attempt 已回滚；worktree {} 保留，可手动清理）：{e}", workspace_id);
        }
        // 时序修复：agent start 返回"就绪"≠输入框可用（欢迎屏初始化期粘贴会被吞）；
        // 等 idle 再发 prompt。超时/失败则照发（宁可迟发不可不发）。
        let _ = herdr_json(&[
            "agent", "wait", &agent_name, "--until", "idle", "--timeout", "15000",
        ]);

        // 4) agent prompt：正文 + API 页脚
        //    fresh opencode 的首条粘贴可能被欢迎屏吞掉（状态停在 idle、无输出）——
        //    发出后轮询 ~6s，未观察到 working/blocked 则重发，最多 3 次
        let prompt = format!(
            "{}{}",
            task.body.trim(),
            prompt_footer(board_root, task_id, n)
        );
        let prompt_text = if prompt.trim().is_empty() {
            task.title.clone()
        } else {
            prompt
        };
        let mut landed = false;
        for round in 0..3 {
            if let Err(e) = herdr_json(&["agent", "prompt", &agent_name, &prompt_text]) {
                // prompt 被拒（如 agent_blocked）不重试
                eprintln!("警告：agent prompt 被拒（agent 已启动）：{e}");
                landed = true;
                break;
            }
            for _ in 0..8 {
                std::thread::sleep(std::time::Duration::from_millis(700));
                if let Some(st) = agent_status_of(&agent_name) {
                    if matches!(st, "working" | "blocked") {
                        landed = true;
                        break;
                    }
                }
            }
            if landed {
                break;
            }
            eprintln!("警告：prompt 第 {} 次未见落袋（agent 空闲无反应），重发", round + 1);
        }
        if !landed {
            eprintln!("警告：prompt 三次未确认提交（agent 已启动，请手动跟进）");
        }

        // 5) 任务自动入 in-progress
        let task_md = board_root
            .join("tasks")
            .join(&task.dir_name)
            .join("task.md");
        write::set_field(&task_md, "status", "in-progress")?;
        write::set_field(&task_md, "updated", &write::utc_now_rfc3339())?;

        // 6) 侧栏 chip（best effort，失败不影响启动）
        let _ = herdr_json(&[
            "workspace", "report-metadata", &workspace_id,
            "--source", "herdr-worker",
            "--token", &format!("hw={task_id:04} ▶"),
        ]);

        println!(
            "started {task_id:04}#{n} agent={agent_kind} branch={branch} pane={pane_id} workspace={workspace_id}"
        );
        Ok(StartedAttempt { n, pane_id, workspace_id, agent_name })
    })
}

/// 供 TUI 使用：板加载后的活跃 attempt 检查
pub fn has_active_attempt(board: &Board, task_id: u32) -> bool {
    board
        .tasks
        .iter()
        .find(|t| t.id == task_id)
        .and_then(|t| t.latest_attempt())
        .map(|a| matches!(a.status.as_str(), "running" | "blocked"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_has_coordinates() {
        let root = Path::new("/repo/.herdr-worker");
        let f = prompt_footer(root, 4, 2);
        assert!(f.contains("--id 4 --attempt 2"));
        assert!(f.contains("report-result"));
        assert!(f.contains("act add-comment"));
    }

    #[test]
    fn attempt_file_shape() {
        let s = attempt_file_content(2, "opencode", "hw/0004-x", "w2:p1", "w2", "hw0004-2");
        assert!(s.contains("n: 2\n"));
        assert!(s.contains("pane: w2:p1\n"));
        assert!(s.contains("workspace: w2\n"));
        assert!(s.contains("agent_name: hw0004-2\n"));
        assert!(s.contains("status: running\n"));
    }

    #[test]
    fn duplicate_active_attempt_rejected_before_any_herdr_call() {
        let base = std::env::temp_dir().join(format!("herdr-worker-orch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join(".herdr-worker");
        let t = root.join("tasks").join("0009-busy");
        std::fs::create_dir_all(t.join("attempts")).unwrap();
        std::fs::write(
            t.join("task.md"),
            "---\nid: 0009\ntitle: 忙\nstatus: in-progress\n---\n正文\n",
        )
        .unwrap();
        std::fs::write(
            t.join("attempts").join("1.md"),
            "---\nn: 1\nagent: opencode\nbranch: hw/0009-busy\nstatus: running\n---\n",
        )
        .unwrap();

        // running → 拒绝；且在调用 herdr 之前就拒绝（消息可判别）
        let err = start_attempt(&root, 9, None).unwrap_err();
        assert!(err.to_string().contains("进行中的 attempt"));

        std::fs::remove_dir_all(&base).unwrap();
    }
}
