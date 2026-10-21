# Consumer feedback

Winnow's original 0.2.0 integration is retained at
`dacc3d07b6ee2c850292e8a5dd4307e6d21278bb`. Its current main accepts the exact global
0.5.0 registry artifact at `ad6b0c5d6cec91a8303a757a76162774d8ea7def`, following
0.3.1 and 0.4.0 adoption. The consumer records 83 passing integration tests;
retained receipts verify all 59 packaged source payloads, the exact installed
binary, network-denied MCP retrieval over 672 notes / 7,337 passages and 670 prior
note/task/graph files unchanged. Its 121,064-path archive is a per-file snapshot,
not proof of capture closure or a transactional database backup. Existing lifecycle
implementation is unchanged from accepted 0.4; prior replay evidence is carried
forward rather than counted as another experiment. These are integration results,
not financial validation.

- **Desk shape:** `desk` returns a view envelope. Summary fields are under
  `data.desk`; datasets and graphs remain beside that summary. Consumer adapters
  should inspect the schema and select that explicit path.
- **Source revisions:** notes retain at most 16 KiB of exact UTF-8. Winnow currently
  supplies lossless partitions with full-source hashes, byte ranges and stable
  citations, then reuses existing note IDs on refresh. A native idempotent source
  revision/import operation would reduce adapter bookkeeping.
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
- **Automation diagnostics (unreleased source fix):** Winnow reports that 0.5.0
  hides ICQ's identity-capacity error and native cron subprocess details. The
  source fix adds `error.diagnostics` and failed wake-receipt `diagnostics` with
  operation/phase, child exit, timeout/output-limit flags and at most 2,048
  characters of stderr. ICQ JSON contributes its error message, excluding navigation
  payloads and raw stdout. Failed/unknown sends remain spent; failed cron rechecks
  refuse before writing. Isolated fixtures cover the reported failures without
  changing native jobs. The accepted global binary is unchanged; no release or
  consumer timer migration is implied.

Paperhands owns its project store, research DAG, provenance, companion and MCP
interface. It uses shared encoder/help crates and borrows useful Ontology
patterns without waiting for a separate live-datapack runtime. Narrow decision-model
eval instruments shipped in 0.4; a reviewed consumer label corpus and actual judge
accuracy remain open. Source/score annotations must distinguish supported,
contradicted and insufficient evidence; one event or one synthetic batch cannot
establish calibration. The companion remains offline.
