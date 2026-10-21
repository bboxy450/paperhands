# Paperhands 0.8.0 candidate

This is an unpublished candidate. This document does not establish completed
source gates, package verification, release approval, signatures, publication or
consumer adoption.

`paperhands zoom --root ROOT` describes patterns in retained research state and
returns `paperhands.zoom/v1`. Its five signal kinds concern repeated refusal,
unreviewed output, concentrated retirements, no newly retained note/experiment
activity, and spent wakes without recorded task-state changes. The result echoes
the thresholds it applies and retains evidence references, gaps and limitations.
Unknown measurements remain null rather than becoming zero activity.

`--since MS` is an inclusive absolute UTC epoch-millisecond boundary and defaults
to the preceding 48 hours. It bounds blocked-task, retirement, wake and task-state
events; unreviewed backlog and freshness inspect all verified retained history.
The CLI exposes refusal-count, unreviewed-age,
retirement-count, cosine, recording-age and spent-wake thresholds; their defaults
are 3, 86,400,000 ms, 5, 0.65, 172,800,000 ms and 2 respectively. The returned
thresholds and fixed rule identify what was applied. The CLI guide gives the exact
flag names and signal sources.

Refusal counts require the same task ID and identical nonempty blocked-event
note; lifecycle errors remain separate counts. Imported notes never become native
refusal or review evidence. Retirement concentration requires explicit
graph/provenance mapping to clustered notes, without embedding prose to invent
that relationship. It tests the most recent configured number of retirements in
the window for links into one semantic cluster. Fewer known retirements than the
count threshold do not fire concentration. At or above that count, missing required mapping makes the signal
null and coverage incomplete, forcing a null verdict. Empty native note and
experiment history also leaves freshness age and verdict null; there is no
filesystem/import-clock fallback.

Unreadable or incomplete required collections force a null verdict. Otherwise
the fixed rule reports stalled for repeated refusal, unreviewed output at least
twice its age threshold, or spent wakes without task changes together with no
new recorded evidence. Any other fired signal gives drifting; otherwise moving.
The response names the applied rule as `paperhands.zoom.verdict/v1`.

Imported Winnow reports and other prose cannot supply native blocked events,
execution/review states or retirement mappings merely by being imported. Missing
native semantics remain unsupported; null is not an all-clear result.

The verdict is `stalled`, `drifting`, `moving` or null. It is a description of
supported signals, not a decision. The questions ask the researcher to reconsider
the premise; the binary does not create tasks, retire experiments, promote a
strategy or learn thresholds from outcomes. A new retained note is not necessarily
new independent market evidence, a semantic neighborhood is not a causal strategy
family, and a wake receipt does not prove completed work.

Zoom reads retained state and starts no model, evaluator, wake or trading action.
Its human output gives a compact verdict, signal and question summary. For
next-work questions, the grounded companion surfaces a supported stalled verdict
before its routine frontier, followed by the same four questions as the managed
skill's ZOOM OUT section: what changed, whether this gate already refused once,
what output remains unreviewed, and whether recent retirements share one
neighborhood. The skill asks the harness to state what it saw before any loop
step and run `paperhands zoom`; this guidance grants no automatic action.

`automation add ... --zoom-prompt FILE` retains an optional exact UTF-8 brief in
the private sealed schedule. For one already-due, non-dry wake, a stalled verdict
substitutes that brief inside the existing wake wrapper. The original slot,
intent, budget and single send are retained; no additional wake is scheduled.
The receipt records `zoom: {verdict, signals_fired}` separately from delivery and
completed-work claims. Existing schedules without this option keep their routine
message behavior.

Inspection errors or null verdicts preserve the routine wake message and retain
the unknown verdict/error in its private receipt. The prompt is frozen at schedule
creation from 1..8,192 UTF-8 bytes containing a non-whitespace character and no NUL;
later file edits do not alter the retained brief. The hook uses the default zoom
window and thresholds. Schedules with the new option require 0.8 readers; upgrade
every writer and reconnect only owned resident MCP processes before opting in.
Older readers refuse the added field; existing schedules without it stay readable.

The public report adds a sanitized zoom summary: verdict, fixed signal-kind names
and generation time. Private reasons, evidence IDs, prompts and source diagnostics
do not enter that public projection. The report allowlist remains the publication
boundary; emitting a report uploads nothing.

Public report readers must use `frontier.catalog_fully_paginated`. Version 0.8
removes the deprecated `frontier.all_evidence_inspected` alias after its promised
single compatibility release in 0.7.1. The field still means execution-catalog
pagination, not source review or financial validation. Private desk internals are
unchanged.

Zoom inspection is bounded to 1,024 experiments, 4,096 task revisions, 4,096 wake
claims, 1,024 clustering notes and 8,192 graph links, alongside existing memory limits. Unknown
coverage is disclosed; no silent sampling or cross-collection transactional
snapshot is claimed. Normal skill maintenance remains separate from these
read-only project operations.

Existing research objects, note IDs, frozen execution bindings, schedules,
recipients and spent slots require no rewrite for the optional hook. Dependency
pins remain `clap-how-to =0.1.5` and `pocket-encoder =0.1.2` with search. The 0.2
help-parser adoption and memory schema migration remain deferred.
