// PROTOTYPE — 已完成使命（wayfinder 票 07 定稿，2026-09-26）：
// 裁决 = A 经典看板为主看板 + 详情浮层沿用 C 的主从排版；B 泳道列表为后续可选视图。
// 一次性代码留档供 spec 撰写时参考，勿用于生产。Tab 循环切换变体；`--snap` 离屏渲染静态帧。

use ratatui::{
    backend::{CrosstermBackend, TestBackend},
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, PartialEq)]
enum Column {
    Todo,
    InProgress,
    Review,
    Done,
}
impl Column {
    fn label(&self) -> &'static str {
        match self {
            Column::Todo => "todo",
            Column::InProgress => "in-progress",
            Column::Review => "review",
            Column::Done => "done",
        }
    }
    fn order() -> [Column; 4] {
        [Column::Todo, Column::InProgress, Column::Review, Column::Done]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum AttStatus {
    Running,
    Blocked,
    IdleAwait,
    Done,
    Failed,
}
impl AttStatus {
    fn badge(&self) -> &'static str {
        match self {
            AttStatus::Running => "▶ running",
            AttStatus::Blocked => "⛔ blocked",
            AttStatus::IdleAwait => "⏳ 空闲待确认",
            AttStatus::Done => "✓ done",
            AttStatus::Failed => "✗ failed",
        }
    }
    fn color(&self) -> Color {
        match self {
            AttStatus::Running => Color::Cyan,
            AttStatus::Blocked => Color::Red,
            AttStatus::IdleAwait => Color::Yellow,
            AttStatus::Done => Color::Green,
            AttStatus::Failed => Color::Magenta,
        }
    }
    fn dot(&self) -> &'static str {
        match self {
            AttStatus::Running => "●",
            AttStatus::Blocked => "⛔",
            AttStatus::IdleAwait => "◔",
            AttStatus::Done => "✓",
            AttStatus::Failed => "✗",
        }
    }
}

struct Comment {
    author: &'static str,
    time: &'static str,
    body: &'static str,
}

struct Task {
    id: u32,
    title: &'static str,
    column: Column,
    agent: Option<&'static str>,
    attempt: Option<u32>,
    attempt_status: Option<AttStatus>,
    attempts: u32,
    tags: &'static [&'static str],
    usage: Option<&'static str>,
    priority: Option<&'static str>,
    body: &'static str,
    comments: &'static [Comment],
}

fn fake_tasks() -> Vec<Task> {
    vec![
        Task {
            id: 41,
            title: "更新依赖并修 CI",
            column: Column::Todo,
            agent: None,
            attempt: None,
            attempt_status: None,
            attempts: 0,
            tags: &["ci", "chore"],
            usage: None,
            priority: Some("urgent"),
            body: "升级到最新 minor，修复 CI 上偶发的 flaky 测试。",
            comments: &[],
        },
        Task {
            id: 40,
            title: "给导出接口加分页参数",
            column: Column::Todo,
            agent: None,
            attempt: None,
            attempt_status: None,
            attempts: 0,
            tags: &["api"],
            usage: None,
            priority: None,
            body: "export 端点加 page/page_size，默认 100。",
            comments: &[],
        },
        Task {
            id: 39,
            title: "修复设置页偶发白屏",
            column: Column::Todo,
            agent: None,
            attempt: None,
            attempt_status: None,
            attempts: 0,
            tags: &["bug", "frontend"],
            usage: None,
            priority: Some("high"),
            body: "切 tab 过快时设置页白屏，疑似渲染竞态。",
            comments: &[],
        },
        Task {
            id: 31,
            title: "重构认证模块",
            column: Column::InProgress,
            agent: Some("claude-code"),
            attempt: Some(2),
            attempt_status: Some(AttStatus::Running),
            attempts: 2,
            tags: &["refactor"],
            usage: None,
            priority: None,
            body: "把 auth 拆成独立 crate，对外接口保持不变。完成后跑全量测试。\n\n注意：旧导出别名保留一个版本。",
            comments: &[
                Comment { author: "human", time: "09-26 10:02", body: "旧别名要保留，很多下游还在用。" },
                Comment { author: "claude-code", time: "09-26 13:40", body: "已拆出 auth-core，旧别名 re-export 完成，正在补测试。" },
            ],
        },
        Task {
            id: 42,
            title: "抽取主题色到配置",
            column: Column::InProgress,
            agent: Some("opencode"),
            attempt: Some(1),
            attempt_status: Some(AttStatus::Blocked),
            attempts: 1,
            tags: &["ui", "config"],
            usage: None,
            priority: None,
            body: "硬编码的主题色抽到 theme.toml，保留默认值。",
            comments: &[Comment { author: "opencode", time: "09-26 14:11", body: "theme.toml 里要不要带暗色/亮色两套？等你确认。" }],
        },
        Task {
            id: 43,
            title: "CLI 加 --json 输出",
            column: Column::Review,
            agent: Some("claude-code"),
            attempt: Some(1),
            attempt_status: Some(AttStatus::Done),
            attempts: 1,
            tags: &["cli"],
            usage: Some("45k tok · $0.12"),
            priority: None,
            body: "所有 list 类子命令支持 --json，供脚本消费。",
            comments: &[Comment { author: "claude-code", time: "09-26 12:00", body: "完成。自报 done，摘要见 attempt。" }],
        },
        Task {
            id: 38,
            title: "修复竞态导致的偶发崩溃",
            column: Column::Review,
            agent: Some("gemini"),
            attempt: Some(3),
            attempt_status: Some(AttStatus::IdleAwait),
            attempts: 3,
            tags: &["bug", "core"],
            usage: None,
            priority: None,
            body: "并发上传时偶发 panic，第 3 次尝试改了锁的粒度。",
            comments: &[],
        },
        Task {
            id: 35,
            title: "补齐 e2e 测试",
            column: Column::Done,
            agent: Some("claude-code"),
            attempt: Some(2),
            attempt_status: Some(AttStatus::Done),
            attempts: 2,
            tags: &["test"],
            usage: Some("120k tok · $0.40"),
            priority: None,
            body: "登录/导出/设置三条主路径。",
            comments: &[],
        },
        Task {
            id: 36,
            title: "文档：快速上手",
            column: Column::Done,
            agent: Some("qwen"),
            attempt: Some(1),
            attempt_status: Some(AttStatus::Done),
            attempts: 1,
            tags: &["docs"],
            usage: Some("12k tok"),
            priority: None,
            body: "写一页 quickstart，配三条命令。",
            comments: &[],
        },
    ]
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

struct State {
    variant: usize,   // 0=A 1=B 2=C
    sel: usize,       // 全局任务下标
    sel_col: usize,   // A 变体的当前列
    detail: bool,
}

fn header_line() -> Line<'static> {
    Line::from(vec![
        Span::styled(" herdr-worker · my-app ", Style::new().fg(Color::Black).bg(Color::Cyan).bold()),
        Span::raw("  "),
        Span::styled("/搜索(标题)", Style::new().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("[过滤 tag=frontend ✕]", Style::new().fg(Color::Yellow)),
    ])
}

fn switcher_line(variant: usize) -> Line<'static> {
    let names = ["A 经典看板", "B 泳道列表", "C 主从双栏"];
    let mut spans = vec![Span::styled(" ◀ ", Style::new().fg(Color::Magenta).bold())];
    for (i, n) in names.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ▏", Style::new().fg(Color::DarkGray)));
        }
        if i == variant {
            spans.push(Span::styled(format!("[{n}]"), Style::new().fg(Color::Black).bg(Color::Magenta).bold()));
        } else {
            spans.push(Span::styled(format!(" {n} "), Style::new().fg(Color::Gray)));
        }
    }
    spans.push(Span::styled(" ▶ ", Style::new().fg(Color::Magenta).bold()));
    spans.push(Span::styled(" Tab 切换 · PROTOTYPE", Style::new().fg(Color::DarkGray)));
    Line::from(spans)
}

fn help_line(variant: usize) -> Line<'static> {
    let keys = match variant {
        0 => "←→ 列  ↑↓ 卡  Enter 详情  s 启动  r 自报审阅  d diff→兄弟pane  / 搜索  q 退出",
        1 => "↑↓ 行  Enter 详情  s 启动  d diff→兄弟pane  / 搜索  q 退出",
        _ => "↑↓ 选任务  s 启动  r 自报审阅  d diff→兄弟pane  / 搜索  q 退出",
    };
    Line::from(Span::styled(format!(" {keys}"), Style::new().fg(Color::DarkGray)))
}

fn card_lines(t: &Task, width: usize, selected: bool) -> Vec<Line<'static>> {
    let title_style = if selected {
        Style::new().fg(Color::Black).bg(Color::Cyan).bold()
    } else {
        Style::new().fg(Color::White).bold()
    };
    let mut lines = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{:04}", t.id), Style::new().fg(Color::Gray)),
            Span::raw(" "),
            Span::styled(trunc(t.title, width.saturating_sub(7)), title_style),
        ]),
    ];
    let mut meta = String::new();
    if let Some(a) = t.agent {
        meta.push_str(a);
    }
    if let Some(st) = t.attempt_status {
        meta.push(' ');
        meta.push_str(st.badge());
        if t.attempts > 1 {
            meta.push_str(&format!(" #{}", t.attempt.unwrap()));
        }
    }
    lines.push(Line::from(Span::styled(trunc(&meta, width), Style::new().fg(t.attempt_status.map(|s| s.color()).unwrap_or(Color::DarkGray)))));
    let tags = t.tags.iter().map(|s| format!("#{s}")).collect::<Vec<_>>().join(" ");
    let mut tail = vec![Span::styled(trunc(&tags, width.saturating_sub(10)), Style::new().fg(Color::DarkGray))];
    if let Some(u) = t.usage {
        tail.push(Span::styled(format!(" {u}"), Style::new().fg(Color::Yellow)));
    }
    lines.push(Line::from(tail));
    lines
}

fn detail_text(t: &Task) -> Text<'static> {
    let mut lines: Vec<Line> = vec![];
    let mut meta = format!(
        "列 {} · agent {} · attempt {}/{}",
        t.column.label(),
        t.agent.unwrap_or("—"),
        t.attempt.map(|n| n.to_string()).unwrap_or("—".into()),
        t.attempts,
    );
    if let Some(p) = t.priority {
        meta.push_str(&format!(" · 优先级 {p}"));
    }
    lines.push(Line::from(Span::styled(meta, Style::new().fg(Color::Cyan))));
    let mut meta2 = format!("标签 {}", t.tags.iter().map(|s| format!("#{s}")).collect::<Vec<_>>().join(" "));
    if let Some(u) = t.usage {
        meta2.push_str(&format!(" · 用量 {u}"));
    }
    lines.push(Line::from(Span::styled(meta2, Style::new().fg(Color::DarkGray))));
    lines.push(Line::from(""));
    for l in t.body.lines() {
        lines.push(Line::from(Span::raw(l.to_string())));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("attempts", Style::new().fg(Color::White).bold().underlined())));
    // 假装的时间线：末次 attempt 展开历史
    match (t.id, t.attempt_status) {
        (31, _) => {
            lines.push(Line::from(Span::styled("  #1 claude-code hw/0031-refactor-auth  ✗ failed  (测试挂了3个)", Style::new().fg(Color::Magenta))));
            lines.push(Line::from(Span::styled("  #2 claude-code hw/0031-refactor-auth  ▶ running", Style::new().fg(Color::Cyan))));
        }
        (38, _) => {
            lines.push(Line::from(Span::styled("  #1 gemini  ✗ failed", Style::new().fg(Color::Magenta))));
            lines.push(Line::from(Span::styled("  #2 gemini  ✗ failed", Style::new().fg(Color::Magenta))));
            lines.push(Line::from(Span::styled("  #3 gemini  ◔ 空闲待确认 (改了锁粒度，等你确认)", Style::new().fg(Color::Yellow))));
        }
        _ => {
            let st = t.attempt_status.map(|s| s.badge()).unwrap_or("—");
            let color = t.attempt_status.map(|s| s.color()).unwrap_or(Color::Gray);
            lines.push(Line::from(Span::styled(format!("  #{} {} {}  {}", t.attempt.unwrap_or(1), t.agent.unwrap_or("—"), format!("hw/{:04}", t.id), st), Style::new().fg(color))));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("comments", Style::new().fg(Color::White).bold().underlined())));
    for c in t.comments {
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", c.author), Style::new().fg(if c.author == "human" { Color::Green } else { Color::Blue })),
            Span::styled(c.time, Style::new().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(Span::raw(format!("  {}", c.body))));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  d = 在兄弟 pane 打开 diff (delta → difft → git diff)", Style::new().fg(Color::DarkGray))));
    Text::from(lines)
}

fn draw_a(f: &mut Frame, tasks: &[Task], st: &State) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)]).split(area);
    f.render_widget(Paragraph::new(header_line()), rows[0]);

    let cols = Column::order();
    let col_areas = Layout::horizontal([Constraint::Percentage(25); 4]).split(rows[1]);
    for (i, col) in cols.iter().enumerate() {
        let ts: Vec<&Task> = tasks.iter().filter(|t| t.column == *col).collect();
        let active_col = i == st.sel_col;
        let block = Block::bordered()
            .title(format!(" {} ({}) ", col.label(), ts.len()))
            .border_style(if active_col { Style::new().fg(Color::Cyan) } else { Style::new().fg(Color::DarkGray) });
        let width = col_areas[i].width.saturating_sub(4) as usize;
        let items: Vec<ListItem> = ts
            .iter()
            .map(|t| {
                let selected = tasks[st.sel].id == t.id;
                ListItem::new(card_lines(t, width, selected))
            })
            .collect();
        f.render_widget(List::new(items).block(block), col_areas[i]);
    }

    f.render_widget(Paragraph::new(help_line(0)), rows[2]);
    f.render_widget(Paragraph::new(switcher_line(st.variant)), rows[3]);

    if st.detail {
        let w = area.width.saturating_sub(8);
        let h = area.height.saturating_sub(4);
        let popup = Rect { x: 4, y: 2, width: w, height: h };
        f.render_widget(Clear, popup);
        let t = &tasks[st.sel];
        let block = Block::bordered()
            .title(format!(" {:04} · {} — 详情 (Esc 返回) ", t.id, t.title))
            .border_style(Style::new().fg(Color::Cyan));
        f.render_widget(Paragraph::new(detail_text(t)).block(block).wrap(Wrap { trim: false }), popup);
    }
}

fn draw_b(f: &mut Frame, tasks: &[Task], st: &State) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)]).split(area);
    f.render_widget(Paragraph::new(header_line()), rows[1 - 1]);

    let mut items: Vec<ListItem> = vec![];
    for col in Column::order() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("— {} —", col.label()),
            Style::new().fg(Color::Cyan).bold(),
        ))));
        for t in tasks.iter().filter(|t| t.column == col) {
            let selected = tasks[st.sel].id == t.id;
            let style = if selected {
                Style::new().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::new().fg(Color::White)
            };
            let stt = t.attempt_status;
            let badge = stt.map(|s| s.badge()).unwrap_or("");
            let badge_style = if selected { style } else { Style::new().fg(stt.map(|s| s.color()).unwrap_or(Color::DarkGray)) };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!(" {:04}  ", t.id), if selected { style } else { Style::new().fg(Color::Gray) }),
                Span::styled(trunc(t.title, 40), style),
                Span::styled(
                    format!("  {:<10}", trunc(t.agent.unwrap_or("—"), 10)),
                    if selected { style } else { Style::new().fg(Color::Blue) },
                ),
                Span::styled(format!("{badge:<16}"), badge_style),
                Span::styled(
                    format!("a{}", t.attempts),
                    if selected { style } else { Style::new().fg(Color::DarkGray) },
                ),
                Span::styled(
                    format!("  {}", t.usage.unwrap_or("")),
                    if selected { style } else { Style::new().fg(Color::Yellow) },
                ),
            ])));
        }
    }
    f.render_widget(List::new(items), rows[1]);
    f.render_widget(Paragraph::new(help_line(1)), rows[2]);
    f.render_widget(Paragraph::new(switcher_line(st.variant)), rows[3]);

    if st.detail {
        let w = area.width.saturating_sub(8);
        let h = area.height.saturating_sub(4);
        let popup = Rect { x: 4, y: 2, width: w, height: h };
        f.render_widget(Clear, popup);
        let t = &tasks[st.sel];
        let block = Block::bordered()
            .title(format!(" {:04} · {} — 详情 (Esc 返回) ", t.id, t.title))
            .border_style(Style::new().fg(Color::Cyan));
        f.render_widget(Paragraph::new(detail_text(t)).block(block).wrap(Wrap { trim: false }), popup);
    }
}

fn draw_c(f: &mut Frame, tasks: &[Task], st: &State) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)]).split(area);
    f.render_widget(Paragraph::new(header_line()), rows[0]);
    let cols = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)]).split(rows[1]);

    let mut items: Vec<ListItem> = vec![];
    for col in Column::order() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" {} ", col.label().to_uppercase()),
            Style::new().fg(Color::Cyan).bold(),
        ))));
        for t in tasks.iter().filter(|t| t.column == col) {
            let selected = tasks[st.sel].id == t.id;
            let dot = t.attempt_status.map(|s| s.dot()).unwrap_or("·");
            let dot_style = Style::new().fg(t.attempt_status.map(|s| s.color()).unwrap_or(Color::DarkGray));
            let title_style = if selected {
                Style::new().fg(Color::Black).bg(Color::Cyan).bold()
            } else {
                Style::new().fg(Color::White)
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!(" {dot} "),
 dot_style),
                Span::styled(format!("{:04} ", t.id), if selected { Style::new().fg(Color::Black).bg(Color::Cyan) } else { Style::new().fg(Color::Gray) }),
                Span::styled(trunc(t.title, (cols[0].width as usize).saturating_sub(12)), title_style),
            ])));
        }
    }
    let list_block = Block::bordered().title(" 任务 (9) ").border_style(Style::new().fg(Color::DarkGray));
    f.render_widget(List::new(items).block(list_block), cols[0]);

    let t = &tasks[st.sel];
    let block = Block::bordered()
        .title(format!(" {:04} · {} ", t.id, t.title))
        .border_style(Style::new().fg(Color::Cyan));
    f.render_widget(Paragraph::new(detail_text(t)).block(block).wrap(Wrap { trim: false }), cols[1]);

    f.render_widget(Paragraph::new(help_line(2)), rows[2]);
    f.render_widget(Paragraph::new(switcher_line(st.variant)), rows[3]);
}

fn draw(f: &mut Frame, tasks: &[Task], st: &State) {
    match st.variant {
        0 => draw_a(f, tasks, st),
        1 => draw_b(f, tasks, st),
        _ => draw_c(f, tasks, st),
    }
}

fn snapshot(tasks: &[Task]) -> std::io::Result<()> {
    let frames: Vec<(&str, State)> = vec![
        ("snap-A-board.txt", State { variant: 0, sel: 3, sel_col: 1, detail: false }),
        ("snap-A-detail.txt", State { variant: 0, sel: 3, sel_col: 1, detail: true }),
        ("snap-B.txt", State { variant: 1, sel: 6, sel_col: 1, detail: false }),
        ("snap-C.txt", State { variant: 2, sel: 3, sel_col: 1, detail: false }),
    ];
    for (file, st) in frames {
        let backend = TestBackend::new(110, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, tasks, &st)).unwrap();
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for y in 0..36u16 {
            let mut prev_wide = false;
            for x in 0..110u16 {
                let cell = buf.cell(Position::new(x, y)).unwrap();
                if prev_wide {
                    prev_wide = false;
                    continue;
                }
                let sym = cell.symbol();
                if !sym.is_empty() {
                    out.push_str(sym);
                    prev_wide = sym.chars().next().map(|c| c.width().unwrap_or(1) > 1).unwrap_or(false);
                }
            }
            out.push('\n');
        }
        std::fs::write(file, out)?;
        println!("wrote {file}");
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let tasks = fake_tasks();
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--snap") {
        return snapshot(&tasks);
    }

    let mut st = State { variant: 0, sel: 3, sel_col: 1, detail: false };
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| draw(f, &tasks, &st))?;
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let tasks_of_col = |c: Column| -> Vec<usize> {
                tasks.iter().enumerate().filter(|(_, t)| t.column == c).map(|(i, _)| i).collect()
            };
            match key.code {
                KeyCode::Tab => st.variant = (st.variant + 1) % 3,
                KeyCode::Char('q') => break,
                KeyCode::Esc => {
                    if st.detail {
                        st.detail = false;
                    } else {
                        break;
                    }
                }
                KeyCode::Enter => st.detail = !st.detail,
                KeyCode::Left | KeyCode::Char('h') => {
                    if st.variant == 0 {
                        st.sel_col = st.sel_col.saturating_sub(1);
                        let col = Column::order()[st.sel_col];
                        if let Some(i) = tasks_of_col(col).first() {
                            st.sel = *i;
                        }
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if st.variant == 0 {
                        st.sel_col = (st.sel_col + 1).min(3);
                        let col = Column::order()[st.sel_col];
                        if let Some(i) = tasks_of_col(col).first() {
                            st.sel = *i;
                        }
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if st.variant == 0 {
                        let col = Column::order()[st.sel_col];
                        let idxs = tasks_of_col(col);
                        let pos = idxs.iter().position(|i| *i == st.sel).unwrap_or(0);
                        if pos > 0 {
                            st.sel = idxs[pos - 1];
                        }
                    } else if st.sel > 0 {
                        st.sel -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if st.variant == 0 {
                        let col = Column::order()[st.sel_col];
                        let idxs = tasks_of_col(col);
                        let pos = idxs.iter().position(|i| *i == st.sel).unwrap_or(idxs.len().saturating_sub(1));
                        if pos + 1 < idxs.len() {
                            st.sel = idxs[pos + 1];
                        }
                    } else if st.sel + 1 < tasks.len() {
                        st.sel += 1;
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
