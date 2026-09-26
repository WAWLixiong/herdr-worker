# herdr-worker

Kanban for coding agents — a [herdr](https://herdr.dev) plugin.

Plan work as tasks on a board, dispatch them to coding agents, and review the results — without leaving your terminal. The board is **markdown in your repo**: tasks, attempts and comments are plain files, so humans and agents read and write the same source of truth.

> A capability successor to [Vibe Kanban](https://vibekanban.com) (now sunsetting), reimagined as a pure TUI on the herdr runtime.

## What you get

- A kanban board (ratatui TUI) in a herdr pane: columns, cards, detail overlay
- One keypress dispatches a task: **git worktree + agent + prompt, fully managed**
- Live status backflow: ▶ running · ⛔ blocked (with a herdr toast notification) · ⏳ idle awaiting confirmation
- Diff review in a sibling pane (delta → difftastic → `git diff` fallback)
- Agent-driven API: agents create tasks, report results, even start work themselves
- Comments (optionally anchored to a file/hunk) are batch-delivered to the agent on follow-up
- Optional per-attempt usage self-report

## Requirements

- herdr ≥ 0.7.0
- Rust toolchain (the plugin builds from source)
- git

## Install

From the marketplace (once published):

```bash
herdr plugin install <owner>/herdr-worker
```

Or link a local checkout for development:

```bash
herdr plugin link /path/to/herdr-worker
herdr plugin action invoke herdr-worker.open-board   # opens the Board pane
```

> Note: `unicode-segmentation` is vendored under `vendor/` (its published 1.13.1 was yanked; this pins the exact version herdr itself uses). Keep the directory when cloning.

## Quickstart

1. Create a board in your repo — `.herdr-worker/board.md`:

   ```yaml
   ---
   columns: [backlog, todo, in-progress, review, done, cancelled]
   hidden: [backlog, cancelled]
   branch_prefix: hw
   default_agent: claude
   ---
   ```

2. Create a task — `.herdr-worker/tasks/0001-fix-login/task.md`:

   ```yaml
   ---
   id: 0001
   title: Fix the login page
   status: todo
   ---
   The task body **is** the prompt the agent receives.
   ```

3. Open the board in herdr, navigate to the card, press `s`. A worktree appears, your agent boots inside it, and the card moves to `in-progress` — all by itself.

## Board layout

```
.herdr-worker/
├── board.md                  # columns, hidden, branch_prefix, default_agent
└── tasks/
    └── 0042-fix-login/
        ├── task.md           # frontmatter + body (= prompt)
        ├── attempts/
        │   └── 1.md          # agent, branch, pane anchor, status, usage, summary
        └── comments/
            └── 20260926-1430.md   # author, optional file/hunk anchor, body
```

Everything is committed to git by default (clone = the whole board; agents self-discover their task card inside their worktree). Want a personal scratch board instead? Add `.herdr-worker/` to `.gitignore`.

## Keys

| Key | Action |
| --- | --- |
| `←→` / `h l` | move between columns |
| `↑↓` / `j k` | move between cards |
| `Enter` / `Esc` | open / close the detail overlay |
| `s` | start attempt (worktree + agent + prompt) |
| `r` | confirm a reviewed task into `done` |
| `f` | follow-up: deliver undelivered comments to the agent |
| `d` | open the worktree diff in a sibling pane |
| `x` | stop the active attempt (cancelled; worktree kept) |
| `c` | clean the task's plugin-owned worktree (branch never deleted) |
| `H J K L` | move card to adjacent column / reorder within column |
| `/` | filter by title substring, or `tag:xxx` (`Esc` clears) |
| `q` | quit |

## Status model (two layers)

- **Live**: herdr events drive attempt state — `working → running`, `blocked → blocked` (toast + sidebar chip). `idle` is **not** completion: the card shows `⏳ idle awaiting confirmation` until the agent self-reports or a human decides. A pane/worktree vanishing without a self-report conservatively marks the attempt `failed`.
- **Terminal**: the agent self-reports via `report-result` (`done` auto-moves the task to `review`; a human confirms with `r`), or the attempt is stopped/cancelled.

## Agent API — markdown is the API

**Read**: any process (agents included) can read the markdown directly; inside a task worktree the card is right there.

**Write**: call the plugin binary. Agents usually receive the exact coordinates in the prompt footer automatically. Discovery: `herdr plugin list --json` → `plugin_root` → `<plugin_root>/target/release/herdr-worker`.

```bash
herdr-worker act create-task --title "Add pagination" [--status todo] [--tag api] [--body-file -]
herdr-worker act move-task --id N --to <column>
herdr-worker act add-comment --id N [--file src/x.rs] [--body "..."]
herdr-worker act report-result --id N --attempt M --status done|failed [--usage "45k tok"] --body "summary"
herdr-worker act start-attempt --id N [--agent claude]
herdr-worker act stop-attempt --id N
herdr-worker act follow-up --id N
herdr-worker act clean-worktree --id N
```

All writes are guarded by a board file lock; hand-editing files is a first-class citizen (the board hot-reloads).

## Publishing to the marketplace

herdr's marketplace auto-indexes public GitHub repos tagged `herdr-plugin` with a parseable `herdr-plugin.toml` at the root. Tag the repo, push, share `herdr plugin install <owner>/herdr-worker`.

## Non-goals (v1)

No PR creation/merge · no multi-repo · no multi-board (use tags + filter) · no collaboration/cloud sync · no browser preview · no TUI column editor (hand-edit `board.md`; the board hot-reloads) · English UI only (no i18n framework).

## License

[MIT](LICENSE)
