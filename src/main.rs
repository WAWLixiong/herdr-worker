use std::path::PathBuf;

mod act;
mod frontmatter;
mod lock;
mod model;
mod orchestrate;
mod sync;
mod tui;
mod write;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("board") => {
            let start = flag_value(&args, "--path")
                .map(PathBuf::from)
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?;
            let root = model::find_board(&start).ok_or_else(|| {
                anyhow::anyhow!("未找到 .herdr-worker/（从 {} 逐级向上）", start.display())
            })?;
            let board = model::load_board(&root)?;
            if let Some(snap) = flag_value(&args, "--snap") {
                let sel_col = board
                    .visible_columns()
                    .iter()
                    .position(|c| !board.tasks_in(c).is_empty())
                    .unwrap_or(0);
                let st = tui::UiState { sel: 0, sel_col, detail: false, error: None, filter: None, input: None, input_kind: tui::InputKind::Filter, select_next: None };
                return tui::snapshot(&board, &st, std::path::Path::new(&snap), 150, 36);
            }
            tui::run(board)
        }
        Some("act") => act::run(&args[1..]),
        Some("sync") => sync::run(),
        Some("diff") => {
            // d 键的 overlay diff：HW_DIFF_DIR 由 plugin pane open --env 注入。
            // 不依赖 git pager（less 在裸 Windows 上常缺失——曾表现为
            // "'less' 不是内部或外部命令"）：捕获输出直接打印 + 时间窗按键驻留；
            // delta/difft 存在时先经渲染链上色。
            use std::io::Write;
            let log = |msg: &str| {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(std::env::temp_dir().join("herdr-worker-debug.log"))
                    .and_then(|mut f| {
                        writeln!(f, "[{}] diff: {msg}", crate::write::utc_now_rfc3339())
                    });
            };
            let dir = std::env::var("HW_DIFF_DIR").map_err(|_| {
                anyhow::anyhow!("diff 子命令需要 HW_DIFF_DIR 环境变量（由板上按 d 注入）")
            })?;
            log(&format!("start dir={dir:?}"));

            let run_git = |args: &[&str]| -> std::io::Result<std::process::Output> {
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(&dir)
                    .args(args)
                    .output()
            };
            let diff = run_git(&["--no-pager", "diff", "HEAD"])?;
            let untracked = run_git(&["ls-files", "--others", "--exclude-standard"])?;
            let diff_text = String::from_utf8_lossy(&diff.stdout).trim().to_string();
            let untracked_text = String::from_utf8_lossy(&untracked.stdout).trim().to_string();
            let git_err = String::from_utf8_lossy(&diff.stderr).trim().to_string();

            if !diff_text.is_empty() {
                // 渲染器探测链：delta → difft → 裸文本
                let rendered = ["delta", "difft"].iter().find_map(|b| {
                    let mut child = std::process::Command::new(b)
                        .args(["--paging", "never"])
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                        .ok()?;
                    child
                        .stdin
                        .as_mut()
                        .and_then(|s| s.write_all(diff_text.as_bytes()).ok())
                        .map(|_| child)
                });
                match rendered {
                    Some(mut child) => {
                        let _ = child.wait();
                    }
                    None => println!("{diff_text}"),
                }
            } else if !git_err.is_empty() {
                // 例如 unborn 分支：fatal: ambiguous argument 'HEAD'
                println!("（git：{git_err}）");
            }
            if !untracked_text.is_empty() {
                println!("\n未跟踪文件：\n{untracked_text}");
            }
            if diff_text.is_empty() && untracked_text.is_empty() && git_err.is_empty() {
                println!("（该 worktree 相对 HEAD 无改动）");
            }
            print!("\n按任意键关闭（5 秒后自动关闭）…");
            let _ = std::io::stdout().flush();
            log(&format!(
                "diff={}B untracked={}B err={}B, holding",
                diff_text.len(),
                untracked_text.len(),
                git_err.len()
            ));
            wait_real_keypress();
            log("closing");
            std::process::exit(0);
        }
        _ => {
            eprintln!("用法：herdr-worker board [--path DIR] | act <action> [--flags]");
            std::process::exit(2);
        }
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == flag {
            return it.next().cloned();
        }
    }
    None
}

/// 等一个真实按键（最多 5 秒自动返回）。
/// overlay pane 的 spawn 会注入类 Esc 噪声事件——前 400ms 到达的事件一律吞掉。
fn wait_real_keypress() {
    let _ = crossterm::terminal::enable_raw_mode();
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > std::time::Duration::from_secs(5) {
            break;
        }
        match crossterm::event::poll(std::time::Duration::from_millis(100)) {
            Ok(true) => {
                let _ = crossterm::event::read();
                // 400ms 后到达的才算用户按键
                if start.elapsed() > std::time::Duration::from_millis(400) {
                    break;
                }
            }
            Ok(false) => {}
            Err(_) => break,
        }
    }
    let _ = crossterm::terminal::disable_raw_mode();
}
