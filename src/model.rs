// 数据层：`.herdr-worker/` 目录 → Board 模型（schema 见 SPEC.md §4）

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::frontmatter::{self, Frontmap};

pub const DEFAULT_COLUMNS: &[(&str, bool)] = &[
    ("backlog", true),
    ("todo", false),
    ("in-progress", false),
    ("review", false),
    ("done", false),
    ("cancelled", true),
];

pub struct Board {
    pub root: PathBuf,
    /// (列名, 是否隐藏)，声明顺序即展示顺序
    pub columns: Vec<(String, bool)>,
    pub branch_prefix: String,
    pub default_agent: String,
    pub tasks: Vec<Task>,
}

pub struct Task {
    pub id: u32,
    pub dir_name: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub tags: Vec<String>,
    pub parent: Option<u32>,
    pub blocked_by: Vec<u32>,
    pub order: Option<i64>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub body: String,
    pub attempts: Vec<Attempt>,
    pub comments: Vec<Comment>,
}

impl Task {
    pub fn latest_attempt(&self) -> Option<&Attempt> {
        self.attempts.last()
    }
}

pub struct Attempt {
    pub n: u32,
    pub agent: String,
    pub branch: String,
    pub pane: Option<String>,
    pub workspace: Option<String>,
    pub agent_name: Option<String>,
    pub started: Option<String>,
    pub ended: Option<String>,
    pub status: String,
    pub usage: Option<String>,
    pub summary: String,
}

pub struct Comment {
    pub author: String,
    pub time: Option<String>,
    pub file: Option<String>,
    pub hunk: Option<String>,
    pub body: String,
}

impl Board {
    /// 可见列 + 未声明状态的影子列（防御手写状态）
    pub fn visible_columns(&self) -> Vec<String> {
        let mut cols: Vec<String> = self
            .columns
            .iter()
            .filter(|(_, hidden)| !hidden)
            .map(|(n, _)| n.clone())
            .collect();
        let mut extra: Vec<String> = self
            .tasks
            .iter()
            .map(|t| t.status.clone())
            .filter(|s| !self.columns.iter().any(|(n, _)| n == s))
            .collect();
        extra.sort();
        extra.dedup();
        cols.extend(extra);
        cols
    }

    pub fn tasks_in(&self, col: &str) -> Vec<&Task> {
        let mut ts: Vec<&Task> = self.tasks.iter().filter(|t| t.status == col).collect();
        ts.sort_by(|a, b| {
            let ka = (a.order.unwrap_or(i64::MAX), a.id);
            let kb = (b.order.unwrap_or(i64::MAX), b.id);
            ka.cmp(&kb)
        });
        ts
    }

    pub fn task_path(&self, id: u32) -> Option<PathBuf> {
        self.tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| self.root.join("tasks").join(&t.dir_name).join("task.md"))
    }
}

/// 目录快照（路径 + mtime），轮询监听用；跳过锁文件
pub fn fs_snapshot(root: &Path) -> Vec<(PathBuf, std::time::SystemTime)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, std::time::SystemTime)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name();
            if name == ".lock" {
                continue;
            }
            if p.is_dir() {
                walk(&p, out);
            } else if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                out.push((p, m));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

/// 从 start 向上查找 `.herdr-worker/`
pub fn find_board(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(".herdr-worker");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn load_board(root: &Path) -> Result<Board> {
    let mut columns: Vec<(String, bool)> = DEFAULT_COLUMNS
        .iter()
        .map(|(n, h)| (n.to_string(), *h))
        .collect();
    let mut branch_prefix = "hw".to_string();
    let mut default_agent = "claude-code".to_string();

    let board_md = root.join("board.md");
    if board_md.is_file() {
        let src = fs::read_to_string(&board_md).with_context(|| format!("读取 {}", board_md.display()))?;
        let (fm, _) = frontmatter::split(&src);
        if let Some(fm) = fm {
            let m = Frontmap::parse(&fm)
                .with_context(|| format!("解析 {}", board_md.display()))?;
            let names = m.str_list("columns");
            if !names.is_empty() {
                let hidden: Vec<String> = m.str_list("hidden");
                columns = names
                    .into_iter()
                    .map(|n| {
                        let hidden_col = hidden.contains(&n);
                        (n, hidden_col)
                    })
                    .collect();
            }
            if let Some(p) = m.str("branch_prefix") {
                branch_prefix = p.to_string();
            }
            if let Some(a) = m.str("default_agent") {
                default_agent = a.to_string();
            }
        }
    }

    let mut tasks = Vec::new();
    let tasks_dir = root.join("tasks");
    if tasks_dir.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&tasks_dir)
            .with_context(|| format!("读取 {}", tasks_dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        entries.sort();
        for dir in entries {
            if let Some(t) = load_task(&dir)
                .with_context(|| format!("解析任务目录 {}", dir.display()))?
            {
                tasks.push(t);
            }
        }
    }
    tasks.sort_by_key(|t| t.id);

    Ok(Board {
        root: root.to_path_buf(),
        columns,
        branch_prefix,
        default_agent,
        tasks,
    })
}

fn load_task(dir: &Path) -> Result<Option<Task>> {
    let task_md = dir.join("task.md");
    if !task_md.is_file() {
        return Ok(None);
    }
    let src = fs::read_to_string(&task_md)?;
    let (fm, body) = frontmatter::split(&src);
    let m = match fm {
        Some(fm) => Some(Frontmap::parse(&fm)?),
        None => None,
    };
    let dir_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let id = dir_name
        .split('-')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .or_else(|| m.as_ref().and_then(|m| m.int("id")).map(|i| i as u32));

    let title = m
        .as_ref()
        .and_then(|m| m.str("title"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| dir_name.clone());

    let status = m
        .as_ref()
        .and_then(|m| m.str("status"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "todo".to_string());

    let attempts = load_attempts(&dir.join("attempts"))?;
    let comments = load_comments(&dir.join("comments"))?;

    Ok(Some(Task {
        id: id.ok_or_else(|| anyhow::anyhow!("无法确定任务 id（目录名或 frontmatter）"))?,
        dir_name,
        title,
        status,
        priority: m.as_ref().and_then(|m| m.str("priority")).map(String::from),
        tags: m.as_ref().map(|m| m.str_list("tags")).unwrap_or_default(),
        parent: m.as_ref().and_then(|m| m.int("parent")).map(|i| i as u32),
        blocked_by: m
            .as_ref()
            .map(|m| m.int_list("blocked-by"))
            .unwrap_or_default()
            .into_iter()
            .map(|i| i as u32)
            .collect(),
        order: m.as_ref().and_then(|m| m.int("order")),
        created: m.as_ref().and_then(|m| m.str("created")).map(String::from),
        updated: m.as_ref().and_then(|m| m.str("updated")).map(String::from),
        body,
        attempts,
        comments,
    }))
}

fn load_attempts(dir: &Path) -> Result<Vec<Attempt>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().map(|e| e == "md").unwrap_or(false))
        .collect();
    entries.sort();
    for path in entries {
        let src = fs::read_to_string(&path)?;
        let (fm, body) = frontmatter::split(&src);
        let m = fm.map(|fm| Frontmap::parse(&fm)).transpose()?;
        let file_stem = path
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let n = file_stem
            .parse::<u32>()
            .ok()
            .or_else(|| m.as_ref().and_then(|m| m.int("n")).map(|i| i as u32))
            .unwrap_or(0);
        out.push(Attempt {
            n,
            agent: m
                .as_ref()
                .and_then(|m| m.str("agent"))
                .unwrap_or("unknown")
                .to_string(),
            branch: m
                .as_ref()
                .and_then(|m| m.str("branch"))
                .unwrap_or("")
                .to_string(),
            pane: m.as_ref().and_then(|m| m.str("pane")).map(String::from),
            workspace: m
                .as_ref()
                .and_then(|m| m.str("workspace"))
                .map(String::from),
            agent_name: m
                .as_ref()
                .and_then(|m| m.str("agent_name"))
                .map(String::from),
            started: m.as_ref().and_then(|m| m.str("started")).map(String::from),
            ended: m.as_ref().and_then(|m| m.str("ended")).map(String::from),
            status: m
                .as_ref()
                .and_then(|m| m.str("status"))
                .unwrap_or("running")
                .to_string(),
            usage: m.as_ref().and_then(|m| m.str("usage")).map(String::from),
            summary: body,
        });
    }
    out.sort_by_key(|a| a.n);
    Ok(out)
}

fn load_comments(dir: &Path) -> Result<Vec<Comment>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().map(|e| e == "md").unwrap_or(false))
        .collect();
    entries.sort();
    for path in entries {
        let src = fs::read_to_string(&path)?;
        let (fm, body) = frontmatter::split(&src);
        let m = fm.map(|fm| Frontmap::parse(&fm)).transpose()?;
        out.push(Comment {
            author: m
                .as_ref()
                .and_then(|m| m.str("author"))
                .unwrap_or("human")
                .to_string(),
            time: m.as_ref().and_then(|m| m.str("time")).map(String::from),
            file: m.as_ref().and_then(|m| m.str("file")).map(String::from),
            hunk: m.as_ref().and_then(|m| m.str("hunk")).map(String::from),
            body,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "herdr-worker-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn load_board_full_shape() {
        let root = temp_root("full");
        fs::write(
            root.join("board.md"),
            "---\ncolumns: [todo, doing, done]\nhidden: [todo]\nbranch_prefix: wk\ndefault_agent: opencode\n---\n",
        )
        .unwrap();
        let t = root.join("tasks").join("0042-fix-login");
        fs::create_dir_all(t.join("attempts")).unwrap();
        fs::create_dir_all(t.join("comments")).unwrap();
        fs::write(
            t.join("task.md"),
            "---\nid: 0042\ntitle: 修复：登录页\nstatus: doing\ntags: [bug, frontend]\nblocked-by: [39]\n---\n正文即 prompt。\n",
        )
        .unwrap();
        fs::write(
            t.join("attempts").join("1.md"),
            "---\nn: 1\nagent: opencode\nbranch: wk/0042-fix-login\nstatus: running\nusage: 12k tok\n---\n进行中。\n",
        )
        .unwrap();
        fs::write(
            t.join("comments").join("20260926-1010.md"),
            "---\nauthor: human\ntime: 09-26 10:10\nfile: src/login.rs\n---\n这里要判空。\n",
        )
        .unwrap();

        let board = load_board(&root).unwrap();
        assert_eq!(board.branch_prefix, "wk");
        assert_eq!(board.default_agent, "opencode");
        assert_eq!(board.visible_columns(), vec!["doing", "done"]);
        assert_eq!(board.tasks.len(), 1);
        let task = &board.tasks[0];
        assert_eq!(task.id, 42);
        assert_eq!(task.title, "修复：登录页");
        assert_eq!(task.tags, vec!["bug", "frontend"]);
        assert_eq!(task.blocked_by, vec![39]);
        assert_eq!(task.attempts.len(), 1);
        assert_eq!(task.attempts[0].agent, "opencode");
        assert_eq!(task.attempts[0].usage, Some("12k tok".to_string()));
        assert_eq!(task.comments.len(), 1);
        assert_eq!(task.comments[0].author, "human");
        assert_eq!(task.comments[0].file, Some("src/login.rs".to_string()));
        assert_eq!(board.tasks_in("doing").len(), 1);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn load_board_defaults_without_board_md() {
        let root = temp_root("defaults");
        let board = load_board(&root).unwrap();
        assert_eq!(board.visible_columns(), vec!["todo", "in-progress", "review", "done"]);
        assert!(board.tasks.is_empty());
        fs::remove_dir_all(&root).unwrap();
    }
}
