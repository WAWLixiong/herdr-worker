// TUI：经典看板（v1 工单 01——只读渲染 + 导航；布局裁决见 SPEC.md §7）

use anyhow::Result;
use ratatui::{
    backend::{CrosstermBackend, TestBackend},
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthChar;

use crate::model::Board;

#[derive(Clone, Copy, PartialEq)]
pub enum InputKind {
    Filter,
    NewTitle,
}

pub struct UiState {
    pub sel: usize,
    pub sel_col: usize,
    pub detail: bool,
    pub error: Option<String>,
    /// 已应用的过滤（标题子串 或 tag:xxx）
    pub filter: Option<String>,
    /// 输入模式（Some = 正在输入）
    pub input: Option<String>,
    pub input_kind: InputKind,
    /// 建卡后跳选：watcher 重载时消费
    pub select_next: Option<u32>,
    /// 大写 D 删除确认：Some(id) = 等待对 id 再按一次 D；任何其他键取消
    pub confirm_delete: Option<u32>,
}

fn trunc(s: &str, width: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > width.saturating_sub(1) {
            out.push('…');
            return out;
        }
        out.push(ch);
        w += cw;
    }
    out
}

fn attempt_badge(att: Option<&crate::model::Attempt>) -> Option<(&'static str, Color)> {
    let a = att?;
    match a.status.as_str() {
        "running" => {
            if a.agent_state.as_deref() == Some("idle") {
                Some(("⏳ 空闲待确认", Color::Yellow))
            } else {
                Some(("▶ running", Color::Cyan))
            }
        }
        "blocked" => Some(("⛔ blocked", Color::Red)),
        "done" => Some(("✓ done", Color::Green)),
        "failed" => Some(("✗ failed", Color::Magenta)),
        "cancelled" => Some(("– cancelled", Color::DarkGray)),
        _ => None,
    }
}

fn header_line(board: &Board, error: &Option<String>, filter: &Option<String>, input: &Option<String>, kind: &InputKind, confirm_delete: &Option<u32>) -> Line<'static> {
    let repo = board
        .root
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "board".to_string());
    let mut spans = vec![Span::styled(
        format!(" herdr-worker · {repo} "),
        Style::new().fg(Color::Black).bg(Color::Cyan).bold(),
    )];
    if let Some(f) = filter {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[过滤 {f} ✕(Esc)]"),
            Style::new().fg(Color::Yellow),
        ));
    }
    if let Some(inp) = input {
        match kind {
            InputKind::Filter => {
                spans.push(Span::raw("  /"));
                spans.push(Span::styled(inp.clone(), Style::new().fg(Color::White).bold()));
                spans.push(Span::styled("_", Style::new().fg(Color::DarkGray)));
            }
            InputKind::NewTitle => {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(" 新任务 ", Style::new().fg(Color::Black).bg(Color::Green).bold()));
                spans.push(Span::styled(inp.clone(), Style::new().fg(Color::White).bold()));
                spans.push(Span::styled("_", Style::new().fg(Color::DarkGray)));
                spans.push(Span::styled(" (#tag 收标签 · Enter 建卡 · Esc 取消)", Style::new().fg(Color::DarkGray)));
            }
        }
    }
    if let Some(e) = error {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!(" ⚠ {}", trunc(e, 80)),
            Style::new().fg(Color::White).bg(Color::Red).bold(),
        ));
    }
    if let Some(id) = confirm_delete {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!(" 再按 D 确认删除 {id:04}（其他键取消）"),
            Style::new().fg(Color::Black).bg(Color::Yellow).bold(),
        ));
    }
    Line::from(spans)
}

/// 过滤谓词：`tag:xxx` 匹配标签，其余为标题子串
fn matches_filter(t: &crate::model::Task, f: &Option<String>) -> bool {
    let Some(f) = f else { return true };
    if let Some(tag) = f.strip_prefix("tag:") {
        return t.tags.iter().any(|x| x.contains(tag));
    }
    t.title.contains(f.as_str())
}

/// 从一行输入提取 标题 + 内联 #标签：`修复登录页 #bug #frontend`
/// → ("修复登录页", ["bug", "frontend"])
fn extract_title_tags(input: &str) -> (String, Vec<String>) {
    let mut title_parts: Vec<&str> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    for tok in input.split_whitespace() {
        if let Some(t) = tok.strip_prefix('#') {
            if !t.is_empty() {
                tags.push(t.to_string());
                continue;
            }
        }
        title_parts.push(tok);
    }
    (title_parts.join(" "), tags)
}

fn help_line() -> Line<'static> {
    Line::from(Span::styled(
        " ←→/hjkl 列 · ↑↓/jk 卡 · n 新建 · e 编辑 · s 启动 · r 审过 · f 反馈 · d diff · x 停止 · c 清理worktree · D 删除卡 · / 过滤 · H/J/K/L 移卡 · Enter 详情 · q 退出",
        Style::new().fg(Color::DarkGray),
    ))
}

fn card_lines(board: &Board, task_idx: usize, width: usize) -> Vec<Line<'static>> {
    let task = &board.tasks[task_idx];
    let title_style = Style::new().fg(Color::White).bold();
    let mut lines = vec![Line::from(vec![
        Span::raw(" "),
        Span::styled(format!("{:04}", task.id), Style::new().fg(Color::Gray)),
        Span::raw(" "),
        Span::styled(trunc(&task.title, width.saturating_sub(7)), title_style),
    ])];

    let att = task.latest_attempt();
    let mut meta = att.map(|a| a.agent.clone()).unwrap_or_default();
    if let Some((badge, _)) = attempt_badge(att) {
        meta.push(' ');
        meta.push_str(badge);
    }
    if task.attempts.len() > 1 {
        meta.push_str(&format!(" #{}", task.attempts.len()));
    }
    let meta_color = att
        .and_then(|a| attempt_badge(Some(a)))
        .map(|(_, c)| c)
        .unwrap_or(Color::DarkGray);
    lines.push(Line::from(Span::styled(
        if meta.is_empty() { " ".to_string() } else { format!(" {meta}") },
        Style::new().fg(meta_color),
    )));

    let tags = task
        .tags
        .iter()
        .map(|t| format!("#{t}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut tail = vec![Span::styled(
        format!(" {}", trunc(&tags, width.saturating_sub(10))),
        Style::new().fg(Color::DarkGray),
    )];
    if let Some(u) = att.and_then(|a| a.usage.as_deref()) {
        tail.push(Span::styled(format!(" {u}"), Style::new().fg(Color::Yellow)));
    }
    lines.push(Line::from(tail));
    lines
}

fn detail_text(board: &Board, task_idx: usize) -> Text<'static> {
    let t = &board.tasks[task_idx];
    let mut lines: Vec<Line> = vec![];
    let att = t.latest_attempt();
    let mut meta = format!(
        "列 {} · agent {} · attempt {}/{}",
        t.status,
        att.map(|a| a.agent.as_str()).unwrap_or("—"),
        att.map(|a| a.n).unwrap_or(0),
        t.attempts.len(),
    );
    if let Some(p) = &t.priority {
        meta.push_str(&format!(" · 优先级 {p}"));
    }
    lines.push(Line::from(Span::styled(meta, Style::new().fg(Color::Cyan))));
    let mut meta2 = format!(
        "标签 {}",
        t.tags.iter().map(|s| format!("#{s}")).collect::<Vec<_>>().join(" ")
    );
    if !t.blocked_by.is_empty() {
        meta2.push_str(&format!(
            " · 被阻塞于 {}",
            t.blocked_by
                .iter()
                .map(|i| format!("{i:04}"))
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if let Some(u) = att.and_then(|a| a.usage.as_deref()) {
        meta2.push_str(&format!(" · 用量 {u}"));
    }
    lines.push(Line::from(Span::styled(meta2, Style::new().fg(Color::DarkGray))));
    lines.push(Line::from(""));

    for l in t.body.lines() {
        lines.push(Line::from(Span::raw(l.to_string())));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "attempts",
        Style::new().fg(Color::White).bold().underlined(),
    )));
    if t.attempts.is_empty() {
        lines.push(Line::from(Span::styled("  （尚未启动）", Style::new().fg(Color::DarkGray))));
    }
    for a in &t.attempts {
        let color = attempt_badge(Some(a)).map(|(_, c)| c).unwrap_or(Color::Gray);
        let badge = attempt_badge(Some(a)).map(|(b, _)| b).unwrap_or(&a.status);
        let usage = a.usage.as_deref().map(|u| format!("  ⚡{u}")).unwrap_or_default();
        lines.push(Line::from(Span::styled(
            format!("  #{} {} {}  {}{}", a.n, a.agent, a.branch, badge, usage),
            Style::new().fg(color),
        )));
        if !a.summary.trim().is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    {}", trunc(a.summary.trim(), 96)),
                Style::new().fg(Color::DarkGray),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "comments",
        Style::new().fg(Color::White).bold().underlined(),
    )));
    if t.comments.is_empty() {
        lines.push(Line::from(Span::styled("  （暂无）", Style::new().fg(Color::DarkGray))));
    }
    for c in &t.comments {
        let mut spans = vec![
            Span::styled(
                format!(" {} ", c.author),
                Style::new().fg(if c.author == "human" { Color::Green } else { Color::Blue }),
            ),
            Span::styled(c.time.clone().unwrap_or_default(), Style::new().fg(Color::DarkGray)),
        ];
        if let Some(f) = &c.file {
            spans.push(Span::styled(format!(" @{}", f), Style::new().fg(Color::Yellow)));
        }
        if c.delivered {
            spans.push(Span::styled(" ✓已投递", Style::new().fg(Color::DarkGray)));
        }
        lines.push(Line::from(spans));
        lines.push(Line::from(Span::raw(format!("  {}", c.body.trim_end()))));
    }
    Text::from(lines)
}

pub fn render(f: &mut Frame, board: &Board, st: &UiState) {
    let area = f.area();
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);
    f.render_widget(Paragraph::new(header_line(board, &st.error, &st.filter, &st.input, &st.input_kind, &st.confirm_delete)), rows[0]);

    let columns = board.visible_columns();
    let col_areas = Layout::horizontal(vec![Constraint::Fill(1); columns.len().max(1)])
        .split(rows[1]);
    let selected_id = board.tasks.get(st.sel).map(|t| t.id);

    for (i, col) in columns.iter().enumerate() {
        let tasks: Vec<&crate::model::Task> = board
            .tasks_in(col)
            .into_iter()
            .filter(|t| matches_filter(t, &st.filter))
            .collect();
        let active = i == st.sel_col;
        let block = Block::bordered()
            .title(format!(" {col} ({}) ", tasks.len()))
            .border_style(if active {
                Style::new().fg(Color::Cyan)
            } else {
                Style::new().fg(Color::DarkGray)
            });
        let width = col_areas.get(i).map(|a| a.width.saturating_sub(4) as usize).unwrap_or(16);
        let items: Vec<ListItem> = tasks
            .iter()
            .map(|t| {
                let idx = board.tasks.iter().position(|x| x.id == t.id).unwrap_or(0);
                let mut lines = card_lines(board, idx, width);
                if Some(t.id) == selected_id {
                    // 选中卡：标题行反白
                    if let Some(l) = lines.first_mut() {
                        for span in l.spans.iter_mut() {
                            span.style = Style::new().fg(Color::Black).bg(Color::Cyan).bold();
                        }
                    }
                }
                ListItem::new(lines)
            })
            .collect();
        let list_area = col_areas.get(i).copied().unwrap_or(Rect::ZERO);
        f.render_widget(List::new(items).block(block), list_area);
    }

    f.render_widget(Paragraph::new(help_line()), rows[2]);

    if st.detail {
        if let Some(t) = board.tasks.get(st.sel) {
            let w = area.width.saturating_sub(8);
            let h = area.height.saturating_sub(4);
            let popup = Rect { x: 4, y: 2, width: w, height: h };
            f.render_widget(Clear, popup);
            let block = Block::bordered()
                .title(format!(" {:04} · {} ", t.id, t.title))
                .border_style(Style::new().fg(Color::Cyan));
            f.render_widget(
                Paragraph::new(detail_text(board, st.sel))
                    .block(block)
                    .wrap(Wrap { trim: false }),
                popup,
            );
        }
    }
}

/// 离屏渲染一帧到文件（开发/测试用）
pub fn snapshot(board: &Board, st: &UiState, path: &std::path::Path, w: u16, h: u16) -> Result<()> {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).map_err(|e| anyhow::anyhow!("{e}"))?;
    terminal
        .draw(|f| render(f, board, st))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let buf = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..h {
        let mut prev_wide = false;
        for x in 0..w {
            if prev_wide {
                prev_wide = false;
                continue;
            }
            if let Some(cell) = buf.cell(ratatui::layout::Position::new(x, y)) {
                let sym = cell.symbol();
                if !sym.is_empty() {
                    out.push_str(sym);
                    prev_wide = sym
                        .chars()
                        .next()
                        .map(|c| c.width().unwrap_or(1) > 1)
                        .unwrap_or(false);
                }
            }
        }
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}

/// 通用动作执行器：对选中任务执行 op，失败红条显示在头部 + 完整错误链落盘（调试用）
fn run_action(
    board: &Board,
    st: &mut UiState,
    op: impl FnOnce(&std::path::Path, u32) -> Result<()>,
) {
    if let Some(id) = board.tasks.get(st.sel).map(|t| t.id) {
        let root = board.root.clone();
        match op(&root, id) {
            Ok(_) => st.error = None,
            Err(e) => {
                st.error = Some(e.to_string());
                let _ = std::fs::write(
                    std::env::temp_dir().join("herdr-worker-debug.log"),
                    format!(
                        "[{}] task {id:04} action error: {e:?}\n",
                        crate::write::utc_now_rfc3339()
                    ),
                );
            }
        }
    }
}

/// H/L：把选中卡移到相邻列（status 变更，order 清除）
fn move_to_col(board: &Board, st: &mut UiState, delta: i32) {
    let cols = board.visible_columns();
    let Some(task) = board.tasks.get(st.sel) else { return };
    let target = st.sel_col as i32 + delta;
    if target < 0 || target as usize >= cols.len() {
        return;
    }
    let target = target as usize;
    let Some(path) = board.task_path(task.id) else { return };
    let col_name = cols[target].clone();
    let root = board.root.clone();
    let _ = crate::lock::with_lock(&root, || {
        crate::write::set_field(&path, "status", &col_name)?;
        crate::write::remove_field(&path, "order")?;
        crate::write::set_field(&path, "updated", &crate::write::utc_now_rfc3339())
    });
    st.sel_col = target;
}

/// J/K：列内排序，与相邻卡交换（整列 order 归一化为 0..n）
fn reorder(board: &Board, st: &mut UiState, delta: i32) {
    let cols = board.visible_columns();
    let Some(col) = cols.get(st.sel_col) else { return };
    let Some(task) = board.tasks.get(st.sel) else { return };
    let tasks = board.tasks_in(col);
    let Some(pos) = tasks.iter().position(|t| t.id == task.id) else { return };
    let target = pos as i32 + delta;
    if target < 0 || target as usize >= tasks.len() {
        return;
    }
    let mut seq: Vec<u32> = tasks.iter().map(|t| t.id).collect();
    seq.swap(pos, target as usize);
    let root = board.root.clone();
    let writes: Vec<(std::path::PathBuf, usize)> = seq
        .iter()
        .enumerate()
        .filter_map(|(i, id)| board.task_path(*id).map(|p| (p, i)))
        .collect();
    let _ = crate::lock::with_lock(&root, || {
        for (p, i) in &writes {
            crate::write::set_field(p, "order", &i.to_string())?;
        }
        Ok(())
    });
}

pub fn run(mut board: Board) -> Result<()> {
    // 登记 pane id（open-board 动作据此去重：已开则聚焦）
    if let (Ok(pane_id), Some(state)) =
        (std::env::var("HERDR_PANE_ID"), crate::model::board_state_file(&board.root))
    {
        let _ = std::fs::write(state, pane_id);
    }
    let mut st = UiState { sel: 0, sel_col: 0, detail: false, error: None, filter: None, input: None, input_kind: InputKind::Filter, select_next: None, confirm_delete: None };
    // 初始选中：第一张可见卡
    if !board.tasks.is_empty() {
        let cols = board.visible_columns();
        for (i, c) in cols.iter().enumerate() {
            if let Some(t) = board.tasks_in(c).first() {
                st.sel = board.tasks.iter().position(|x| x.id == t.id).unwrap_or(0);
                st.sel_col = i;
                break;
            }
        }
    }
    let mut last_snap = crate::model::fs_snapshot(&board.root);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    loop {
        terminal.draw(|f| render(f, &board, &st))?;
        if event::poll(std::time::Duration::from_millis(400))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                // 输入模式：吃掉所有按键（过滤 / 新建标题）
                if let Some(inp) = st.input.as_mut() {
                    match key.code {
                        KeyCode::Enter => match st.input_kind {
                            InputKind::Filter => {
                                st.filter = if inp.is_empty() { None } else { Some(inp.clone()) };
                                st.input = None;
                            }
                            InputKind::NewTitle => {
                                let (title, tags) = extract_title_tags(inp);
                                st.input = None;
                                if !title.is_empty() {
                                    let root = board.root.clone();
                                    match crate::act::create_task_core(&root, &title, &tags, "todo", "") {
                                        Ok(id) => {
                                            st.error = None;
                                            st.select_next = Some(id);
                                        }
                                        Err(e) => st.error = Some(e.to_string()),
                                    }
                                }
                            }
                        },
                        KeyCode::Esc => {
                            if st.input_kind == InputKind::Filter {
                                st.filter = None;
                            }
                            st.input = None;
                        }
                        KeyCode::Backspace => {
                            inp.pop();
                        }
                        KeyCode::Char(c) => inp.push(c),
                        _ => {}
                    }
                    continue;
                }
                let columns = board.visible_columns();
                let move_col = |st: &mut UiState, delta: i32| {
                    if columns.is_empty() {
                        return;
                    }
                    st.sel_col = (st.sel_col as i32 + delta)
                        .clamp(0, columns.len() as i32 - 1)
                        as usize;
                    if let Some(t) = board.tasks_in(&columns[st.sel_col]).first() {
                        st.sel = board.tasks.iter().position(|x| x.id == t.id).unwrap_or(0);
                    }
                };
                match key.code {
                    KeyCode::Char('q') => break,
                KeyCode::Esc => {
                    // Esc 不退出（overlay 场景下启动期可能收到类 Esc 事件）：
                    // 只负责关详情浮层 / 清过滤；q 是唯一退出键
                    if st.detail {
                        st.detail = false;
                    } else if st.filter.is_some() {
                        st.filter = None;
                    }
                }
                    KeyCode::Enter => {
                        if !board.tasks.is_empty() {
                            st.detail = !st.detail;
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') => move_col(&mut st, -1),
                    KeyCode::Right | KeyCode::Char('l') => move_col(&mut st, 1),
                    KeyCode::Char('H') => move_to_col(&board, &mut st, -1),
                    KeyCode::Char('L') => move_to_col(&board, &mut st, 1),
                    KeyCode::Char('J') => reorder(&board, &mut st, 1),
                    KeyCode::Char('K') => reorder(&board, &mut st, -1),
                    KeyCode::Char('s') => run_action(&board, &mut st, |r, id| {
                        crate::orchestrate::start_attempt(r, id, None).map(|_| ())
                    }),
                    KeyCode::Char('r') => run_action(&board, &mut st, crate::orchestrate::confirm_review),
                    KeyCode::Char('f') => run_action(&board, &mut st, |r, id| {
                        crate::orchestrate::follow_up(r, id).map(|_| ())
                    }),
                    KeyCode::Char('d') => run_action(&board, &mut st, crate::orchestrate::open_diff),
                    KeyCode::Char('x') => run_action(&board, &mut st, crate::orchestrate::stop_attempt),
                    KeyCode::Char('c') => run_action(&board, &mut st, crate::orchestrate::clean_worktree),
                    KeyCode::Char('e') => run_action(&board, &mut st, crate::orchestrate::open_editor),
                    KeyCode::Char('/') => {
                        st.input = Some(String::new());
                        st.input_kind = InputKind::Filter;
                    }
                    KeyCode::Char('n') => {
                        st.input = Some(String::new());
                        st.input_kind = InputKind::NewTitle;
                    }
                    KeyCode::Char('D') => {
                        if let Some(id) = board.tasks.get(st.sel).map(|t| t.id) {
                            st.confirm_delete = Some(id);
                        }
                    }
                KeyCode::Up | KeyCode::Char('k') => {
                    if st.sel_col < columns.len() {
                        let idxs: Vec<usize> = board
                            .tasks_in(&columns[st.sel_col])
                            .iter()
                            .map(|t| board.tasks.iter().position(|x| x.id == t.id).unwrap_or(0))
                            .collect();
                        if let Some(pos) = idxs.iter().position(|i| *i == st.sel) {
                            if pos > 0 {
                                st.sel = idxs[pos - 1];
                            }
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if st.sel_col < columns.len() {
                        let idxs: Vec<usize> = board
                            .tasks_in(&columns[st.sel_col])
                            .iter()
                            .map(|t| board.tasks.iter().position(|x| x.id == t.id).unwrap_or(0))
                            .collect();
                        if let Some(pos) = idxs.iter().position(|i| *i == st.sel) {
                            if pos + 1 < idxs.len() {
                                st.sel = idxs[pos + 1];
                            }
                        }
                    }
                }
                _ => {}
                }
            }
        }
        // 轮询监听：.herdr-worker/ 变化 → 重载（选中和列保持稳定；建卡跳选优先）
        let snap = crate::model::fs_snapshot(&board.root);
        if snap != last_snap {
            last_snap = snap;
            let sel_id = board.tasks.get(st.sel).map(|t| t.id);
            if let Ok(nb) = crate::model::load_board(&board.root) {
                board = nb;
                let jump = st.select_next.take();
                let target = jump.or(sel_id);
                st.sel = target
                    .and_then(|id| board.tasks.iter().position(|t| t.id == id))
                    .unwrap_or_else(|| st.sel.min(board.tasks.len().saturating_sub(1)));
                if let Some(t) = board.tasks.get(st.sel) {
                    if let Some(c) = board
                        .visible_columns()
                        .iter()
                        .position(|c| *c == t.status)
                    {
                        st.sel_col = c;
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::load_board;
    use std::fs;

    #[test]
    fn title_tag_extraction() {
        assert_eq!(
            extract_title_tags("修复登录页 #bug #frontend"),
            ("修复登录页".to_string(), vec!["bug".to_string(), "frontend".to_string()])
        );
        assert_eq!(
            extract_title_tags("纯标题无标签"),
            ("纯标题无标签".to_string(), vec![])
        );
        // 只有 # 开头的词才是标签；词中含 # 不算（如 C#）
        assert_eq!(
            extract_title_tags("C# 语法高亮 #parser"),
            ("C# 语法高亮".to_string(), vec!["parser".to_string()])
        );
        // 孤立 # 留在标题
        assert_eq!(extract_title_tags("# 标题"), ("# 标题".to_string(), vec![]));
        assert_eq!(extract_title_tags(""), (String::new(), vec![]));
    }

    #[test]
    fn filter_predicate() {
        use crate::model::Task;
        let mk = |title: &str, tags: Vec<&str>| Task {
            id: 1,
            dir_name: "0001-x".into(),
            title: title.into(),
            status: "todo".into(),
            priority: None,
            tags: tags.into_iter().map(String::from).collect(),
            parent: None,
            blocked_by: vec![],
            order: None,
            created: None,
            updated: None,
            body: String::new(),
            attempts: vec![],
            comments: vec![],
        };
        let t = mk("修复登录页", vec!["bug", "frontend"]);
        assert!(matches_filter(&t, &None));
        assert!(matches_filter(&t, &Some("登录".into())));
        assert!(!matches_filter(&t, &Some("导出".into())));
        assert!(matches_filter(&t, &Some("tag:bug".into())));
        assert!(!matches_filter(&t, &Some("tag:api".into())));
    }

    #[test]
    fn render_smoke_contains_known_content() {
        let root = std::env::temp_dir().join(format!("herdr-worker-tui-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let t = root.join("tasks").join("0007-fix-login");
        fs::create_dir_all(t.join("attempts")).unwrap();
        fs::write(
            t.join("task.md"),
            "---\ntitle: 修复登录页偶发白屏\nstatus: in-progress\ntags: [bug]\n---\n正文即 prompt。\n",
        )
        .unwrap();
        fs::write(
            t.join("attempts").join("1.md"),
            "---\nn: 1\nagent: opencode\nbranch: hw/0007-fix-login\nstatus: blocked\nusage: 5k tok\n---\n等确认主题配置。\n",
        )
        .unwrap();
        let board = load_board(&root).unwrap();

        let backend = TestBackend::new(160, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let st = UiState { sel: 0, sel_col: 1, detail: false, error: None, filter: None, input: None, input_kind: InputKind::Filter, select_next: None, confirm_delete: None };
        terminal.draw(|f| render(f, &board, &st)).unwrap();
        let text = buffer_text(terminal.backend().buffer());
        assert!(text.contains("修复登录页偶发白屏"), "看板帧应含任务标题");
        assert!(text.contains("in-progress"), "看板帧应含列名");
        assert!(text.contains("⛔ blocked"), "看板帧应含 blocked 徽章");
        assert!(text.contains("5k tok"), "看板帧应含用量");

        let st = UiState { sel: 0, sel_col: 1, detail: true, error: None, filter: None, input: None, input_kind: InputKind::Filter, select_next: None, confirm_delete: None };
        terminal.draw(|f| render(f, &board, &st)).unwrap();
        let text = buffer_text(terminal.backend().buffer());
        assert!(text.contains("attempts"), "详情帧应含 attempts 区");
        assert!(text.contains("comments"), "详情帧应含 comments 区");
        assert!(text.contains("正文即 prompt"), "详情帧应含正文");

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn move_to_col_and_reorder_write_back() {
        let root = std::env::temp_dir().join(format!("herdr-worker-move-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (name, title) in [("0001-alpha", "甲"), ("0002-beta", "乙")] {
            let t = root.join("tasks").join(name);
            fs::create_dir_all(&t).unwrap();
            fs::write(
                t.join("task.md"),
                format!("---\nid: {}\ntitle: {}\nstatus: todo\norder: 0\n---\n正文\n", &name[..4], title),
            )
            .unwrap();
        }
        let board = load_board(&root).unwrap();
        // 选中 0001（order 0 排前）
        let sel = board.tasks.iter().position(|t| t.id == 1).unwrap();
        let mut st = UiState { sel, sel_col: 0, detail: false, error: None, filter: None, input: None, input_kind: InputKind::Filter, select_next: None, confirm_delete: None };

        // J：列内交换，0001 与 0002 换位，整列 order 归一化
        reorder(&board, &mut st, 1);
        let a = fs::read_to_string(root.join("tasks").join("0001-alpha").join("task.md")).unwrap();
        let b = fs::read_to_string(root.join("tasks").join("0002-beta").join("task.md")).unwrap();
        assert!(a.contains("order: 1"), "0001 应排后：{a}");
        assert!(b.contains("order: 0"), "0002 应排前：{b}");

        // L：todo → in-progress（重载后 0001 仍在 todo，order=1 排第二）
        let board = load_board(&root).unwrap();
        let sel = board.tasks.iter().position(|t| t.id == 1).unwrap();
        let mut st = UiState { sel, sel_col: 0, detail: false, error: None, filter: None, input: None, input_kind: InputKind::Filter, select_next: None, confirm_delete: None };
        move_to_col(&board, &mut st, 1);
        let src = fs::read_to_string(root.join("tasks").join("0001-alpha").join("task.md")).unwrap();
        assert!(src.contains("status: in-progress"), "应已移入 in-progress：{src}");
        assert!(!src.contains("order:"), "移列后 order 应被清除：{src}");
        assert!(src.contains("id: 0001"), "id 保持零填充：{src}");

        fs::remove_dir_all(&root).unwrap();
    }

    fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
        let mut out = String::new();
        let area = buf.area;
        for y in 0..area.height {
            let mut prev_wide = false;
            for x in 0..area.width {
                if prev_wide {
                    prev_wide = false;
                    continue;
                }
                if let Some(cell) = buf.cell(ratatui::layout::Position::new(x, y)) {
                    let sym = cell.symbol();
                    if !sym.is_empty() {
                        out.push_str(sym);
                        prev_wide = sym.chars().next().map(|c| c.width().unwrap_or(1) > 1).unwrap_or(false);
                    }
                }
            }
            out.push('\n');
        }
        out
    }
}
