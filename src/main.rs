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
        Some("editor") => {
            // e 键的 overlay 编辑器：目标文件由 plugin pane open --env 注入。
            // 直接用 argv 启动编辑器进程（不经 shell）——EDITOR 路径含空格不再有引号问题。
            let file = std::env::var("HW_EDITOR_FILE").map_err(|_| {
                anyhow::anyhow!("editor 子命令需要 HW_EDITOR_FILE 环境变量（由板上按 e 注入）")
            })?;
            let editor = std::env::var("EDITOR")
                .ok()
                .filter(|e| !e.is_empty())
                .unwrap_or_else(|| {
                    if cfg!(windows) {
                        "notepad".to_string()
                    } else {
                        "vi".to_string()
                    }
                });
            // EDITOR 整串是一个存在的路径 → 整体当程序（覆盖含空格路径）；
            // 否则按空白切分（支持 "code -w" 这类带参数写法）
            let argv: Vec<String> = if std::path::Path::new(&editor).exists() {
                vec![editor]
            } else {
                editor.split_whitespace().map(String::from).collect()
            };
            let status = std::process::Command::new(&argv[0])
                .args(&argv[1..])
                .arg(&file)
                .status()?;
            std::process::exit(status.code().unwrap_or(0));
        }
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

            // diff 工具配置：HW_BOARD_ROOT（open_diff 注入）→ board.md 的 diff_tool；
            // auto = 探测 delta→difft→纯文本；none = 纯文本；其他值 = 指定命令名（stdin 喂 diff）
            let board_root = std::env::var("HW_BOARD_ROOT")
                .ok()
                .map(PathBuf::from)
                .or_else(|| std::env::current_dir().ok().and_then(|d| model::find_board(&d)));
            let diff_tool = board_root
                .as_deref()
                .and_then(|r| model::load_board(r).ok())
                .map(|b| b.diff_tool)
                .unwrap_or_else(|| "auto".to_string());
            log(&format!("diff_tool={diff_tool}"));

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
            let has_content = !diff_text.is_empty() || !untracked_text.is_empty();

            if !diff_text.is_empty() {
                let tool: Option<String> = match diff_tool.as_str() {
                    "none" | "" => None,
                    "auto" => ["delta", "difft"]
                        .iter()
                        .find(|b| std::process::Command::new(*b).arg("--version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map(|mut c| c.wait().map(|s| s.success()).unwrap_or(false)).unwrap_or(false))
                        .map(|s| s.to_string()),
                    named => Some(named.to_string()),
                };
                let rendered = tool.and_then(|t| {
                    let mut child = std::process::Command::new(&t)
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
            if !has_content && git_err.is_empty() {
                println!("（该 worktree 相对 HEAD 无改动）");
            }
            // 有真内容：无限等待按键；空结果：5 秒自动关闭
            if has_content {
                print!("\n按任意键关闭…");
            } else {
                print!("\n按任意键关闭（5 秒后自动关闭）…");
            }
            let _ = std::io::stdout().flush();
            log(&format!(
                "diff={}B untracked={}B err={}B, holding ({})",
                diff_text.len(),
                untracked_text.len(),
                git_err.len(),
                if has_content { "forever" } else { "5s" }
            ));
            wait_real_keypress(if has_content {
                None
            } else {
                Some(std::time::Duration::from_secs(5))
            });
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

/// 等一个真实按键。timeout=None 时无限等待。
/// overlay pane 的 spawn 会注入类 Esc 噪声事件——前 400ms 到达的事件一律吞掉。
fn wait_real_keypress(timeout: Option<std::time::Duration>) {
    let _ = crossterm::terminal::enable_raw_mode();
    let start = std::time::Instant::now();
    loop {
        if let Some(t) = timeout {
            if start.elapsed() > t {
                break;
            }
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
