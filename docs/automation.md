# Project setup and bounded research wakes

`paperhands init --name NAME --research 'the research question'` creates a stable
project, preserves a user-owned `research/BRIEF.md` and `research/program.md`, records
direction and seeds a four-task research DAG. Repeating the same bootstrap preserves
existing decisions; a different brief requires an explicit new direction. Partial
initialization remains inspectable. No collector, model or evaluator starts.

Native MCP registration is included in normal project init. `paperhands setup`
also installs it independently: user Codex `~/.codex/config.toml` and Claude
`~/.claude.json`; `--project-dir DIR` additionally binds `.codex/config.toml` and
`.mcp.json` in that project. Only Paperhands entries change. Unrelated settings,
TOML comments, approval choices and inbound policy are preserved; changed original
configuration bytes are backed up privately outside the project. Conflicting
transports or malformed settings are refused. `setup --dry-run` previews changes;
`setup --home DIR` supports isolated user homes. Configure custom harness homes
explicitly before relying on these default-location registrations.

Every invocation repairs the bundled skill at `~/.agents/skills/paperhands`.
Codex and Claude discover that same directory through links under their `skills`
directories; existing unrelated skill directories are reported as conflicts and
retained. Claude exposes it as `/paperhands`; Codex can select `$paperhands`.
`PAPERHANDS_SKILLS_DIR` isolates skill refresh and skips automatic global setup
during init; explicit `setup` still edits its selected home. `init --no-setup`
skips MCP registration. Existing clients may need to reconnect/restart to discover
new tools/skills. Configured MCP is not proof of a live client connection.

In global MCP connections, pass the project's explicit `--root STORE` on research
operations if the server's working directory differs from the agent's project.
Project registrations bind `mcp --project-dir DIR` to avoid that ambiguity. Moving a
project requires refreshing its project registration; its stable project ID remains.

## The harness assembles the loop

Read the user's research question, define controls and budgets, then use the real
CLI/MCP operations. The binary supplies state and instruments; the harness reasons
about hypotheses and experiments. Keep useful failures and questions. A schedule
does not grant new account, spending or collector-control permissions.

```sh
paperhands init --name 'Queue research' --research 'Which mechanism replenishes the queue?'
paperhands ask 'What should I do next?'
paperhands automation bind
paperhands automation add --id desk-review --every 900 --duration 86400 \
  --max-wakes 48 --message 'Inspect closed evidence, review open questions and record the cheapest next test.'
paperhands automation tick --dry-run
paperhands automation install --dry-run
paperhands automation install
paperhands automation status
```

These commands create a finite schedule; adapt its lifetime and wake budget to the
operator's research mandate. The first slot is immediately due, then every declared
interval. Cadence is elapsed UTC milliseconds, not calendar cron semantics or market
time. Cron checks once a minute. Delayed ticks coalesce to the latest slot, with at
most four wake attempts per tick, serving least-attempted schedules first. A schedule
permits 1–10,000 wake attempts, interval
60 seconds–7 days, and lifetime at most 30 days. There are at most 128 schedules per
store. Cancellation uses `automation cancel --id ID --reason TEXT` and retains data.

Status includes `next_due_at_ms` (the nominal UTC slot boundary), `next_due_reason`
and `remaining_attempts`. The boundary is not a promised cron poll or model start.
Spent or ambiguous slots are skipped. Reasons are `due`, `waiting`, `cancelled`,
`expired`, `budget_exhausted` or `expires_before_next_due`; the last four have no
next due time. Expiry is exclusive. Remaining attempts count unused budget even
after expiry/cancellation and do not authorize more execution.

## ICQ owns socket delivery

Install/start ICQ with its documented skill first. `automation bind` selects the
calling harness's own public identity, or an explicit `--recipient PUBLIC_KEY`.
It creates a separate standalone sender for this project's automation. It never
borrows a harness key, scans other sessions or starts a new model. Bindings retain
absolute local store/runtime references and must not be copied into another live
instance. Rebinding retains an immutable binding revision and changes the default
for new schedules. Existing schedules keep their original pinned recipient: cancel
them and register fresh schedules to move recurring work to a replacement harness.

Ticks call the pinned ICQ executable using exact argv and its explicit home/sender.
The intended ICQ mutation is previewed before execution. ICQ uses its existing Codex
`thread/queue/add` protocol or Claude messaging socket. Claude's effective
`crossSessionInbound` and project/managed policy may hold/refuse delivery; Paperhands
does not weaken those settings. Headless receivers need the documented ICQ launch
setup, including acceptance where authorized; no `--bare` receiving Claude.

Each due slot is exclusively claimed and sealed before invoking ICQ. Receipts keep
the ICQ message identity/state separately from receiver acceptance and model action.
An interrupted/uncertain slot remains spent and is never automatically replayed.
Later scheduled slots are separate authorized wakes. `recipient-stored`, native
socket readiness and Codex admission are not evidence that research completed.
Use an explicit task decision/result receipt to establish work performed.

`automation tick --dry-run` inspects without sending; ordinary CLI skill maintenance
still applies. Every subprocess has a ten-second deadline and 64 KiB output bounds.
Schedules, cancellations and intent/result receipts are local private control state.
They are not imported research executions or shared-capital portfolio accounts.

Failed ICQ/cron commands expose private `error.diagnostics`: operation/phase,
child exit, timeout/output-limit flags and bounded stderr (at most 2,048 characters).
ICQ JSON contributes its error message, excluding raw stdout/navigation payloads.
Failed wake receipts retain diagnostics separately from delivery/work claims.
Inspect the cause; identity-capacity errors do not trigger key deletion, alternate
homes or automatic retries. Failed cron read/recheck refuses before writing.

## Cron ownership

`automation install` reads the actual crontab, preserves other jobs and edits one
marked block for this canonical store path. It uses an absolute Paperhands command
and store, checks concurrent observed edits, retains a private backup and verifies
the installed text. Percent/newline paths and malformed/duplicate markers refuse.
`automation uninstall` removes only that block; evidence and schedules remain.
There is no LaunchAgent replacement, existing collector restart, paid model fallback
or unbounded headless optimizer. Native cron is required on this release; unsupported
platforms or host permission failures remain explicit. Inspect ICQ readiness and
retained wake receipts after installation; cron registration alone proves no delivery.

Implementation references: ICQ `src/harness.rs` provides native admission and uncertain
outcomes; `src/native.rs` limits discovery to the calling harness. Native MCP formats
follow [OpenAI's MCP configuration](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
and [Claude's MCP scopes](https://code.claude.com/docs/en/mcp).
