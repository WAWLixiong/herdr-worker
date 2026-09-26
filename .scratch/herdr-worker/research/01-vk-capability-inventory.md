# Vibe Kanban — Complete Capability Inventory (Comparison Baseline for herdr)

Research ticket: `issues/01-vk-capability-inventory.md` · Status: resolved
Date: 2026-09-26 · Method: primary-source extraction from vibekanban.com/docs (all `.md` pages fetched directly).

> **Source access note:** `github.com/BloopAI/vibe-kanban` and `raw.githubusercontent.com` were unreachable from this environment (persistent transport errors). All findings below are sourced from the official docs site `vibekanban.com/docs`, which mirrors the product's full capability surface. The homepage confirms: *"Vibe Kanban is sunsetting. The project will continue as open source and community maintained."* ([vibekanban.com](https://vibekanban.com/))

Each checklist item cites its source URL. Use as a line-by-line baseline when writing the herdr plugin spec.

---

## 0. Product model in one paragraph

Vibe Kanban is a local-first (`npx vibe-kanban`) kanban + execution layer for third-party coding agents. Hierarchy: **Organisation → Project → Issue → (linked) Workspace → Session**. Issues are the planning unit (kanban board, cloud-synced, team-collaborative); Workspaces are the execution unit (git worktree + working branch per repository, multi-repo capable); Sessions are individual agent conversations inside a workspace (multiple runs/attempts per task). The docs still use the legacy term **"task attempts"** for what the Workspaces UI now implements as workspaces + sessions. ([docs/getting-started](https://vibekanban.com/docs/getting-started.md), [docs/workspaces/index](https://vibekanban.com/docs/workspaces/index.md))

---

## 1. Task / kanban data model

### 1.1 Issue fields
- [ ] **Title** (required) + **description** (rich text: bold/italic/lists/inline code/headings/links; auto-saves with ~0.5s debounce). ([docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md), [docs/issue-management](https://vibekanban.com/docs/issue-management.md))
- [ ] **Description becomes the agent prompt** — "Your issue description becomes the prompt your coding agent receives." ([docs/issue-management](https://vibekanban.com/docs/issue-management.md))
- [ ] **Status** (column membership), changed by drag-drop or dropdown. ([docs/cloud/kanban-board](https://vibekanban.com/docs/cloud/kanban-board.md))
- [ ] **Priority**: `urgent | high | medium | low` (default: none). Exposed via MCP `list_issue_priorities`. ([docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md), [docs/integrations/vibe-kanban-mcp-server](https://vibekanban.com/docs/integrations/vibe-kanban-mcp-server.md))
- [ ] **Assignees** — multiple people per issue. ([docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md))
- [ ] **Tags** — project-scoped, created inline with colours, used for filtering. ([docs/cloud/customisation](https://vibekanban.com/docs/cloud/customisation.md), [docs/issue-management](https://vibekanban.com/docs/issue-management.md))
- [ ] **Simple ID** — short human reference like `TASK-123` / `ISS-1`, shown on cards, panel, URL. ([docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md))
- [ ] **Age / created / updated timestamps** surface on cards and list rows. ([docs/cloud/list-view](https://vibekanban.com/docs/cloud/list-view.md))

### 1.2 Status columns
- [ ] **Six default statuses per project**: `Backlog` (hidden), `To do`, `In progress`, `In review`, `Done`, `Cancelled` (hidden). ([docs/cloud/projects](https://vibekanban.com/docs/cloud/projects.md))
- [ ] **Custom columns**: add, rename (display-only; issues keep status), recolour, reorder, hide/show, delete (only when empty). ([docs/cloud/customisation](https://vibekanban.com/docs/cloud/customisation.md))
- [ ] Status tabs above board: `Active` (kanban), `All` (list view incl. hidden), per-hidden-status tabs. ([docs/cloud/kanban-board](https://vibekanban.com/docs/cloud/kanban-board.md))
- [ ] **Issue statuses update automatically when coding agents start work and when PRs are created or merged** (team workflow claim). ([vibekanban.com homepage](https://vibekanban.com/))

### 1.3 Attempts / multiple runs per task
- [ ] **Multiple workspaces can link to one issue** — "useful for running agents in parallel on different parts of the same feature." ([docs/issue-management](https://vibekanban.com/docs/issue-management.md))
- [ ] **Multiple sessions per workspace** — each session = separate agent conversation with own history; sessions **share files/git state but not conversation context**. ([docs/workspaces/sessions](https://vibekanban.com/docs/workspaces/sessions.md))
- [ ] Legacy/parallel terminology: agent/variant is chosen "when creating task attempts"; default agent used "when creating new task attempts or follow-ups". ([docs/supported-coding-agents](https://vibekanban.com/docs/supported-coding-agents.md), [docs/settings/general](https://vibekanban.com/docs/settings/general.md))
- [ ] "Create draft workspace immediately" toggle at issue creation. ([docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md))
- [ ] Workspaces do **not** have to be linked to an issue (ad-hoc questions about a codebase). ([docs/getting-started](https://vibekanban.com/docs/getting-started.md))
- [ ] Workspace-level status shown on issue panel: Active/idle/completed + PR status. ([docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md))

### 1.4 Comments / discussion
- [ ] **Issue comments thread**: add comment, emoji reactions, edit/delete own comments (Cloud issues). ([docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md), [docs/cloud/index](https://vibekanban.com/docs/cloud/index.md))
- [ ] **Inline diff review comments** (separate mechanism): collected as pending, sent to agent together with next chat message, then consumed into chat history; badge shows pending count; Clear button. ([docs/reviewing-code](https://vibekanban.com/docs/reviewing-code.md), [docs/workspaces/chat-interface](https://vibekanban.com/docs/workspaces/chat-interface.md))
- [ ] **GitHub PR comments displayed inline in the diff** (only submitted reviews; per-file badge count). ([docs/workspaces/changes](https://vibekanban.com/docs/workspaces/changes.md))

### 1.5 Hierarchy & relations
- [ ] **Sub-issues**: parent/child, max one level deep; children have independent status; completing all children does NOT complete parent; deleting parent makes children standalone; create-new or link-existing. ([docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md), [docs/issue-management](https://vibekanban.com/docs/issue-management.md))
- [ ] **Issue relationships** (beyond parent/child) via MCP: `blocking`, `related`, `has_duplicate`. ([docs/integrations/vibe-kanban-mcp-server](https://vibekanban.com/docs/integrations/vibe-kanban-mcp-server.md))
- [ ] **Projects** group issues + statuses + tags; orgs group projects; members with Admin/Member roles. ([docs/cloud/projects](https://vibekanban.com/docs/cloud/projects.md), [docs/cloud/index](https://vibekanban.com/docs/cloud/index.md))

---

## 2. Execution orchestration (per-task environments)

### 2.1 Worktrees & branches
- [ ] **Git worktree per repository per workspace** — original repo untouched; isolated working directory + branch; multiple workspaces on same repo don't conflict. ([docs/workspaces/index](https://vibekanban.com/docs/workspaces/index.md), [docs/workspaces/repositories](https://vibekanban.com/docs/workspaces/repositories.md))
- [ ] **Working branch auto-created from target branch**, name pattern `{prefix}/{hash}-{task-slug}` e.g. `vk/1a2b-implement-auth`; prefix configurable. ([docs/workspaces/creating-workspaces](https://vibekanban.com/docs/workspaces/creating-workspaces.md), [docs/settings/general](https://vibekanban.com/docs/settings/general.md))
- [ ] **Target branch** (merge destination) set per repo, changeable later (may require rebase). ([docs/workspaces/repositories](https://vibekanban.com/docs/workspaces/repositories.md))
- [ ] **Multi-repo workspaces**: each repo has independent git state (own working/target branch, commits, PRs); agent reads/edits across repos in one session. ([docs/workspaces/repositories](https://vibekanban.com/docs/workspaces/repositories.md))
- [ ] Worktree storage: default OS temp (`%TEMP%\vibe-kanban\worktrees` on Windows, `/var/tmp/...` Linux, `$TMPDIR/...` macOS) or `{configured-dir}/.vibe-kanban-workspaces`; nothing pushed to remote until explicit PR. ([docs/workspaces/managing-workspaces](https://vibekanban.com/docs/workspaces/managing-workspaces.md), [docs/workspaces/creating-workspaces](https://vibekanban.com/docs/workspaces/creating-workspaces.md))
- [ ] Manually deleted worktrees are **auto-recreated from the branch's last commit** (uncommitted changes lost); orphaned worktrees cleaned at startup. ([docs/workspaces/managing-workspaces](https://vibekanban.com/docs/workspaces/managing-workspaces.md))

### 2.2 Terminals
- [ ] **Integrated terminal per workspace** (xterm.js, full emulation, persistent across panel toggles) — Workspaces UI only, not in classic UI. ([docs/workspaces/interface](https://vibekanban.com/docs/workspaces/interface.md))
- [ ] **Logs panel**: per-process tabs, real-time stdout/stderr, log search. ([docs/workspaces/interface](https://vibekanban.com/docs/workspaces/interface.md))

### 2.3 Dev servers & preview
- [ ] **Dev server script per repository** (e.g. `npm run dev`); started via Play icon / command bar; blue indicator when running. ([docs/settings/projects-repositories](https://vibekanban.com/docs/settings/projects-repositories.md), [docs/workspaces/interface](https://vibekanban.com/docs/workspaces/interface.md))
- [ ] URL **auto-detected from dev-server stdout** and loaded into built-in preview browser. ([docs/browser-testing](https://vibekanban.com/docs/browser-testing.md))
- [ ] **Built-in preview browser**: back/forward, URL bar, refresh, open-in-tab; **device modes** (Desktop / Mobile 390×844 frame / Responsive drag); multiple dev servers as tabs. ([docs/browser-testing](https://vibekanban.com/docs/browser-testing.md), [docs/workspaces/interface](https://vibekanban.com/docs/workspaces/interface.md))
- [ ] **Click-to-component inspect** — selected element is sent to chat as agent context; works with React, Vue, Svelte, Astro, plain HTML. ([docs/browser-testing](https://vibekanban.com/docs/browser-testing.md))
- [ ] **Built-in DevTools** (Eruda): Console, Elements, Network, Resources, Sources, Info. ([docs/browser-testing](https://vibekanban.com/docs/browser-testing.md))

### 2.4 Lifecycle scripts
- [ ] **Setup script per repo** — runs once at workspace start, before agent (deps install, builds, env prep). ([docs/settings/projects-repositories](https://vibekanban.com/docs/settings/projects-repositories.md), [docs/workspaces/creating-workspaces](https://vibekanban.com/docs/workspaces/creating-workspaces.md))
- [ ] **Cleanup script per repo** — runs when workspace closes (stop services, free resources, format code); recommended idempotent. ([docs/settings/projects-repositories](https://vibekanban.com/docs/settings/projects-repositories.md))
- [ ] Scripts editable via MCP tools (`update_setup_script`, `update_cleanup_script`, `update_dev_server_script`). ([docs/integrations/vibe-kanban-mcp-server](https://vibekanban.com/docs/integrations/vibe-kanban-mcp-server.md))

### 2.5 Session orchestration
- [ ] Session states: `Running` / `Idle` / `Needs Attention` (approval or question pending, raised-hand icon); workspace sidebar aggregates; accordion grouping by status. ([docs/workspaces/sessions](https://vibekanban.com/docs/workspaces/sessions.md), [docs/workspaces/interface](https://vibekanban.com/docs/workspaces/interface.md))
- [ ] **Queue a follow-up message while agent is running**; Stop button. ([docs/workspaces/chat-interface](https://vibekanban.com/docs/workspaces/chat-interface.md))
- [ ] **Edit-and-resend any user message** — creates a new conversation branch; later messages replaced. ([docs/workspaces/chat-interface](https://vibekanban.com/docs/workspaces/chat-interface.md))
- [ ] Different agents per session within one workspace. ([docs/workspaces/sessions](https://vibekanban.com/docs/workspaces/sessions.md))
- [ ] Workspace actions: rename, **duplicate** (same repos/branches, fresh conversation), pin, archive (preserves everything), delete. ([docs/workspaces/managing-workspaces](https://vibekanban.com/docs/workspaces/managing-workspaces.md))

### 2.6 Git operations
- [ ] PR creation from Git panel or command bar: title auto-filled from task, **AI-generated PR description**, base branch, **draft PR** option. ([docs/workspaces/git-operations](https://vibekanban.com/docs/workspaces/git-operations.md))
- [ ] Merge (target → working branch) with behind-target detection prompting rebase. ([docs/workspaces/git-operations](https://vibekanban.com/docs/workspaces/git-operations.md))
- [ ] Rebase with **conflict resolution dialog** (file list, resolve/abort, mark resolved); agent can be asked to help resolve conflicts. ([docs/workspaces/git-operations](https://vibekanban.com/docs/workspaces/git-operations.md))
- [ ] Push appears only when unpushed commits + open PR exist; per-repo actions in multi-repo workspaces (Create PR / Merge / Rebase / Change Target / Push per repo). ([docs/workspaces/git-operations](https://vibekanban.com/docs/workspaces/git-operations.md))
- [ ] **Workspace auto-links to PR** when a PR is created from it or the branch matches an existing PR; PR status badge in sidebar and on issue. ([docs/workspaces/changes](https://vibekanban.com/docs/workspaces/changes.md), [docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md))
- [ ] GitHub integration via **GitHub CLI (`gh`)** — auto-detected, guided install (Homebrew on macOS), `gh auth login`; no manual VK-side config. Azure Repos integration also exists. ([docs/integrations/github-integration](https://vibekanban.com/docs/integrations/github-integration.md), [docs index](https://vibekanban.com/docs/llms.txt))

---

## 3. Diff review & feedback-to-agent

- [ ] **Changes panel**: hierarchical file tree (added/modified/deleted), filename search, expand/collapse all. ([docs/workspaces/changes](https://vibekanban.com/docs/workspaces/changes.md))
- [ ] **Diff viewer**: syntax-highlighted, colour-coded (green add / red del / grey context), dual line numbers. ([docs/workspaces/changes](https://vibekanban.com/docs/workspaces/changes.md))
- [ ] **Unified and side-by-side modes**; diff options: wrap lines, ignore whitespace, expand/collapse all sections. ([docs/reviewing-code](https://vibekanban.com/docs/reviewing-code.md), [docs/workspaces/changes](https://vibekanban.com/docs/workspaces/changes.md))
- [ ] **Inline comments on diff lines** — hover → comment icon → write → submit; purposes: request change / ask question / provide context / approve section. ([docs/reviewing-code](https://vibekanban.com/docs/reviewing-code.md))
- [ ] **Feedback loop**: comments are batched and delivered to the agent with your next chat message (with optional extra context); agent sees them and acts; repeat review→feedback until satisfied, then create PR. ([docs/reviewing-code](https://vibekanban.com/docs/reviewing-code.md))
- [ ] Chat shows per-file change counts (+lines/−lines) clickable into Changes panel. ([docs/workspaces/chat-interface](https://vibekanban.com/docs/workspaces/chat-interface.md))
- [ ] Changes panel resets after commits are pushed; check Git section for latest commit. ([docs/reviewing-code](https://vibekanban.com/docs/reviewing-code.md))
- [ ] **Approval workflow** for agent plans: approval card with plan summary/details, Approve or Request Changes; approval timeouts require re-sending a message. ([docs/workspaces/chat-interface](https://vibekanban.com/docs/workspaces/chat-interface.md))

---

## 4. Coding agent integration

### 4.1 Supported agents
- [ ] **Ten documented agents**: Claude Code, OpenAI Codex, GitHub Copilot CLI, Gemini CLI, Amp, Cursor Agent CLI, OpenCode (SST), Factory Droid, CCR (Claude Code Router), Qwen Code. ([docs/supported-coding-agents](https://vibekanban.com/docs/supported-coding-agents.md))
- [ ] MCP executor identifiers: `claude-code`, `amp`, `gemini`, `codex`, `opencode`, `cursor_agent`, `qwen-code`, `copilot`, `droid`. ([docs/integrations/vibe-kanban-mcp-server](https://vibekanban.com/docs/integrations/vibe-kanban-mcp-server.md))

### 4.2 Auth model
- [ ] **Bring-your-own-CLI**: install and authenticate each agent in its own CLI first (e.g. `npx -y @anthropic-ai/claude-code` then login; `npx -y opencode-ai` then login); VK detects and lists it for attempts. No VK-managed agent credentials. ([docs/agents/claude-code](https://vibekanban.com/docs/agents/claude-code.md), [docs/agents/opencode](https://vibekanban.com/docs/agents/opencode.md))

### 4.3 Agent profiles / variants
- [ ] **Named configuration variants per agent** (e.g. Default, Opus, Approvals, Plan; or GLM, OPENROUTER), form-based editor, clone-from, one default per agent, min one config per agent. ([docs/settings/agent-configurations](https://vibekanban.com/docs/settings/agent-configurations.md))
- [ ] Per-agent options: Claude Code `plan`, `claude_code_router`, `dangerously_skip_permissions`; Codex `sandbox` (read-only / workspace-write / danger-full-access), `approval` (untrusted / on-failure / on-request / never), `model_reasoning_effort`, `model_reasoning_summary`; Gemini `model`, `yolo`; Amp `dangerously_allow_all`; Cursor `force`, `model`; OpenCode `model`, `agent`; Qwen `yolo`; Droid `autonomy`, `model`, `reasoning_effort`. Universal: `append_prompt`. ([docs/settings/agent-configurations](https://vibekanban.com/docs/settings/agent-configurations.md))
- [ ] **Per-profile environment variables** injected at agent launch (override shell); enables third-party providers via `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN` (Z.ai GLM, OpenRouter, self-hosted). ([docs/settings/agent-configurations](https://vibekanban.com/docs/settings/agent-configurations.md))
- [ ] Default agent + variant preselected in attempt creation and follow-ups; overridable per attempt. ([docs/settings/general](https://vibekanban.com/docs/settings/general.md))
- [ ] **MCP servers configurable per agent inside VK** (JSON editor + one-click popular servers: Vibe Kanban itself, Context7, Playwright, Exa, Chrome DevTools, Dev Manager); writes through to the agent's own global config file. ([docs/settings/mcp-servers](https://vibekanban.com/docs/settings/mcp-servers.md))
- [ ] **Slash commands surfaced from each agent's native CLI** (discovered per workspace+agent, cached; TUI-only commands excluded): e.g. Claude Code `/compact`, `/review`, `/cost`, `/context`; Codex `/status`, `/model`; OpenCode `/models`, `/agents`. ([docs/workspaces/slash-commands](https://vibekanban.com/docs/workspaces/slash-commands.md))

---

## 5. Non-interactive / agent-driven API

Vibe Kanban's only documented programmatic surface is a **local MCP server** (stdio), not a REST API. Launched via `npx -y vibe-kanban@latest --mcp`; **local-only** (no public URLs). ([docs/integrations/vibe-kanban-mcp-server](https://vibekanban.com/docs/integrations/vibe-kanban-mcp-server.md))

- [ ] **Context**: `get_context` — inside an active workspace session, returns project/issue/workspace metadata (agent self-discovery).
- [ ] **Orgs/projects**: `list_organizations`, `list_org_members`, `list_projects`.
- [ ] **Issues**: `list_issues` (filters: `project_id`, `status`, `priority`, `search`, `simple_id`, `parent_issue_id`, `assignee_user_id`, `tag_id`, `tag_name`, `limit`/`offset` pagination), `create_issue` (`title` required; optional `description`, `priority`, `parent_issue_id`), `get_issue` (full detail incl. tags, relations, sub-issues, PRs), `update_issue` (`parent_issue_id` three-state: omit/null/UUID), `delete_issue`, `list_issue_priorities`.
- [ ] **Assignees**: `list_issue_assignees`, `assign_issue`, `unassign_issue`.
- [ ] **Tags**: `list_tags`, `list_issue_tags`, `add_issue_tag`, `remove_issue_tag`.
- [ ] **Relationships**: `create_issue_relationship` / `delete_issue_relationship` (`blocking` / `related` / `has_duplicate`).
- [ ] **Repos & scripts**: `list_repos`, `get_repo` (incl. scripts), `update_setup_script`, `update_cleanup_script`, `update_dev_server_script`.
- [ ] **Workspaces**: `list_workspaces` (filters: `archived`, `pinned`, `branch`, `name_search`, pagination), `update_workspace` (name/archive/pin), `delete_workspace` (`delete_remote`, `delete_branches` options), `link_workspace_issue`.
- [ ] **Sessions / execution**: `start_workspace` (required `name`, `executor`, `repositories[{repo_id, branch}]`; optional `prompt`, `variant`, `issue_id` — **if `prompt` omitted, the linked issue's title+description become the prompt**), `create_session` (`workspace_id`, `executor`), `list_sessions`, **`run_session_prompt`** (`session_id`, `prompt` → execution), **`get_execution`** (`execution_id` → status + final message). This is a submit-then-poll agent-driven execution loop.
- [ ] **Context inference**: `project_id` / `organization_id` auto-inferred when running inside a workspace linked to a remote project (exception: `list_projects` always needs explicit `organization_id`).
- [ ] **Self-hosting agents pattern**: an agent inside VK with the VK MCP server attached can create issues from a plan (planning-agent workflow) and start workspaces for issues. ([docs/integrations/vibe-kanban-mcp-server](https://vibekanban.com/docs/integrations/vibe-kanban-mcp-server.md))

All tool tables: [docs/integrations/vibe-kanban-mcp-server](https://vibekanban.com/docs/integrations/vibe-kanban-mcp-server.md)

---

## 6. Search & filtering

- [ ] Board **search matches issue titles only**; case-insensitive; partial matches; live as-you-type; Simple ID lookup. ([docs/cloud/filtering](https://vibekanban.com/docs/cloud/filtering.md))
- [ ] Filters: **priority** (multi-select), **assignee** (multi-select + Unassigned), **tags** (multi-select). ([docs/cloud/filtering](https://vibekanban.com/docs/cloud/filtering.md))
- [ ] Filter logic: **OR within a category, AND across categories**. ([docs/cloud/filtering](https://vibekanban.com/docs/cloud/filtering.md))
- [ ] Sorting per column: **Manual (drag-reorder), Priority, Created, Updated, Title**; ascending/descending toggle; non-Manual disables drag-reorder. ([docs/cloud/filtering](https://vibekanban.com/docs/cloud/filtering.md))
- [ ] Clear All / per-filter clear; filters persist per session, reset on project switch/sign-out; personal (not shared). ([docs/cloud/filtering](https://vibekanban.com/docs/cloud/filtering.md))
- [ ] Team/Personal tabs (all project issues vs assigned-to-me). ([docs/issue-management](https://vibekanban.com/docs/issue-management.md))
- [ ] List view (All tab): grouped by status, collapsible, shows hidden statuses; multi-select with bulk actions (status/priority/assignees/delete) via checkboxes, Cmd/Ctrl+Click, Shift+Click, Shift+J/K, Cmd/Ctrl+A. ([docs/cloud/list-view](https://vibekanban.com/docs/cloud/list-view.md), [docs/cloud/issues](https://vibekanban.com/docs/cloud/issues.md))
- [ ] Workspace sidebar search: filter workspaces by name or branch. ([docs/workspaces/interface](https://vibekanban.com/docs/workspaces/interface.md))
- [ ] Changes-panel file filter by name; logs search. ([docs/workspaces/changes](https://vibekanban.com/docs/workspaces/changes.md), [docs/workspaces/interface](https://vibekanban.com/docs/workspaces/interface.md))
- [ ] MCP `list_issues` mirrors filters server-side (status/priority/search/parent/assignee/tag + pagination). ([docs/integrations/vibe-kanban-mcp-server](https://vibekanban.com/docs/integrations/vibe-kanban-mcp-server.md))

---

## 7. Usage tracking (tokens / cost)

- [ ] **Per-session context gauge**: % of context window used with absolute tokens (e.g. "13% — 27K of 200K"); colour thresholds grey 0–30%, default 30–60%, orange 60–80%, red 80%+ (start a new session). ([docs/workspaces/chat-interface](https://vibekanban.com/docs/workspaces/chat-interface.md), [docs/workspaces/sessions](https://vibekanban.com/docs/workspaces/sessions.md))
- [ ] **Cost/duration via agent slash commands**, not a VK-native dashboard: Claude Code `/cost` (total cost + duration of current session), `/context`; Codex `/status` (session config + token usage). ([docs/workspaces/slash-commands](https://vibekanban.com/docs/workspaces/slash-commands.md))
- [ ] Agent task-list progress indicator in chat (e.g. "5/5 completed" with expandable steps). ([docs/workspaces/chat-interface](https://vibekanban.com/docs/workspaces/chat-interface.md))
- [ ] **No documented aggregate token/cost dashboard or per-attempt cost history** in the current docs — usage visibility is per-session and largely delegated to the agent CLIs.

---

## 8. Configuration options

- [ ] **General**: theme (light/dark), language; default coding agent + variant; editor (VS Code, Cursor, Windsurf, Zed, Antigravity, Neovim, Emacs, Sublime, custom shell command) with **Remote SSH host** for VS Code/Cursor/Windsurf when VK runs on a remote server (`vscode://vscode-remote/...` URLs). ([docs/settings/general](https://vibekanban.com/docs/settings/general.md))
- [ ] **Git**: branch prefix for auto-generated branches; workspace (worktree) directory. ([docs/settings/general](https://vibekanban.com/docs/settings/general.md))
- [ ] **Notifications**: sound effects, browser push notifications; telemetry toggle; Enter vs Cmd/Ctrl+Enter to send. ([docs/settings/general](https://vibekanban.com/docs/settings/general.md))
- [ ] **Task tags** (prompt snippets): global reusable text snippets inserted into prompts via `@mention`. ([docs/settings/general](https://vibekanban.com/docs/settings/general.md), [docs/settings/creating-task-tags](https://vibekanban.com/docs/settings/creating-task-tags.md))
- [ ] **Projects & repositories**: per-project settings; per-repo display name, path, dev-server/setup/cleanup scripts. ([docs/settings/projects-repositories](https://vibekanban.com/docs/settings/projects-repositories.md))
- [ ] **Agent profiles** (see §4.3) and **MCP servers** (see §4.3) settings pages. ([docs/settings/agent-configurations](https://vibekanban.com/docs/settings/agent-configurations.md), [docs/settings/mcp-servers](https://vibekanban.com/docs/settings/mcp-servers.md))
- [ ] **Remote access**: pairing code flow — access a local host instance's workspaces from another device (e.g. mobile) via cloud.vibekanban.com. ([docs/remote-access](https://vibekanban.com/docs/remote-access.md))
- [ ] **Cloud vs local**: sign-in (GitHub/Google) optional; without sign-in only workspaces work (no board/issues/team); cloud adds orgs, members, real-time sync; self-hosting via Docker Compose / local development documented. ([docs/getting-started](https://vibekanban.com/docs/getting-started.md), [docs/cloud/index](https://vibekanban.com/docs/cloud/index.md), [docs/self-hosting/deploy-docker](https://vibekanban.com/docs/self-hosting/deploy-docker.md))
- [ ] Onboarding wizard: preferred agent, IDE, notifications. ([docs/getting-started](https://vibekanban.com/docs/getting-started.md))
- [ ] VSCode extension integration (VSCode, Cursor, Windsurf). ([docs/integrations/vscode-extension](https://vibekanban.com/docs/integrations/vscode-extension.md))

---

## 9. Things that would surprise a spec writer (comparison-relevant deltas)

1. **Three-level execution model**: Issue → Workspace (worktree env, N per issue) → Session (agent run, N per workspace, share files but not context). "Attempt" is legacy vocabulary still sprinkled through settings docs. A herdr "task → attempt" model maps to VK's "issue → workspace+session", but VK lets one issue fan out to multiple independent worktrees AND multiple conversation threads inside one worktree.
2. **The non-interactive API is MCP-only, local stdio** (`npx vibe-kanban --mcp`). No REST/HTTP API is documented. It is nevertheless complete: full issue/tag/assignee/relationship CRUD, repo script editing, workspace/session lifecycle, and a `run_session_prompt` → `get_execution` submit/poll execution primitive. `start_workspace` without a `prompt` auto-uses the linked issue's title+description as the prompt.
3. **Agents are expected to call the API on themselves** — the documented "planning agent" pattern has an agent inside VK create the issue breakdown for its own project via the VK MCP server (`get_context` provides self-identity; `project_id`/`organization_id` are inferred from workspace context).
4. **Review comments are batched, not live**: inline diff comments accumulate as pending and are only delivered to the agent with the next chat message, after which they are consumed into chat history. There is no per-comment thread on the diff itself.
5. **Auth is fully delegated**: VK stores no agent credentials — each agent CLI must be pre-installed and pre-authenticated; VK just wraps the CLI (env vars per profile are the only credential-adjacent surface). GitHub PRs go through the `gh` CLI, likewise auto-detected.
6. **Search is title-only** (case-insensitive partial match) in the board UI; richer filtering exists only through the filter dropdowns (priority/assignee/tag) and the MCP `list_issues` parameters.
7. **Usage tracking is minimal**: one context gauge per session plus agent-native `/cost`-style slash commands; no aggregate cost dashboard documented.
8. **Board statuses are customizable per project** (add/rename/recolour/reorder/hide/delete-empty) on top of six defaults, and issue status changes can be **triggered automatically by agent lifecycle events** (agent starts work; PR created/merged).
9. **Sub-issues are strictly one level deep** and never auto-complete their parent; richer relations (`blocking`, `related`, `has_duplicate`) exist only through the MCP API, not the board UI.
10. **Worktree lifecycle safety**: archive preserves everything; delete removes worktree + conversations but keeps branches/commits; manually deleted worktrees are silently recreated from the branch tip (uncommitted work lost); orphaned worktrees are reaped at startup.

---

## Source index (all fetched 2026-09-26)

Primary: https://vibekanban.com/docs/llms.txt (index) · getting-started.md · issue-management.md · reviewing-code.md · browser-testing.md · remote-access.md · supported-coding-agents.md · agents/claude-code.md · agents/opencode.md · workspaces/index.md · workspaces/creating-workspaces.md · workspaces/managing-workspaces.md · workspaces/repositories.md · workspaces/sessions.md · workspaces/chat-interface.md · workspaces/slash-commands.md · workspaces/interface.md · workspaces/changes.md · workspaces/git-operations.md · cloud/index.md · cloud/projects.md · cloud/issues.md · cloud/kanban-board.md · cloud/list-view.md · cloud/filtering.md · cloud/customisation.md · settings/general.md · settings/agent-configurations.md · settings/mcp-servers.md · settings/projects-repositories.md · settings/creating-task-tags.md · integrations/vibe-kanban-mcp-server.md · integrations/github-integration.md · https://vibekanban.com/ (homepage, sunsetting notice)

Unreachable: github.com/BloopAI/vibe-kanban, raw.githubusercontent.com/BloopAI/vibe-kanban/main/README.md (transport errors from this environment).
