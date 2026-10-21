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

`automation status` reads already-prepared control state using an existing
read-only shared lock. It neither initializes directories nor changes permissions;
missing, invalid or busy state is an explicit error. Writers and ticks retain
exclusive locking. Managed skill refresh still needs a separate writable location
(or an already-current skill); this is project-scoped read-only access.

## Optional public report refresh

`automation add --report-out DIR` retains an optional report target in the
schedule. A non-dry `automation tick` selects targets from
`due` or `waiting` schedules at its start-of-tick inspection, before wake processing.
Schedules already cancelled, expired or budget-exhausted at that inspection supply
no target. A selected target remains selected if the same tick's last due wake
spends the final wake budget. The tick checks each selected target's
`report.json`: it refreshes a missing report or one generated more than one hour
(3,600,000 ms) ago.

Written and failed refreshes retain private sealed JSON/SHA-256 receipt pairs.
A fresh-report skip updates the single mutable
`automation/reports/latest-skip.json`; it creates no new sealed pair per skip.
It is one last-skip snapshot across all targets, not a history of every target or
tick. The skip's tick row contains `latest_skip_sha256`, never `receipt_sha256`.
Historical receipt pairs remain retained, including earlier skipped receipts.
`recorded_at_ms` is taken after the check or write returns and before its receipt
or status is persisted. `generated_at_ms` is the report's own generation clock;
it is null when generation fails. A known previous report clock remains separate
in `previous_generated_at_ms`.
Inspect the report result and its reason; a wake receipt alone does not establish
that a report was written.

`automation status` and `automation tick --dry-run` do not publish reports.
Schedules without a report target retain their existing behavior. For an explicit
refresh, use `paperhands report --root ROOT --out DIR`; the MCP `paperhands` argv
tool shares that operation. Each output file is atomically replaced, but JSON and
Markdown are not an atomic pair. The public allowlist and refusal rules are in
the `cli` guide. These operations do not upload a site, call a provider or start
an evaluator, and a report does not establish review or financial validation.

## Optional premise-review wake text

The 0.8.0 candidate accepts `automation add ... --zoom-prompt FILE`. It retains
the regular, non-symlink file's exact UTF-8 text in the private sealed schedule:
1..8,192 bytes containing a non-whitespace character and no NUL. Later file edits
do not change that schedule's text or become a new runtime dependency.
Schedules containing this option require 0.8 readers. Upgrade every writer and
reconnect only owned resident MCP processes before opting in; older readers
refuse the new field. Existing schedules without it remain readable unchanged.

Only a non-dry, already-due attempt selects zoom text for this hook, using its
default window and thresholds. The tick takes one pre-claim native-history
snapshot under its existing exclusive automation lock. A `stalled` verdict
substitutes the retained premise-review text for the routine message
inside the existing wake wrapper. Other supported verdicts retain the routine
message. The same slot, intent, wake budget and send are used; this never adds a
second wake. Both intent and result retain `zoom: {verdict, signals_fired}`.
An inspection error or null verdict keeps the routine message and records the
null verdict with its private error. It does not retry inspection or add a wake.
Status and dry-run operations do not send either prompt. Existing schedules
without the option retain their message behavior. A waiting schedule does not
select premise-review text or spend a wake merely because a report is refreshed.
Native automation history is bounded to 4,096 claims and 16,384 attempt members;
unreadable, invalid or over-bound history remains unknown rather than empty.

The detector reads retained state; it starts no model or evaluator and creates
no task, retirement or strategy promotion. A stalled signal asks the researcher
to examine the premise within the operator's scope. It does not grant additional
execution, account or spending permission, or prove that the wake reached a model.

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
