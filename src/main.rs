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
            // d 键的 overlay diff：HW_DIFF_CMD 由 plugin pane open --env 注入
            let cmd = std::env::var("HW_DIFF_CMD").map_err(|_| {
                anyhow::anyhow!("diff 子命令需要 HW_DIFF_CMD 环境变量（由板上按 d 注入）")
            })?;
            let shell: (&str, &str) = if cfg!(windows) {
                ("cmd", "/c")
            } else {
                ("sh", "-c")
            };
            let status = std::process::Command::new(shell.0)
                .args([shell.1, &cmd])
                .status()?;
            std::process::exit(status.code().unwrap_or(0));
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
