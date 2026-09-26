# Research 02: herdr plugin API surface inventory

Ticket: `.scratch/herdr-worker/issues/02-herdr-api-inventory.md`
Status: resolved
Sources: English docs at `E:\projects\herdr\docs\next\website\src\content\docs\` (cited as `<file> §<section>`), verified against Rust source at `E:\projects\herdr\src\` (cited as `src/...`).

Core fact: **the entire Herdr CLI is the plugin API** — there is no restricted SDK. A plugin is a directory with a `herdr-plugin.toml` manifest; Herdr validates it, injects runtime context/env, launches declared commands, and records logs. Commands call back via `HERDR_BIN_PATH` (portable CLI) or `HERDR_SOCKET_PATH` (raw newline-delimited JSON socket; Unix socket on Unix, named pipe on Windows). (plugins.mdx §intro / §Commands and environment; socket-api.mdx §Socket transport)

---

## 1. Worktree lifecycle

CLI (`cli-reference.mdx §Worktrees`; socket: socket-api.mdx §Raw methods → Worktree):

| Operation | CLI | Socket method | Notes |
| --- | --- | --- | --- |
| List | `herdr worktree list [--workspace ID \| --cwd PATH] [--trust-repository]` | `worktree.list` | Full repo worktree discovery. At most one of `workspace_id`/`cwd`; omit both for active workspace. Returns `WorktreeInfo { path, branch, is_bare, is_detached, is_prunable, is_linked_worktree, open_workspace_id, label }` (src/api/schema/worktrees.rs) |
| Create | `herdr worktree create [--workspace ID \| --cwd PATH] [--branch NAME] [--base REF] [--path PATH] [--label TEXT] [--focus\|--no-focus]` | `worktree.create` | Params: `workspace_id, cwd, branch, base, path, label, focus, trust_repository` (src/api/schema/worktrees.rs). Creates checkout + workspace; groups with parent. Existing local branch → checkout; else create from `--base`/`HEAD`. Default path `<worktrees.directory>/<repo>/<branch-slug>`. Returns `workspace`, `tab`, `root_pane`, `worktree` records. |
| Open existing | `herdr worktree open (--path PATH \| --branch NAME) ...` | `worktree.open` | Exactly one of `path`/`branch`. Returns already-open workspace if open. |
| Remove | `herdr worktree remove --workspace ID [--force]` | `worktree.remove` | Runs `git worktree remove`; never deletes the branch; `--force` for dirty checkouts. Emits `worktree.removed` (+ `workspace.closed` if still open). |

Events (socket-api.mdx §Event subscriptions; payload shapes from src/api/schema/events.rs `EventData`):
- `worktree.created` — `{workspace: WorkspaceInfo, worktree: WorktreeInfo}`. `worktree.create` also emits `workspace.created`, `tab.created`, `pane.created`.
- `worktree.opened` — `{workspace, worktree, already_open: bool}` (+ workspace/tab/pane creation events when it opens a new workspace).
- `worktree.removed` — `{workspace_id, worktree, forced, workspace?}`.
- Worktree commands can also emit `workspace.updated` when an existing workspace gains/changes worktree provenance.

Related semantics: `workspace.close` refuses to close a primary workspace while linked worktree workspaces are open unless `close_group: true` (error `workspace_group_close_required`); group close emits one `workspace.closed` per workspace (socket-api.mdx). Workspace get/list responses include optional `worktree` provenance; `session.snapshot` also includes it (§Raw methods → Session).

All three worktree events **are** available to plugin `[[events]]` hooks (src/api/schema/events.rs `PLUGIN_HOOK_EVENT_KINDS`).

## 2. Workspace / tab / pane operations

Workspace (socket-api.mdx §Raw methods; cli-reference.mdx §Workspaces):
`workspace.create/list/get/focus/rename/move/move_block/report_metadata/close`.
- `workspace.create --cwd --label --env --no-focus` → response exposes `.result.workspace.workspace_id`, `.result.tab.tab_id`, `.result.root_pane.pane_id`.
- `workspace.move_block` atomically reorders a block of `workspace_ids` (before an anchor, or to end).
- `workspace.report_metadata` — display-only workspace tokens (see §5).

Tab: `tab.create/list/get/focus/rename/move/close`. Creating a tab creates its root pane; closing the last tab closes the workspace (`confirm_close` may return `confirmation_required`).

Pane (full list; socket-api.mdx §Raw methods + src/api/schema.rs `Method` enum):
`pane.split, pane.swap, pane.move, pane.zoom, pane.layout, pane.process_info, pane.neighbor, pane.edges, pane.focus_direction, pane.focus, pane.resize, pane.list, pane.current, pane.get, pane.rename, pane.send_text, pane.send_keys, pane.send_input, pane.read, pane.wait_for_output, pane.input.set, pane.link.activate, pane.report_agent, pane.report_agent_session, pane.report_metadata, pane.clear_agent_authority, pane.release_agent, pane.close` (+ experimental `pane.graphics.info/set/clear/stream` behind `[experimental].kitty_graphics = true`; + UI-coupled `pane.scroll`, `pane.edit_scrollback`, `pane.selection.read`, `pane.copy_motion`, `pane.copy_search` in the schema).
CLI equivalents incl. `herdr pane run` (text+Enter, bracketed-paste aware) and `herdr pane wait-output`.

Constraints and shapes (socket-api.mdx):
- **split**: direction `right|down`, optional `ratio`, `cwd`, `env`, `right_click`; new pane at `.result.pane.pane_id`.
- **swap**: same-tab only; directional or explicit `source_pane_id`/`target_pane_id`; preserves split shape, ratios, pane ids, running processes. Reasons: `no_neighbor, same_pane, not_found, cross_tab`. Works while zoomed (mutates hidden layout).
- **move**: to existing tab (`split: right|down` required, optional `target_pane_id`+`ratio`), new tab, or new workspace. Zoomed source/target tab → `changed:false, reason:"zoomed_tab"`. Cross-workspace move keeps the terminal alive but assigns a **new public pane id** (`previous_pane_id` preserved as alias); emits `pane.moved` (no fake close/create). In-flight waits end with `agent_not_running` (agent-automation.mdx).
- **zoom**: `toggle|on|off`; reasons `single_pane, already_zoomed, already_unzoomed`.
- **resize**: direction + `amount` fraction.
- **read** sources: `visible | recent | recent-unwrapped | detection` (`detection` = bottom buffer used for agent screen detection); default 80 rows.
- `PaneInfo` includes `scroll` metrics, `agent_session`, `foreground_cwd`, `terminal_title(_stripped)`, `revision`.

Layout: `layout.export` (portable BSP tree of pane/split nodes), `layout.apply` (fresh tab from declarative tree; restores structure/labels/cwd/env/argv, not live PTYs), `layout.set_split_ratio`. (socket-api.mdx)

Popup: `popup.close` only (see §7 for plugin pane placements).

## 3. Agent operations

Launching (agent-automation.mdx §Agent identity and launch; cli-reference.mdx §Agents):
- `herdr agent start <name> --kind KIND --pane <ID> [--timeout MS] [-- <args>]` (socket `agent.start`). **Requires an existing available shell pane** (interactive shell owns foreground; no command/editor/agent running) — `agent start` never creates or moves layout; topology must be created separately. Name must match `[a-z][a-z0-9_-]{0,31}`, unique among live agents. Kinds: `pi, claude, codex, gemini, cursor, devin, agy, cline, omp, mastracode, opencode, copilot, kimi, kiro, droid, amp, grok, hermes, kilo, qodercli, qwen, maki, muse`. Returns only after the agent owns the terminal and is ready for input (`agent_not_ready` if `blocked` during startup; ready for prompts once `idle`). Timeout default 30000 ms (3000–300000).
- Manually launched agents are auto-detected and addressable by pane id; `agent rename` gives a stable name. The name follows the current pane occupant and is cleared when the agent exits/is released/replaced.

Prompting / messaging (agent-automation.mdx §Choose the control surface; socket-api.mdx §Raw methods):
- `agent prompt <target> <text> [--wait] [--until STATUS]... [--timeout MS]` (socket `agent.prompt` with optional `wait {until, timeout_ms}`) — submits text+encoded Enter, honors bracketed paste. Can prompt while working. Returns `agent_blocked` (without sending) if already blocked. `--wait`: must observe `working|blocked` within 5 s or `agent_prompt_stalled`; then waits for first requested settled status; does **not** track individual turns.
- `agent send-keys` — logical keys (`enter`, `esc`, `ctrl+c`, chords, `f1`, named punctuation) to the agent's interactive UI (how you answer approval dialogs deliberately).
- Agent-to-agent messaging model: an orchestrator agent (or any process) calls the CLI against another agent's name/pane — `agent prompt`, `agent send-keys`, `agent wait`, `agent read`. Agents can also invoke plugin actions: `herdr plugin action invoke <plugin.id.action>` (plugins.mdx; cli-reference.mdx §Plugins).
- Pane-level equivalents (`pane run/send-text/send-keys/send_input`) address the terminal regardless of occupant; agent commands resolve the live agent and reject if it no longer controls the pane.

Querying:
- `agent list` / `agent get <target>` → `AgentInfo` (src/api/schema/agents.rs): `terminal_id, name?, agent?, title?, terminal_title?, terminal_title_stripped?, display_agent?, agent_status, screen_detection_skipped, state_labels, tokens (display-only metadata), agent_session?, workspace_id, tab_id, pane_id, focused, launch_pending, interactive_ready, state_change_seq, cwd?, foreground_cwd?, revision`. **No usage/token-count/cost fields anywhere.**
- `agent read` — same sources as pane read; alternate-screen history reads (Claude Code, OpenCode) automatically page via the agent's mouse-scroll when `--lines` exceeds the visible screen and the agent is idle (else `agent_not_idle`).
- `agent explain` — why the pane has its current state (manifest source/version, matched rule, evidence, fallback/skip reasons).

Status determination (agents.mdx §Status authority, §Blocked state; cli-reference.mdx §Agents):
- One status authority per pane. If a **complete lifecycle integration** is installed and actively reporting → hook reports authoritatively (`pane.report_agent` from the agent's own hooks). Otherwise **screen-manifest detection**: foreground process identification + TOML rules matched against the live bottom-buffer snapshot (+ OSC title/progress evidence where emitted).
- States: `idle` (ready for input **and its tab has been seen in the focused UI**), `done` (same underlying idle after unseen background work; focusing the tab or `pane focus`/`agent focus` marks it seen — CLI reads do not), `blocked` (recognized approval/question UI; deliberately strict — unknown prompt shapes fall back to idle with `default_known_agent_idle_fallback`), `working`, `unknown` (agent present but unclassifiable — NOT success).
- Custom agents report their own state via `pane report-agent --source custom:x --agent y --state ...` and release with `pane release-agent` (integrations.mdx §Integrate your own agent).

Polling vs subscription:
- **One-shot/poll**: `agent wait <target> --until <status>... [--timeout MS]` (server-owned, event-driven, pins the resolved pane occupant); `agent list/get`; `pane read --source detection`.
- **Subscription**: socket `events.subscribe` with `{type: "pane.agent_status_changed", pane_id, agent_status?}` — pushed lines after the ack; no replay of pre-subscription events. Push payload includes `pane_id, workspace_id, agent_status, agent?, title?, display_agent?, state_labels` (src/api/schema/events.rs).
- **Plugin hook**: `[[events]] on = "pane.agent_status_changed"` fires the manifest command with `HERDR_PLUGIN_EVENT_JSON` = the full `EventEnvelope`.
- `agent.view.set` / `agent.view.clear` — transient declarative projection of the built-in Agents view (filter/sort by status, workspace, pane, attention, tokens; `source: "plugin:<id>"`; not durable — reapply from a `[[startup]]` hook; does not change `agent.list`).

## 4. Event subscription lists

### 4a. Socket `events.subscribe` (Subscription types; socket-api.mdx §Event subscriptions + src/api/schema/events.rs)

Lifecycle (all unparameterized): `workspace.created, workspace.updated, workspace.metadata_updated, workspace.renamed, workspace.moved, workspace.reordered, workspace.closed, workspace.focused, worktree.created, worktree.opened, worktree.removed, tab.created, tab.closed, tab.focused, tab.renamed, tab.moved, pane.created, pane.updated, pane.closed, pane.focused, pane.moved, pane.exited, pane.agent_detected, layout.updated`.

Parameterized: `pane.output_matched {pane_id, source, lines?, match: substring|regex, strip_ansi}`, `pane.agent_status_changed {pane_id, agent_status?}`, `pane.scroll_changed {pane_id}`.

Notes: `workspace.metadata_updated` fires for token changes/TTL expiry and is **not** delivered to plugin hooks. `pane.output_changed {pane_id, revision}` exists as an EventKind and in `events.wait`'s `EventMatch`, but is not a `Subscription` type — output watching is via `pane.output_matched` subscriptions or `pane.wait_for_output`. `events.wait` (one-shot, optional `timeout_ms`) supports EventMatch on workspace/tab/pane lifecycle + `PaneOutputChanged`/`PaneAgentDetected`/`PaneAgentStatusChanged`. No replay: subscriptions start when accepted.

### 4b. Plugin `[[events]] on = "..."` hook names (src/api/schema/events.rs `PLUGIN_HOOK_EVENT_KINDS` — the authoritative validation list)

22 names, intentionally narrower (high-volume output-change hook semantics not implemented):

- Workspace: `workspace.created`, `workspace.updated`, `workspace.closed`, `workspace.renamed`, `workspace.moved`, `workspace.reordered`, `workspace.focused`
- Worktree: `worktree.created`, `worktree.opened`, `worktree.removed`
- Tab: `tab.created`, `tab.closed`, `tab.renamed`, `tab.moved`, `tab.focused`
- Pane: `pane.created`, `pane.closed`, `pane.focused`, `pane.moved`, `pane.exited`, `pane.agent_detected`, `pane.agent_status_changed`

**Not hookable** (socket-subscribable only, or wait-only): `workspace.metadata_updated`, `pane.updated`, `layout.updated`, `pane.output_changed`, `pane.output_matched`, `pane.scroll_changed`. Unknown `on` values don't block `plugin link` but surface as warnings in `plugin.link`/`plugin.list` (socket-api.mdx §Plugin APIs).

Event hook runtime: command runs in plugin root cwd with full plugin env + `HERDR_PLUGIN_EVENT` and `HERDR_PLUGIN_EVENT_JSON` (serialized `EventEnvelope {event, data}` — same shape as the socket push, e.g. `{"event":"worktree.created","data":{"workspace":{...},"worktree":{...}}}`). Max 32 concurrent plugin commands; logs capped (200 entries, 64 KiB output each) and queryable via `plugin.log.list` (src/app/api/plugins/runtime.rs).

## 5. Usage / token / cost data

**Not exposed.** Confirmed against both docs and source:
- `AgentInfo` has no usage, token-count, cost, or turn-count fields (src/api/schema/agents.rs:184).
- The only "tokens" in the API are **display-only metadata tokens** that a plugin/integration itself reports via `pane.report_metadata` / `workspace.report_metadata` (per-resource patches, ≤16 keys/report, ≤32 keys retained, 80-char cap, optional TTL/`seq`; rendered as `$name` in sidebars). Herdr stores and echoes them but generates nothing (socket-api.mdx §Agent state reporting; cli-reference.mdx §Panes).
- No socket method or CLI command queries agent usage/cost.

Implication for the "usage tracking" feature idea: a herdr plugin **cannot** read usage/cost from herdr v1. Feasible fallbacks only: (a) scrape agent terminal output (`pane read`/`agent read` + regex, e.g. Claude Code's cost lines), (b) self-report via `report-metadata` tokens from per-agent integrations, (c) read agent-local files (e.g. `~/.claude`/`~/.codex` usage logs) from the plugin's own code. This is a hard gap; the ticket should be shaped accordingly.

## 6. Startup hooks, keybindings, link handlers, action invocation

Startup hooks (plugins.mdx §Startup hooks):
- `[[startup]]` commands run once per enabled plugin **after session restore and when the API socket is ready**; rerun when a new server takes over during live handoff; NOT rerun on client attach, config reload, or plugin link/enable. Async, one-shot init (not supervised daemons); failure does not stop the server. Env: normal plugin env + `HERDR_PLUGIN_EVENT=startup`. Recommended pattern for durable agent views: save under `HERDR_PLUGIN_STATE_DIR`, reapply from startup.

Keybindings (plugins.mdx §Keybindings; configuration.mdx §Custom command keybindings):
- `[[keys.command]]` with `type = "plugin_action"` and `command = "<plugin.id.action>"` binds a key to a plugin action → the action runs with `invocation_source = "keybinding"`.
- Other custom command types (not plugin-specific): `popup` (session-modal, `width`/`height` cells or `%`, gets `HERDR_ACTIVE_*` env, not `HERDR_PANE_ID`), `pane` (temporary zoomed pane), `shell` (detached). Custom commands receive `HERDR_SOCKET_PATH, HERDR_BIN_PATH, HERDR_ACTIVE_WORKSPACE_ID, HERDR_ACTIVE_TAB_ID, HERDR_ACTIVE_PANE_ID, HERDR_ACTIVE_PANE_CWD`.

Link handlers (plugins.mdx §Link handlers):
- `[[link_handlers]]` with Rust-regex `pattern` + `action` (same plugin). Modified click = **Control on every platform**. Matching URL is routed to the action with `invocation_source = "link_click"`, `clicked_url`, `link_handler_id` in context (+ `HERDR_PLUGIN_CLICKED_URL`, `HERDR_PLUGIN_LINK_HANDLER_ID` env). Handlers checked in manifest order.

Action invocation (plugins.mdx; socket-api.mdx §Plugin APIs; cli-reference.mdx §Plugins):
- Manifest `[[actions]]`: `id` (no dots), `title`, `contexts` = `global|workspace|tab|pane|selection`, `platforms`, `command` argv. Globally qualified as `plugin.id.action`.
- CLI: `herdr plugin action list [--plugin ID]`, `herdr plugin action invoke <action_id> [--plugin ID]`. Socket: `plugin.action.list`, `plugin.action.invoke {action_id, plugin_id?, context?}` — missing context fields filled from active workspace/tab/focused pane/worktree provenance/request id. Any process can invoke, including coding agents running in panes (this is the agent→plugin entry point). Disabled plugin → `plugin_disabled`; wrong platform → `platform_unsupported`.
- v1 limits: **no runtime action registration**; actions/panes/link handlers are manifest-only. No native non-terminal plugin UI.

## 7. Plugin panes (popup/overlay/split/tab/zoomed constraints)

(plugins.mdx §Panes; socket-api.mdx §Plugin APIs; cli-reference.mdx §Plugins)

`[[panes]]` entrypoint: `id`, `title`, `placement` (default `overlay`), optional popup `width`/`height`, `platforms`, `command`. Open via `herdr plugin pane open --plugin ID --entrypoint ID [--placement ...] [--width] [--height] [--workspace] [--target-pane] [--direction] [--cwd] [--env] [--focus]` or socket `plugin.pane.open` (request placement overrides manifest). `plugin.pane.focus` / `plugin.pane.close` manage opened panes.

- `overlay` (default): temporary zoomed overlay over the active tiled pane; restores previous focus/zoom on close.
- `popup`: **session-modal singleton, not a Herdr pane** — no pane id, no `HERDR_PANE_ID`, no pane lifecycle events, outside all `pane.*`/agent/layout/persistence APIs, doesn't change plugin focus context. Gets all terminal input including Escape; closes on command exit or `popup.close`. Dimensions: cells or `"80%"`; default half terminal; clamped to minimum. Returns `ok` after launch; `ui_busy` while Settings/Copy mode/another modal is active.
- `split` / `tab` / `zoomed`: become normal Herdr panes after open — `pane.move`, `pane.swap`, `pane.resize`, `pane.zoom` all work; plugin pane ownership follows the pane across tabs/workspaces.

## 8. Runtime environment variables

Injected into every runtime plugin command (plugins.mdx §Commands and environment; src/app/api/plugins/runtime.rs):

| Variable | Meaning |
| --- | --- |
| `HERDR_SOCKET_PATH` | Local socket (Unix socket / Windows named pipe) |
| `HERDR_BIN_PATH` | Running herdr binary — preferred portable CLI entry |
| `HERDR_ENV=1` | Set inside all Herdr-managed pane processes |
| `HERDR_PLUGIN_ID` | Plugin id |
| `HERDR_PLUGIN_ROOT` | Installed/linked plugin dir (cwd of runtime commands; don't store state here) |
| `HERDR_PLUGIN_CONFIG_DIR` | User-editable config dir (plugin-owned) |
| `HERDR_PLUGIN_STATE_DIR` | Plugin-owned durable state dir |
| `HERDR_PLUGIN_CONTEXT_JSON` | Serialized `PluginInvocationContext` (see below) |
| `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`, `HERDR_PANE_ID` | When available for the invocation (pane = focused pane; popup commands get none) |
| `HERDR_PLUGIN_ACTION_ID` | Action commands |
| `HERDR_PLUGIN_EVENT`, `HERDR_PLUGIN_EVENT_JSON` | Event hooks (name + full `EventEnvelope` JSON) |
| `HERDR_PLUGIN_ENTRYPOINT_ID` | Pane commands |
| `HERDR_PLUGIN_CLICKED_URL`, `HERDR_PLUGIN_LINK_HANDLER_ID` | Link handler actions |

`PluginInvocationContext` fields (src/api/schema/plugins.rs:364): `workspace_id?, workspace_label?, workspace_cwd?, worktree?, tab_id?, tab_label?, focused_pane_id?, focused_pane_cwd?, focused_pane_agent?, focused_pane_status?, selected_text?, invocation_source?, correlation_id?, clicked_url?, link_handler_id?`.

Manifest plumbing: `plugin install/link/unlink/enable/disable`, `plugin config-dir`, `plugin list --json`, `plugin log list --limit N`. Registry persists in `plugins.json`; global per user across sessions. No `plugin update` in v1 (reinstall). No Herdr-managed plugin storage API in v1 — plugins own their files/DB.

Other useful runtime surface: `notification.show` (title/body ≤80/240 chars, position, sound; reasons `shown/disabled/rate_limited/no_foreground_client/busy`), `client.window_title.set/clear`, `session.snapshot` / `herdr api snapshot` (one-time bootstrap: workspace/tab/pane records, layouts, agent records — pair with `events.subscribe` buffering to avoid gaps), `herdr api schema --json` (full JSON Schema of the protocol), `server.stop`, `server.reload_config`, `server.agent_manifests`, `server.reload_agent_manifests`, `integration.install/uninstall/list`.

## 9. Gaps for a kanban orchestrator plugin (explicit)

1. **No usage/token/cost query** (§5) — the biggest decision-relevant gap; usage tracking cannot be a native feature, only output-scraping / integration self-report / agent-local file reads.
2. **No Herdr-managed plugin storage API** — kanban board state (cards, columns) must live in plugin-owned files/DB under `HERDR_PLUGIN_STATE_DIR`, restored by a `[[startup]]` hook. No transactions/backup from herdr.
3. **No native (non-terminal) plugin UI, no runtime action registration** — the board must be a TUI in a plugin pane; actions/panes/link handlers are manifest-declared only. (plugins.mdx §intro)
4. **Plugin event hooks miss high-frequency/dirty events**: no `pane.updated`, `layout.updated`, `workspace.metadata_updated`, or output events as hooks (§4b). A live board needing layout/output-derived refresh must hold a raw socket subscription instead of (or in addition to) `[[events]]` hooks — meaning the plugin needs a long-lived process (which startup hooks are explicitly NOT: "one-shot initialization commands rather than supervised daemons"). This pushes toward either a TUI pane process (owns a socket connection for its lifetime) or accepting `[[events]]`-level granularity only.
5. **No event replay** — subscriptions and hooks start at accept time; missed events between server start and hook/subscribe are gone. Bootstrap must come from `session.snapshot`/`agent list`/`worktree list`, then apply live events.
6. **`agent prompt` returns `agent_blocked` without sending** when blocked — an orchestrator must resolve blocks (via `agent send-keys`, notification, or human) before queueing work; there is no built-in prompt queueing.
7. **No per-turn tracking** — `agent prompt --wait` settles on a status, not a turn boundary; "task finished" is approximated by `idle`/`done` transitions (`done` requires unseen→seen focus semantics to flip; CLI reads don't mark seen).
8. **Agent identity is ephemeral** — names clear when the agent exits/released/replaced; no durable agent-instance handle across restarts (session restore exists for official integrations only, via stored `agent_session` references).
9. **Pane id instability across workspaces** — `pane.move` cross-workspace changes the public pane id (alias retained); kanban state keyed on pane ids must track `pane.moved`.
10. **No `worktree.remove` by branch/name** — removal needs the linked workspace id (from `worktree.list` → `open_workspace_id`); no bulk/group remove besides `workspace.close --group` + per-worktree `worktree.remove`.
11. **Swap is same-tab only**; zoomed tabs block moves (`zoomed_tab`); popups are singletons and return `ui_busy` while another modal is active.
12. **Concurrency cap**: max 32 in-flight plugin commands (runtime.rs) — event-hook storms (e.g. many `pane.agent_status_changed`) could hit `plugin_command_limit_reached`; hooks should be fast/cheap.
13. **Windows transport**: raw socket = named pipe; use `HERDR_BIN_PATH` CLI calls for portability (docs' explicit recommendation).

## 10. Feasibility verdict for a kanban orchestrator

The core loop is fully supported: `worktree.create` per ticket → `agent start` in its root pane → `agent prompt` tasks → subscribe `pane.agent_status_changed` + `worktree.*` + `pane.exited`/`pane.agent_detected` → `agent read`/`pane read` results → `notification.show` on blocked → `worktree.remove` on done. Board UI: TUI in a `[[panes]]` split/tab/overlay pane; durable state under `HERDR_PLUGIN_STATE_DIR` + `[[startup]]` restore; `workspace.report_metadata` tokens for kanban status chips in the Space sidebar. The two hard constraints to design around: (1) usage/cost data must be sourced outside herdr, and (2) live board updates beyond the 22 hookable events require a long-lived socket subscription, which herdr's plugin model accommodates only via a pane process (TUI) rather than a supervised background daemon.
