# Consumer feedback

Winnow's original 0.2.0 integration is retained at
`dacc3d07b6ee2c850292e8a5dd4307e6d21278bb`. Its current main accepts the exact global
0.5.1 registry artifact at `af8599d7bec57dba7385fad35c4b9ae1cf7b4eee`, following
0.3.1, 0.4.0 and 0.5.0 adoption. Retained logs record 88 system-Python consumer
tests and nine final status checks passing, plus 19 installed CLI probes and three
native MCP responses under network denial. Project-write-denied reads cover
805 notes / 9,028 passages without gaps. The final refresh reaches 828 notes /
9,323 passages in seven consumer query partitions, retaining all 868 prior objects.
The 122,313-path / 9,079,630,940-byte archive is a per-file snapshot, not proof of
capture closure or a transactional database backup. Original closed030 and
200-member fixture inspections retain 824 and 3,610 files unchanged; the batch's
historical exit 3 and unreviewed evidence remain visible. Lifecycle/judge source
is unchanged from accepted 0.5; prior replay evidence is carried forward rather
than counted as another experiment. These are integration results, not financial
validation. See [the patch release evidence](release-0.5.1.md).

- **Desk shape:** `desk` returns a view envelope. Summary fields are under
  `data.desk`; datasets and graphs remain beside that summary. Consumer adapters
  should inspect the schema and select that explicit path.
- **Source revisions:** notes retain at most 16 KiB of exact UTF-8. Winnow currently
  supplies lossless partitions with full-source hashes, byte ranges and stable
  citations, then reuses existing note IDs on refresh. A native idempotent source
  revision/import operation would reduce adapter bookkeeping.
- **Store capacity:** the accepted refresh has 828 notes, below Winnow's 900-note
  guard and the 0.5.1 native 1,024-note limit. A later health receipt verifies
  893 notes / 10,114 passages after 23 normal revision/catalog additions, leaving
  seven consumer slots. Version 0.6 adds explicit append-only memory volumes,
  sealed history, complete read-only retrieval/desk coverage and pre-append
  capacity checks. Adoption remains deliberate: upgrade every writer, preserve a
  verified copy, use returned paths and note-set bindings, and retain per-active-
  volume guards plus the unchanged combined query bounds. This does not authorize
  pruning history or merely lifting the old guard. See [the rollout notes](release-0.6.md).
- **Leading-hyphen text:** separate argv entries `--text`, `-2 ...` are rejected
  by the shipped parser. Use a single `--text=-2 ...` argument or the existing
  `--file FILE` input. An isolated check against the installed 0.5.0 binary
  reproduces the refusal and verifies that equals syntax retains exact UTF-8.
- **Multiple stores:** a read-only desk spanning explicitly selected historical
  stores would help inspect old cohorts without copying attempts, changing their
  identities or treating separate evidence as a combined portfolio.
- **Managed skills:** consumers running historical binaries can set a separate
  `PAPERHANDS_SKILLS_DIR` per version. Winnow does this so an older inspection
  cannot downgrade current shared instructions.
- **Resident MCP processes:** replacing the Cargo executable leaves already
  running servers on their prior code. Before volume opt-in, inspect each owned
  server's responding version/format support, reconnect only owned servers and
  quiesce writers that cannot upgrade. Checking the CLI version alone is insufficient.
- **Global activation:** replacing a shared PATH binary invalidates a consumer's
  pinned executable hash until it accepts the exact new artifact. Winnow correctly
  refused this drift before adopting 0.5. Preserve frozen execution bindings.
- **Background initialization:** ordinary `init` now registers native MCP. Existing
  refresh automation explicitly uses `--no-setup` and preserves harness settings,
  timers and prior objects. New wake schedules must retain recipient bindings and
  distinguish send/storage/admission from model action and research completion.
- **Lifecycle states:** inspection success can contain a failed historical run;
  complete supplied evidence remains `unreviewed`, including after a crash.
  Historical batch receipts, current integrity and current individual dispositions
  are separate. Winnow also checks missing required artifacts after failures and
  retains its attributed normalization adapter because package examples are not
  installed beside the binary.
- **Automation diagnostics (0.5.1):** Winnow reports that 0.5.0
  hides ICQ's identity-capacity error and native cron subprocess details. The
  source fix adds `error.diagnostics` and failed wake-receipt `diagnostics` with
  operation/phase, child exit, timeout/output-limit flags and at most 2,048
  characters of stderr. ICQ JSON contributes its error message, excluding navigation
  payloads and raw stdout. Failed/unknown sends remain spent; failed cron rechecks
  refuse before writing. Isolated fixtures cover the reported failures without
  changing native jobs. Consumer acceptance/activation is separate; no timer
  migration is required.
- **Wake navigation (0.5.1):** schedule status adds `next_due_at_ms`,
  `remaining_attempts` and `next_due_reason`. Due times are nominal UTC slot
  boundaries, not a promised cron/model start. Spent slots are skipped without
  replay; expiry, cancellation, exhausted budget or expiry before the next slot
  yield no next due time. Remaining attempts count unused budget even when the
  schedule has expired or been cancelled; they grant no further execution.
- **Work-review references (design follow-up):** consumers can retain a separate
  review bound to an immutable wake-intent hash. An optional reference/read surface
  could expose its exact reviewer/source/artifact digest and as-of scope without
  changing transport delivery flags. It must distinguish a verified binding from
  the reviewer's claims, retain conflicting reviews, and avoid automatic
  acknowledgment, work-completion claims or experiment promotion. No native
  registration/read surface for those consumer reviews is shipped or added here.
- **Read-only project workers (0.5.1):** 0.5.0 opens memory/task/
  graph lock files with write access even for inspection. Readers now open existing
  lock files read-only and retain shared locking; writers still require write
  access and exclusive locking. A native macOS write-denial regression verifies
  populated CLI/desk/companion reads, real writer contention, unchanged bytes and
  refused writes. Missing locks remain explicit gaps and are never bypassed or
  recreated by a reader. Managed skill installation needs its own writable location;
  this does not promise an atomic snapshot across the three collections.

Paperhands owns its project store, research DAG, provenance, companion and MCP
interface. It uses shared encoder/help crates and borrows useful Ontology
patterns without waiting for a separate live-datapack runtime. Narrow decision-model
eval instruments shipped in 0.4; a reviewed consumer label corpus and actual judge
accuracy remain open. Source/score annotations must distinguish supported,
contradicted and insufficient evidence; one event or one synthetic batch cannot
establish calibration. The companion remains offline.
