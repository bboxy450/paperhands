# Local experiment lifecycle

Paperhands 0.2 executes trusted external evaluators over retained, closed inputs.
It preserves evidence and explicit researcher decisions. It does not supply a
strategy predictor, venue connection, paper-account simulator or financial truth
engine. Existing evaluators own their data interpretation, economics and diagnostics.
MCP, ACP and app rendering remain proposed in repository design notes.

## Prepare and validate

Generate a fresh synthetic example using the installed binary:

```sh
python3 examples/make_fixture.py --paperhands "$(command -v paperhands)" --out ./fresh-fixture --members 200 --include-failures
paperhands experiment validate --plan ./fresh-fixture/plans/policy-000.json
paperhands experiment create --root ./store --plan ./fresh-fixture/plans/policy-000.json --dry-run
```

The plan uses `schema_version: 1`, a fresh `id`, a `question`, and an explicit
`data_use`: development, validation, prospective-paper or unknown. These labels
are declarations, not proof of an untouched holdout. Read or inspected data must
be described honestly. No command executes during validation/dry-run; no experiment
identity is reserved. Normal automatic managed-skill refresh still applies.

`bundles` names retained trees with roles `input`, `evaluator`, `accounting`,
`config` or `candidate`, absolute source/inventory paths and the inventory's SHA-256.
Input, evaluator and config roles are mandatory; input inventories require a
declared closing-evidence member. Its domain meaning remains the evaluator's job.
All members, including raw failures and empty directories, are retained and hashed.
Missing data stays missing. Inventory seal time is local time, never venue time.

`external_files` pins dependencies that remain outside the snapshot. Every command's
absolute executable must be pinned with role `executable`; its resolved target and
bytes are recorded. This expressly permits venv executable symlinks; inventory tree
members and ordinary dependency files cannot be symlinks. Declare external provenance
referenced by original manifests. Byte relocation does not make those references
self-contained or rewrite historical manifests.

`commands` is an ordered array of `{argv, cwd}`. No shell is inserted. The templates
`{snapshot:NAME}`, `{work}` and `{output}` resolve to the attempt's exact paths.
`cwd` is `{work}` or an exact snapshot reference. The environment is cleared and
replaced by explicit plan `env` plus `PYTHONDONTWRITEBYTECODE=1`; actual argv and env
are retained. Commands and their children are trusted code. Path checks and process
groups are not a security sandbox; networking, arbitrary descendant execution and
writes outside declared output are not isolated. Use an external sandbox when needed.

Declare `required_artifacts` relative to output and optionally `cases`, which must
be one required artifact. `limits` requires `wall_time_ms`, `cpu_time_seconds`,
`stdout_bytes`, `stderr_bytes`, `artifact_bytes`, `artifact_files`. Version 0.2 uses
Unix process groups: wall time covers command preparation and observed completion;
initial dependency checks and final integrity hashing add measured overhead.
An expired preparation budget prevents spawning. Completion observed after the
deadline is timed_out even if the child exited zero while the runner was stalled;
the actual exit remains recorded separately. CPU and individual file-size limits use inherited OS rlimits
per child process, not aggregate process-tree quotas.
Stdout/stderr retain bounded prefixes per command. Aggregate artifact size/count is
sampled every few milliseconds, so a write can overshoot before termination. No RSS
limit is claimed. Record actual OS child CPU and peak RSS, not guessed utilization.

## Freeze and execute once

```sh
paperhands experiment create --root ./store --plan ./fresh-fixture/plans/policy-000.json
paperhands experiment run --dir ./store/experiments/policy-000 --dry-run
paperhands experiment run --dir ./store/experiments/policy-000
paperhands experiment inspect --dir ./store/experiments/policy-000
```

Creation exclusively reserves the identity and copies data into shared
content-addressed snapshots. Multiple attempts use one identical snapshot instead
of copying a dataset per policy. Changed source bytes/member sets, unsafe relative
paths, special files and links refuse verification. Verify the frozen snapshots
and external dependencies before and after execution. Original source edits after
freezing do not change the authoritative snapshot; external files still in use are
rechecked. SHA receipts detect changed records; they are not signatures against an
attacker able to rewrite the entire store and all receipts.

The start record spends an identity permanently. A file lock rejects concurrent
execution. Each step records real PID/process group, exact argv/environment, exit
or signal, wall/CPU/RSS measurements and stdout/stderr hashes. Timeout or resource
exhaustion kills only that fresh owned process group. Retain crashes and partial
artifacts. An interrupted runner is never silently resumed, and Paperhands never
resets Git, deletes an attempt or kills an unrelated session. Escaped child groups
are outside the trusted-command boundary and need separate supervision.

Process states include planned, queued, running, command_succeeded, failed,
timed_out, resource_limited and interrupted. Evaluator evidence and researcher
review remain separate. Successful exit does not mean valid financial evidence.
Inspecting the result verifies its retained output/process inventories; later
mutation refuses comparison/export. Missing required artifacts mark evidence invalid.

## Bounded batches

Create each fixture plan into one store, then run its batch:

```sh
for plan in ./fresh-fixture/plans/*.json; do paperhands experiment create --root ./store --plan "$plan"; done
paperhands batch run --root ./store --plan ./fresh-fixture/batch.json
paperhands batch inspect --dir ./store/batches/fixture-batch
```

A batch declares `schema_version`, fresh `id`, distinct `attempts`, `max_parallel`,
`wall_time_ms`, `total_output_bytes` and `total_artifact_bytes`. It supports up to
4096 members and 1..256 workers. Those are implementation bounds, not a capacity
promise. Every planned member gets a queued record and a final success, failure or
not_started_budget result. A spent/reserved member or batch cannot be restarted.
Totals include measured wall/CPU/output/artifact use and maximum child-reported
RSS; this is not simultaneous batch RSS. Failures do not erase companion results.

`batch inspect` preserves the historical completion record and rechecks every
member's original plan binding, queue owner, receipt, frozen dependencies and
retained output/process inventories. Successful verification adds
`integrity_status: verified` and `integrity_checked_at_ms` to its response; review
fields in the completion record remain historical. Changed or unverifiable member
evidence refuses inspection with JSON error/exit 2 naming the member. Original
batch/member receipts are never rewritten. An unfinished batch reports integrity
as `not_checked` rather than asserting that active outputs are immutable.

This is a worker pool for supplied evaluators. A process may evaluate many logical
policies; Paperhands does not invent separate account ledgers or infer independent
evidence from 200 names. Shared-liquidity scenario totals are not a funded portfolio.

## Compare evidence

```sh
paperhands experiment compare --baseline ./store/experiments/policy-000 --candidate ./store/experiments/policy-001 --out ./comparison.json
```

Comparison requires identical fixed input/evaluator/accounting/config snapshots,
external pins and data-use labels. Candidate-role bundles and argv may differ.
Normalized case files have `schema_version:1`, an `accounting` identity, `units`
mapping metric names to units, `cases`, `diagnostics` and `limitations`. Each case
has a unique `id`, `group`, a complete `metrics` map, and raw `context`.
Metric values are finite JSON numbers or explicit nulls. Null means unknown,
never zero. Missing keys, duplicate IDs, different units, malformed/nonfinite
values and incompatible fixed inputs are refused with a retained failure receipt.

Results preserve exact common, baseline-only and candidate-only objects, raw
diagnostics, nulls and context differences. Deltas use ordinary floating arithmetic;
retain exact financial decimal strings and authoritative calculations in supplied
diagnostics/context. An adapter defines common identities; matching anchors can have
different quantities/cash. Comparison does not independently reconstruct fills,
validate conservation, group economic aliases or establish chain truth. The Winnow
example supplies these domain artifacts and explicit unsupported cases separately.
No improving scalar automatically selects, promotes or deploys anything.

## Closed consumer example

`examples/winnow_closed.py` reads a consumer-supplied reference JSON, closed
capture, existing source checkout and existing interpreter. It verifies declared
source hashes and commit, copies Python source into a fresh Paperhands staging
directory, pins the original external provenance and seals four shared bundles.
It never edits the consumer or runs a collector. Supply local paths explicitly:

```sh
python3 examples/winnow_closed.py --reference "$CONSUMER_REFERENCE" --python "$CONSUMER_PYTHON" --paperhands "$(command -v paperhands)" --out ./fresh-winnow
paperhands experiment create --root ./winnow-store --plan ./fresh-winnow/plans/baseline.json
paperhands experiment create --root ./winnow-store --plan ./fresh-winnow/plans/candidate.json
```

Each plan executes the existing `research.passive_exit_miner` with an exact argv
array, strict RPC receipt guard, taker exit and 60-second evaluator budget. Only
the candidate adds `--improve-entry`. A second command normalizes supplied case
anchors using `examples/winnow_cases.py`. Both preserve raw catalogs, parameters,
audit, report, complete episodes, Decimal strings, unknown holdings, costs,
economic signatures, event concentration and raw stream/RPC bindings. Baseline
offset1 sensitivity labels stay in the raw catalog outside the offset0 comparison.
Actual opposite actions remain supplied transactions; no inverse is synthesized.

Execute each created attempt inside an externally enforced offline sandbox, then
use `experiment compare`. On macOS a local network-denial wrapper is:

```sh
sandbox-exec -p '(version 1)(allow default)(deny network*)' paperhands experiment run --dir ./winnow-store/experiments/winnow-baseline
sandbox-exec -p '(version 1)(allow default)(deny network*)' paperhands experiment run --dir ./winnow-store/experiments/winnow-candidate
paperhands experiment compare --baseline ./winnow-store/experiments/winnow-baseline --candidate ./winnow-store/experiments/winnow-candidate --out ./winnow-comparison.json
```

These are posthoc development inputs. This example reproduces consumer evidence;
it does not supply an untouched holdout, independent financial audit, live trading
engine or new strategy predictor. Python runtime libraries remain dependencies
of the pinned executable; the runner does not package an entire Python runtime.

## Review and export

```sh
paperhands experiment review --dir ./store/experiments/policy-000 --disposition retire --note ./private-review.txt
paperhands experiment export --dir ./store/experiments/policy-000 --out ./fresh-public-summary
```

A final immutable private review uses continue, revise, nominate, inconclusive or
retire plus an evidence/revisit note. It is never an automatic deployment. Public
JSON/Markdown export requires that review and binds the result hashes. Its fixed
allowlist includes lifecycle enums, hash identities and numeric run metadata only.
It excludes raw notes, experiment labels, argv, paths, environments, sessions,
messages, keys, metrics and evaluator diagnostics. Keep financial publication in
the consumer's existing reviewed/sanitized publisher. Exports never upload data.

Exit 0 means the requested operation completed; inspect the separate process and
evidence fields. Refused input/schema/integrity operations exit 2 with JSON stderr.
Completed runs/batches containing failures or missing artifacts exit 3 with retained
JSON results on stdout and `ok:false`. Startup/skill/encoder runtime errors exit 1.
No ICQ messaging, cron changes, live orders, paid inference or background optimizer
is invoked by these commands.
