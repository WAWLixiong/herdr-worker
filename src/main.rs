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
