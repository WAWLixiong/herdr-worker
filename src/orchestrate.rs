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
    // 有些命令（如 pane run）成功时无输出——宽容返回 Null
    if stdout.trim().is_empty() {
        return Ok(serde_json::Value::Null);
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
                    if matches!(st.as_str(), "working" | "blocked") {
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

// ---------- 06：人侧闭环 ----------

/// r：review → done（人审确认）
pub fn confirm_review(board_root: &Path, task_id: u32) -> Result<()> {
    with_lock(board_root, || {
        let board = model::load_board(board_root)?;
        let task = board
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 不存在"))?;
        if task.status != "review" {
            bail!("只有 review 列的任务能确认入 done（当前在 {status} 列）", status = task.status);
        }
        let path = board
            .task_path(task_id)
            .ok_or_else(|| anyhow::anyhow!("找不到任务文件"))?;
        write::set_field(&path, "status", "done")?;
        write::set_field(&path, "updated", &write::utc_now_rfc3339())?;
        println!("confirmed {task_id:04} -> done");
        Ok(())
    })
}

/// follow-up 的纯准备段（可测）：返回 (agent 名, prompt 文本, 待标记的评论文件)
fn prepare_follow_up(board: &Board, task_id: u32) -> Result<(String, String, Vec<std::path::PathBuf>)> {
    let task = board
        .tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 不存在"))?;
    let att = task
        .latest_attempt()
        .filter(|a| matches!(a.status.as_str(), "running" | "blocked"))
        .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 没有进行中的 attempt"))?;
    if att.status == "blocked" {
        bail!("agent 正在 blocked 等你回答——去 pane 里答复；f 仅用于补充反馈");
    }
    let name = att
        .agent_name
        .clone()
        .ok_or_else(|| anyhow::anyhow!("attempt 没有记录 agent 名"))?;
    let undelivered: Vec<&crate::model::Comment> =
        task.comments.iter().filter(|c| !c.delivered).collect();
    if undelivered.is_empty() {
        bail!("没有未投递的评论（先 act add-comment 或手写评论文件）");
    }
    let mut text = String::from("用户补充反馈（逐条处理）：\n");
    for c in &undelivered {
        let loc = match (&c.file, &c.hunk) {
            (Some(f), Some(h)) => format!("（位置 {f} {h}）"),
            (Some(f), None) => format!("（位置 {f}）"),
            _ => String::new(),
        };
        text.push_str(&format!("- {loc}{}\n", c.body.trim()));
    }
    let files: Vec<std::path::PathBuf> = undelivered
        .iter()
        .map(|c| {
            board
                .root
                .join("tasks")
                .join(&task.dir_name)
                .join("comments")
                .join(&c.file_name)
        })
        .collect();
    Ok((name, text, files))
}

/// f：向活跃 attempt 的 agent 追加 prompt，携带未投递评论并标记已投递
pub fn follow_up(board_root: &Path, task_id: u32) -> Result<usize> {
    with_lock(board_root, || {
        let board = model::load_board(board_root)?;
        let (name, text, files) = prepare_follow_up(&board, task_id)?;
        herdr_json(&["agent", "prompt", &name, &text])?;
        for f in &files {
            write::set_field(f, "delivered", "true")?;
        }
        println!("followed up {task_id:04} with {} comments", files.len());
        Ok(files.len())
    })
}

/// x：停止活跃 attempt（ctrl+c → 尽力 release → cancelled；worktree 保留）
pub fn stop_attempt(board_root: &Path, task_id: u32) -> Result<()> {
    with_lock(board_root, || {
        let board = model::load_board(board_root)?;
        let task = board
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 不存在"))?;
        let att = task
            .latest_attempt()
            .filter(|a| matches!(a.status.as_str(), "running" | "blocked"))
            .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 没有进行中的 attempt"))?;
        if let Some(name) = &att.agent_name {
            let _ = herdr_json(&["agent", "send-keys", name, "ctrl+c"]);
        }
        std::thread::sleep(std::time::Duration::from_millis(600));
        if let Some(pane) = &att.pane {
            let _ = herdr_json(&[
                "pane", "release-agent", pane,
                "--source", "herdr-worker",
                "--agent", &att.agent,
            ]);
        }
        let att_path = board
            .root
            .join("tasks")
            .join(&task.dir_name)
            .join("attempts")
            .join(format!("{}.md", att.n));
        write::set_field(&att_path, "status", "cancelled")?;
        write::set_field(&att_path, "ended", &write::utc_now_rfc3339())?;
        println!("stopped {task_id:04}#{} (worktree 保留)", att.n);
        Ok(())
    })
}

/// c：清理插件自建 worktree（分支永不删）——校验段可测
fn validate_clean(board: &Board, task_id: u32) -> Result<String> {
    let task = board
        .tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 不存在"))?;
    let att = task
        .latest_attempt()
        .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 没有 attempt"))?;
    let ws = att
        .workspace
        .clone()
        .ok_or_else(|| anyhow::anyhow!("attempt 没有记录 workspace"))?;
    if !att
        .branch
        .starts_with(&format!("{}/", board.branch_prefix))
    {
        bail!(
            "分支 {} 不是插件前缀 {} 开头，拒绝清理",
            att.branch,
            board.branch_prefix
        );
    }
    Ok(ws)
}

pub fn clean_worktree(board_root: &Path, task_id: u32) -> Result<()> {
    let ws = with_lock(board_root, || {
        let board = model::load_board(board_root)?;
        validate_clean(&board, task_id)
    })?;
    // 锁外调 herdr（remove 可能慢；事件钩子会看到 worktree.removed）
    if let Err(first) = herdr_json(&["worktree", "remove", "--workspace", &ws]) {
        // Windows 竞态：herdr 关 workspace 杀进程是异步的，git 删目录可能抢在锁释放前
        // → 等 1.5s 重试 --force；若 worktree 已不在列表（已实质移除）按成功处理
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let _ = herdr_json(&["worktree", "remove", "--workspace", &ws, "--force"]);
        let still_listed = herdr_json(&["worktree", "list"])
            .ok()
            .and_then(|l| {
                l.pointer("/result/worktrees").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter().any(|w| {
                        w.get("open_workspace_id").and_then(|s| s.as_str()) == Some(ws.as_str())
                    })
                })
            })
            .unwrap_or(true);
        if !still_listed {
            println!("cleaned worktree {ws}（目录若因进程锁残留可稍后手删；分支保留）");
            return Ok(());
        }
        return Err(anyhow::anyhow!(
            "worktree remove 失败（worktree 脏或被进程锁定时需要 --force；分支不受影响）：{first}"
        ));
    }
    println!("cleaned worktree {ws}");
    Ok(())
}

/// d：在兄弟 pane 打开任务 worktree 的 diff（delta → difftastic → git diff 回退）
pub fn open_diff(board_root: &Path, task_id: u32) -> Result<()> {
    let board = model::load_board(board_root)?;
    let task = board
        .tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 不存在"))?;
    let att = task
        .latest_attempt()
        .filter(|a| a.workspace.is_some())
        .ok_or_else(|| anyhow::anyhow!("任务 {task_id:04} 没有关联 worktree 的 attempt"))?;
    let ws = att.workspace.clone().unwrap();

    // worktree list → path
    let list = herdr_json(&["worktree", "list"])?;
    let path = list
        .pointer("/result/worktrees")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|w| w.get("open_workspace_id").and_then(|s| s.as_str()) == Some(ws.as_str()))
                .and_then(|w| w.get("path").and_then(|p| p.as_str()))
                .map(String::from)
        })
        .ok_or_else(|| anyhow::anyhow!("worktree list 里找不到 workspace {ws}（可能已清理）"))?;

    // 渲染器探测链
    let renderer = ["delta", "difft"]
        .iter()
        .find(|b| {
            std::process::Command::new(b)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map(|mut c| c.wait().map(|s| s.success()).unwrap_or(false))
                .unwrap_or(false)
        })
        .copied();
    let cmd = match renderer {
        Some("delta") => format!("git -C \"{path}\" diff HEAD | delta"),
        Some("difft") => format!("git -C \"{path}\" diff HEAD | difft"),
        _ => format!("git -C \"{path}\" diff HEAD"),
    };

    let split = herdr_json(&[
        "pane", "split", "--current", "--direction", "right", "--no-focus",
    ])?;
    let pane_id = ["/result/pane/pane_id", "/result/pane_id", "/result/root_pane/pane_id"]
        .iter()
        .find_map(|p| split.pointer(p).and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow::anyhow!("pane split 响应缺 pane_id"))?;
    herdr_json(&["pane", "run", pane_id, &cmd])?;
    println!("diff opened in {pane_id} ({})", renderer.unwrap_or("git diff"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn review_and_followup_and_clean_guards() {
        let base = std::env::temp_dir().join(format!("herdr-worker-t06-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join(".herdr-worker");
        let t = root.join("tasks").join("0006-loop");
        fs::create_dir_all(t.join("attempts")).unwrap();
        fs::create_dir_all(t.join("comments")).unwrap();
        fs::write(
            t.join("task.md"),
            "---\nid: 0006\ntitle: 闭环\nstatus: review\n---\n正文\n",
        )
        .unwrap();
        fs::write(
            t.join("attempts").join("1.md"),
            "---\nn: 1\nagent: opencode\nbranch: hw/0006-loop\npane: w2:p1\nworkspace: w2\nagent_name: hw0006-1\nstatus: running\n---\n",
        )
        .unwrap();
        fs::write(
            t.join("comments").join("20260926-a.md"),
            "---\nauthor: human\ntime: 09-26 10:00\nfile: src/a.rs\n---\n这里要判空\n",
        )
        .unwrap();
        fs::write(
            t.join("comments").join("20260926-b.md"),
            "---\nauthor: human\ntime: 09-26 10:01\ndelivered: true\n---\n旧反馈\n",
        )
        .unwrap();

        let board = model::load_board(&root).unwrap();

        // r：非 review 报错
        // （当前就是 review，直接测确认流程的纯文件段）—— confirm_review 会调 herdr？不会：它只写文件
        confirm_review(&root, 6).unwrap();
        let board = model::load_board(&root).unwrap();
        assert_eq!(board.tasks[0].status, "done");
        assert!(confirm_review(&root, 6).is_err(), "done 列再按 r 应报错");

        // f：准备段组合未投递评论（b 已投递，只带 a）
        let board = model::load_board(&root).unwrap();
        // 把任务挪回 in-progress 以便 follow_up（直接改文件模拟 sync）
        write::set_field(&root.join("tasks").join("0006-loop").join("task.md"), "status", "in-progress").unwrap();
        let board = model::load_board(&root).unwrap();
        let (name, text, files) = prepare_follow_up(&board, 6).unwrap();
        assert_eq!(name, "hw0006-1");
        assert!(text.contains("（位置 src/a.rs）这里要判空"), "文本应含位置与内容：{text}");
        assert!(!text.contains("旧反馈"), "已投递评论不得重复携带");
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("20260926-a.md"));

        // f：没有未投递评论时报错（标记 a 为已投递后再试）
        write::set_field(&files[0], "delivered", "true").unwrap();
        let board = model::load_board(&root).unwrap();
        assert!(prepare_follow_up(&board, 6).is_err());

        // c：分支前缀校验（换成非插件前缀应拒绝）
        let board = model::load_board(&root).unwrap();
        assert!(validate_clean(&board, 6).is_ok());
        fs::write(
            t.join("attempts").join("1.md"),
            "---\nn: 1\nagent: opencode\nbranch: feature/other\npane: w2:p1\nworkspace: w2\nagent_name: hw0006-1\nstatus: done\n---\n",
        )
        .unwrap();
        let board = model::load_board(&root).unwrap();
        assert!(validate_clean(&board, 6).is_err(), "非插件前缀的 worktree 拒绝清理");

        fs::remove_dir_all(&base).unwrap();
    }

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
