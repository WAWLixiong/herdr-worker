// act：六件套之文件四件套（create-task / move-task / add-comment / report-result）
// 调用方式：直接执行本二进制，argv 传参；全部操作在板文件锁内完成。
// 无参动作 open-board 注册在 manifest 上（herdr plugin action invoke / 键位绑定）。

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::lock::with_lock;
use crate::model::{self, Board};
use crate::write;

pub fn run(args: &[String]) -> Result<()> {
    let Some((cmd, rest)) = args.split_first() else {
        bail!("用法：herdr-worker act <create-task|move-task|add-comment|report-result|open-board> [flags]");
    };
    match cmd.as_str() {
        "create-task" => create_task(rest),
        "move-task" => move_task(rest),
        "add-comment" => add_comment(rest),
        "report-result" => report_result(rest),
        "start-attempt" => start_attempt(rest),
        "stop-attempt" => stop_attempt(rest),
        "follow-up" => follow_up(rest),
        "clean-worktree" => clean_worktree(rest),
        "open-board" => open_board(),
        other => bail!("未知 action：{other}"),
    }
}

fn start_attempt(args: &[String]) -> Result<()> {
    let flags = parse_flags(args);
    let id: u32 = one(&flags, "--id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("start-attempt 需要 --id N"))?;
    let agent = one(&flags, "--agent");
    let root = board_root(&flags)?;
    let started = crate::orchestrate::start_attempt(&root, id, agent)?;
    println!(
        "started {id:04}#{} agent pane={} workspace={}",
        started.n, started.pane_id, started.workspace_id
    );
    Ok(())
}

fn stop_attempt(args: &[String]) -> Result<()> {
    let flags = parse_flags(args);
    let id: u32 = one(&flags, "--id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("stop-attempt 需要 --id N"))?;
    let root = board_root(&flags)?;
    crate::orchestrate::stop_attempt(&root, id)?;
    println!("stopped {id:04} (worktree 保留)");
    Ok(())
}

fn follow_up(args: &[String]) -> Result<()> {
    let flags = parse_flags(args);
    let id: u32 = one(&flags, "--id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("follow-up 需要 --id N"))?;
    let root = board_root(&flags)?;
    let n = crate::orchestrate::follow_up(&root, id)?;
    println!("followed up {id:04} with {n} comments");
    Ok(())
}

fn clean_worktree(args: &[String]) -> Result<()> {
    let flags = parse_flags(args);
    let id: u32 = one(&flags, "--id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("clean-worktree 需要 --id N"))?;
    let root = board_root(&flags)?;
    crate::orchestrate::clean_worktree(&root, id)?;
    println!("cleaned {id:04} worktree (分支保留)");
    Ok(())
}

/// --key value / --key=value / 重复 flag 聚成 Vec；key 保留完整 `--key` 形式
fn parse_flags(args: &[String]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a.starts_with("--") {
            let (key, inline) = match a.split_once('=') {
                Some((k, v)) => (k.to_string(), Some(v.to_string())),
                None => (a.clone(), None),
            };
            let v = match inline {
                Some(v) => v,
                None => {
                    i += 1;
                    args.get(i).cloned().unwrap_or_default()
                }
            };
            map.entry(key).or_default().push(v);
        }
        i += 1;
    }
    map
}

fn one<'a>(flags: &'a HashMap<String, Vec<String>>, key: &str) -> Option<&'a str> {
    flags.get(key).and_then(|v| v.first()).map(|s| s.as_str())
}

fn board_root(flags: &HashMap<String, Vec<String>>) -> Result<PathBuf> {
    let start = one(flags, "--path")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)?;
    model::find_board(&start).ok_or_else(|| {
        anyhow::anyhow!("未找到 .herdr-worker/（从 {} 逐级向上）", start.display())
    })
}

fn read_body(flags: &HashMap<String, Vec<String>>) -> Result<String> {
    if let Some(s) = one(flags, "--body") {
        return Ok(s.to_string());
    }
    match one(flags, "--body-file") {
        Some("-") => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        Some(p) => fs::read_to_string(p).with_context(|| format!("读取 {p}")),
        None => Ok(String::new()),
    }
}

/// 引号规则：值含 [ " ' 或首尾空白时加双引号（解析器支持剥离）
fn yaml_str(s: &str) -> String {
    if s.contains('[') || s.contains('"') || s.contains('\'') || s != s.trim() {
        format!("\"{}\"", s.replace('"', "'"))
    } else {
        s.to_string()
    }
}

/// 目录名 slug：ASCII 小写化、空白与非法字符转 -、连续 - 折叠、CJK 保留、截到 ~24 显示宽
fn slugify(title: &str) -> String {
    let mapped: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c.is_ascii() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let mut out = String::new();
    let mut w = 0;
    let mut prev_dash = false;
    for ch in mapped.trim_matches('-').chars() {
        if ch == '-' {
            if prev_dash {
                continue;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
        }
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if w + cw > 24 {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.trim_matches('-').to_string()
}

fn next_id(root: &Path) -> u32 {
    let mut max = 0;
    if let Ok(entries) = fs::read_dir(root.join("tasks")) {
        for e in entries.flatten() {
            if let Some(n) = e
                .file_name()
                .to_string_lossy()
                .split('-')
                .next()
                .and_then(|s| s.parse::<u32>().ok())
            {
                max = max.max(n);
            }
        }
    }
    max + 1
}

fn create_task(args: &[String]) -> Result<()> {
    let flags = parse_flags(args);
    let Some(title) = one(&flags, "--title").map(String::from) else {
        bail!("create-task 需要 --title");
    };
    let status = one(&flags, "--status").unwrap_or("todo").to_string();
    let tags: Vec<String> = flags
        .get("--tag")
        .map(|v| v.iter().cloned())
        .unwrap_or_default()
        .collect();
    let body = read_body(&flags)?;
    let root = board_root(&flags)?;
    let id = create_task_core(&root, &title, &tags, &status, &body)?;
    println!("created {id:04}");
    Ok(())
}

/// 创建核心（TUI 的 n 键与 act create-task 共用）：锁内分配 id、写 task.md
pub fn create_task_core(
    root: &Path,
    title: &str,
    tags: &[String],
    status: &str,
    body: &str,
) -> Result<u32> {
    with_lock(root, || {
        let board = model::load_board(root)?;
        if !board.columns.iter().any(|(n, _)| n == status) {
            bail!(
                "未知列 {status}；board.md 声明的列：{}",
                board.columns.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ")
            );
        }
        let id = next_id(root);
        let dir = root
            .join("tasks")
            .join(format!("{:04}-{}", id, slugify(title)));
        fs::create_dir_all(&dir)?;
        let now = write::utc_now_rfc3339();
        let mut fm = String::from("---\n");
        fm.push_str(&format!("id: {:04}\n", id));
        fm.push_str(&format!("title: {}\n", yaml_str(title)));
        fm.push_str(&format!("status: {status}\n"));
        if !tags.is_empty() {
            let quoted: Vec<String> = tags.iter().map(|t| yaml_str(t)).collect();
            fm.push_str(&format!("tags: [{}]\n", quoted.join(", ")));
        }
        fm.push_str(&format!("created: {now}\n"));
        fm.push_str(&format!("updated: {now}\n"));
        fm.push_str("---\n\n");
        fm.push_str(body);
        if !body.ends_with('\n') {
            fm.push('\n');
        }
        fs::write(dir.join("task.md"), fm)?;
        Ok(id)
    })
}

fn find_task_dir(board: &Board, id: u32) -> Result<PathBuf> {
    board
        .tasks
        .iter()
        .find(|t| t.id == id)
        .map(|t| board.root.join("tasks").join(&t.dir_name))
        .ok_or_else(|| anyhow::anyhow!("任务 {id:04} 不存在"))
}

fn move_task(args: &[String]) -> Result<()> {
    let flags = parse_flags(args);
    let id: u32 = one(&flags, "--id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("move-task 需要 --id N"))?;
    let to = one(&flags, "--to")
        .ok_or_else(|| anyhow::anyhow!("move-task 需要 --to COLUMN"))?
        .to_string();
    let root = board_root(&flags)?;

    with_lock(&root, || {
        let board = model::load_board(&root)?;
        if !board.columns.iter().any(|(n, _)| n == &to) {
            bail!(
                "未知列 {to}；board.md 声明的列：{}",
                board.columns.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ")
            );
        }
        let dir = find_task_dir(&board, id)?;
        let path = dir.join("task.md");
        write::set_field(&path, "status", &to)?;
        write::set_field(&path, "updated", &write::utc_now_rfc3339())?;
        println!("moved {id:04} -> {to}");
        Ok(())
    })
}

fn context_author() -> String {
    if let Ok(ctx) = std::env::var("HERDR_PLUGIN_CONTEXT_JSON") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&ctx) {
            if let Some(agent) = v
                .get("focused_pane_agent")
                .and_then(|a| a.as_str())
                .filter(|s| !s.is_empty())
            {
                return agent.to_string();
            }
        }
    }
    "human".to_string()
}

fn add_comment(args: &[String]) -> Result<()> {
    let flags = parse_flags(args);
    let id: u32 = one(&flags, "--id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("add-comment 需要 --id N"))?;
    let author = one(&flags, "--author").map(String::from).unwrap_or_else(context_author);
    let file = one(&flags, "--file").map(String::from);
    let hunk = one(&flags, "--hunk").map(String::from);
    let body = read_body(&flags)?;
    let root = board_root(&flags)?;

    with_lock(&root, || {
        let board = model::load_board(&root)?;
        let dir = find_task_dir(&board, id)?;
        let cdir = dir.join("comments");
        fs::create_dir_all(&cdir)?;
        // 文件名：UTC 时间戳；同秒冲突则加序号
        let base = write::utc_now_rfc3339()
            .replace(['-', ':', 'T'], "")
            .replace("Z", "");
        let base = &base[..14.min(base.len())]; // YYYYMMDDHHMMSS
        let mut name = format!("{base}.md");
        let mut n = 1;
        while cdir.join(&name).exists() {
            n += 1;
            name = format!("{base}-{n}.md");
        }
        let mut fm = String::from("---\n");
        fm.push_str(&format!("author: {}\n", yaml_str(&author)));
        fm.push_str(&format!("time: {base}\n"));
        if let Some(f) = &file {
            fm.push_str(&format!("file: {}\n", yaml_str(f)));
        }
        if let Some(h) = &hunk {
            fm.push_str(&format!("hunk: {}\n", yaml_str(h)));
        }
        fm.push_str("---\n\n");
        fm.push_str(&body);
        if !body.ends_with('\n') {
            fm.push('\n');
        }
        fs::write(cdir.join(&name), fm)?;
        println!("commented {id:04} {name}");
        Ok(())
    })
}

fn report_result(args: &[String]) -> Result<()> {
    let flags = parse_flags(args);
    let id: u32 = one(&flags, "--id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("report-result 需要 --id N"))?;
    let n: u32 = one(&flags, "--attempt")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("report-result 需要 --attempt M"))?;
    let status = one(&flags, "--status")
        .ok_or_else(|| anyhow::anyhow!("report-result 需要 --status done|failed|cancelled"))?
        .to_string();
    if !matches!(status.as_str(), "done" | "failed" | "cancelled") {
        bail!("report-result 的 status 只接受 done|failed|cancelled（活状态归 sync 管）");
    }
    let usage = one(&flags, "--usage").map(String::from);
    let summary = read_body(&flags)?;
    let root = board_root(&flags)?;

    with_lock(&root, || {
        let board = model::load_board(&root)?;
        let dir = find_task_dir(&board, id)?;
        let att_path = dir.join("attempts").join(format!("{n}.md"));
        if !att_path.is_file() {
            bail!("任务 {id:04} 没有 attempt {n}");
        }
        let task = board.tasks.iter().find(|t| t.id == id).unwrap();
        let att = task
            .attempts
            .iter()
            .find(|a| a.n == n)
            .ok_or_else(|| anyhow::anyhow!("attempt {n} 解析失败"))?;

        // 整文件重写：frontmatter 是我们自己的格式；保留 pane/workspace/agent_name（事件大脑定位用）
        let now = write::utc_now_rfc3339();
        let mut fm = String::from("---\n");
        fm.push_str(&format!("n: {n}\n"));
        fm.push_str(&format!("agent: {}\n", yaml_str(&att.agent)));
        fm.push_str(&format!("branch: {}\n", yaml_str(&att.branch)));
        if let Some(p) = &att.pane {
            fm.push_str(&format!("pane: {p}\n"));
        }
        if let Some(w) = &att.workspace {
            fm.push_str(&format!("workspace: {w}\n"));
        }
        if let Some(a) = &att.agent_name {
            fm.push_str(&format!("agent_name: {a}\n"));
        }
        if let Some(s) = &att.started {
            fm.push_str(&format!("started: {s}\n"));
        }
        fm.push_str(&format!("ended: {now}\n"));
        fm.push_str(&format!("status: {status}\n"));
        if let Some(u) = usage.as_deref().or(att.usage.as_deref()) {
            fm.push_str(&format!("usage: {}\n", yaml_str(u)));
        }
        fm.push_str("---\n\n");
        fm.push_str(&summary);
        if !summary.ends_with('\n') {
            fm.push('\n');
        }
        fs::write(&att_path, fm)?;

        // 自报 done → 任务自动入 review（SPEC §5 工作流）
        if status == "done" && task.status != "done" && task.status != "cancelled" {
            let task_md = dir.join("task.md");
            write::set_field(&task_md, "status", "review")?;
            write::set_field(&task_md, "updated", &now)?;
        }
        println!("reported {id:04}#{n} {status}");
        Ok(())
    })
}

fn open_board() -> Result<()> {
    let bin = std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
    let out = std::process::Command::new(&bin)
        .args(["plugin", "pane", "open", "--plugin", "herdr-worker", "--entrypoint", "board"])
        .output()
        .with_context(|| format!("调用 {bin} 失败"))?;
    std::io::Write::write_all(&mut std::io::stdout(), &out.stdout)?;
    std::io::Write::write_all(&mut std::io::stderr(), &out.stderr)?;
    if !out.status.success() {
        bail!("plugin pane open 退出码 {:?}", out.status.code());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::load_board;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("herdr-worker-act-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn with_fixture(tag: &str) -> (PathBuf, PathBuf) {
        // 返回 (base, board_root)：base 含 .herdr-worker 子目录，作为 --path 起点
        let base = temp_root(tag);
        let board = base.join(".herdr-worker");
        fs::create_dir_all(board.join("tasks")).unwrap();
        (base, board)
    }

    fn argify(root: &Path, s: &str) -> Vec<String> {
        s.split_whitespace()
            .map(String::from)
            .chain([format!("--path"), root.display().to_string()])
            .collect()
    }

    #[test]
    fn create_move_comment_report_flow() {
        let (base, root) = with_fixture("flow");

        create_task(&argify(&base, "create-task --title 修复：登录页 --status todo --tag bug --tag frontend --body 正文即prompt"))
            .unwrap();
        let board = load_board(&root).unwrap();
        assert_eq!(board.tasks.len(), 1);
        let t = &board.tasks[0];
        assert_eq!(t.id, 1);
        assert_eq!(t.title, "修复：登录页");
        assert_eq!(t.tags, vec!["bug", "frontend"]);
        assert_eq!(t.body.trim(), "正文即prompt");
        assert_eq!(t.status, "todo");
        assert!(t.dir_name.starts_with("0001-"));

        // 第二个任务 id 递增
        create_task(&argify(&base, "create-task --title 第二个")).unwrap();
        assert_eq!(load_board(&root).unwrap().tasks.len(), 2);

        // 非法列名拒绝
        let err = move_task(&argify(&base, "move-task --id 1 --to 不存在的列"));
        assert!(err.is_err());

        // 合法移动（含隐藏列 cancelled）
        move_task(&argify(&base, "move-task --id 2 --to cancelled")).unwrap();
        assert_eq!(load_board(&root).unwrap().tasks[1].status, "cancelled");

        // attempt + 自报 done → 自动入 review
        let dir = root.join("tasks").join(&load_board(&root).unwrap().tasks[0].dir_name);
        fs::create_dir_all(dir.join("attempts")).unwrap();
        fs::write(
            dir.join("attempts").join("1.md"),
            "---\nn: 1\nagent: opencode\nbranch: hw/0001-x\nstatus: running\n---\n跑着\n",
        )
        .unwrap();
        add_comment(&argify(&base, "add-comment --id 1 --author human --body 要判空")).unwrap();
        report_result(&argify(&base, "report-result --id 1 --attempt 1 --status done --usage 45k --body 完成")).unwrap();

        let board = load_board(&root).unwrap();
        let t = &board.tasks[0];
        assert_eq!(t.status, "review", "自报 done 应自动入 review");
        assert_eq!(t.attempts[0].status, "done");
        assert_eq!(t.attempts[0].usage.as_deref(), Some("45k"));
        assert_eq!(t.attempts[0].summary.trim(), "完成");
        assert_eq!(t.comments.len(), 1);
        assert_eq!(t.comments[0].body.trim(), "要判空");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn concurrent_create_task_ids_do_not_collide() {
        let (base, root) = with_fixture("concurrent");
        let b1 = base.clone();
        let b2 = base.clone();
        let h1 = std::thread::spawn(move || {
            create_task(&argify(&b1, "create-task --title 甲任务")).is_ok()
        });
        let h2 = std::thread::spawn(move || {
            create_task(&argify(&b2, "create-task --title 乙任务")).is_ok()
        });
        assert!(h1.join().unwrap() && h2.join().unwrap());
        let board = load_board(&root).unwrap();
        assert_eq!(board.tasks.len(), 2);
        let ids: Vec<u32> = board.tasks.iter().map(|t| t.id).collect();
        assert_ne!(ids[0], ids[1], "并发建卡 id 不得相撞");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn slugify_handles_cjk_and_illegal() {
        assert_eq!(slugify("Fix Login: page"), "fix-login-page");
        assert!(!slugify("修复：登录页白屏").is_empty());
        assert!(slugify("修复：登录页白屏").chars().all(|c| !r#"<>:"/\|?*"#.contains(c)));
    }
}
