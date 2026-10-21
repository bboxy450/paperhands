# Paperhands 0.7.1 candidate

This is an unpublished candidate. This document does not establish completed
checks, package verification, release approval, publication or consumer adoption.

Scheduled public report checks stop retaining a sealed pair for every fresh-report
skip. Written and failed refreshes retain their private JSON/SHA-256 receipt pairs;
a skip replaces the single mutable `automation/reports/latest-skip.json`. That file
is one last-skip diagnostic across all targets, not a history of all skips. Skip
tick rows use `latest_skip_sha256`, not `receipt_sha256`. Historical receipts remain
unchanged, including skipped receipt pairs written by 0.7.0. No retention deletion
or rollover is added.

Non-dry ticks select `--report-out` targets from `due` and `waiting` schedules at
their start-of-tick inspection, before wake processing. Schedules already
cancelled, expired or budget-exhausted then supply no target. A selected target
remains selected if the same tick's last due wake spends the final wake budget.
Missing reports and reports older than one hour retain the existing refresh rule.
Status and dry-run ticks do not publish reports. No new flags, wake permissions or
model/evaluator work are introduced.

`recorded_at_ms` is taken after the check or write returns, before the receipt or
status is persisted. `generated_at_ms` carries the report's own generation clock
and is null on failed generation; a known previous report clock remains separate
in `previous_generated_at_ms`. These clocks do not establish source review or
completed research.

The public `paperhands.report/v1` frontier now uses `catalog_fully_paginated` for
the selected root's execution-catalog pagination state. The value covers inclusion
of experiments, batches and judge runs in the inspection page; it does not assert
successful integrity checks, substantive source review or financial validation.
For 0.7.1 only, `all_evidence_inspected` remains a deprecated same-value alias.
Report consumers should use the canonical field and continue checking gaps and
explicit reviews separately.

The source changes address Winnow's report-receipt observations at `177ad00b` and
the public pagination wording follow-up. Existing store identities, note bytes,
frozen executions, schedules, recipients and spent wake slots require no rewrite.
The public allowlist and per-file atomic replacement contract are unchanged.

Dependency pins remain `clap-how-to =0.1.5` and `pocket-encoder =0.1.2` with search.
The 0.2 help-parser adoption and memory schema migration remain deferred; no
existing note is reindexed or assigned a new identity.
