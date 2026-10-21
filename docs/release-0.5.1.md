# Paperhands 0.5.1

This patch carries the three fixes reviewed in source as `85afeb5`, `70fd115` and
`16ada06`. Source/retained-receipt review is separate from acceptance of the exact
registry artifact.

- Bounded ICQ/cron failures expose private structured diagnostics, including child
  exit, timeout/output-limit flags, operation/phase and bounded stderr. ICQ JSON
  contributes its error message rather than raw stdout or navigation payloads.
  Failed cron reads/rechecks refuse before writing; ambiguous wakes remain spent.
- Schedule status adds nominal `next_due_at_ms`, `next_due_reason` and unused
  `remaining_attempts`. Nominal due time is not a promised poll/model start.
  Expiry is exclusive; unused budget after expiry/cancellation is not eligibility.
- Prepared memory/task/graph readers use existing read-only lock descriptors and
  retain shared locking. Writers retain read/write exclusive locks. Missing locks
  remain gaps. Managed skills need a separate writable location; collection reads
  do not promise one atomic whole-project snapshot.

No store/schema migration, schedule recreation or frozen-binding rewrite is
required. Existing experiment/judge implementations and shared dependency pins
remain unchanged. No work-review-reference API, timer migration, new provider call
or collector operation is included.

The macOS regression now observes the child's exact read-only lock descriptor
before checking liveness under the held exclusive lock. This removes the former
startup-delay-only observation; it does not claim every kernel interleaving.
Bundled guides and the managed skill describe the patch behavior, and prepared
semantic-help sources are regenerated from them.

`cargo test --locked` passes all 54 tests (16 unit, 10 agent, 8 automation, 5 CLI,
4 judge and 11 lifecycle), including the native macOS write-denial/readiness
regression. Strict all-target Clippy, format, diff and managed-skill validation
pass. The regenerated help index retains 243 sources and five companion routes.
Tests use owned temporary projects and local provider fixtures; no real provider,
collector or native timer operation is performed.

The packaged tree also passes all 54 tests with locked offline resolution, and
`cargo publish --dry-run --locked` verifies its build. `cargo publish --locked`
completed successfully. An independent registry download matches the tested
archive exactly; all 59 source payloads match the clean source.

- Published version: `0.5.1`, publisher **bboxy450 / user 445014**, unyanked.
- Clean packaged VCS revision/tag `v0.5.1`:
  `f48eabca9afdfbcab112af02fafbadc43878fa38`.
- Registry archive: 929,531 bytes, SHA-256
  `67d309a2da80efa239fd9a1664c78416e1a40ea5994e85a62bcfa215e6e7533b`.
- Exact shared pins remain `clap-how-to =0.1.4` and
  `pocket-encoder =0.1.1` with search.

Availability does not replace a consumer's accepted global executable pin. The
builder left the global 0.5.0 executable unchanged; each consumer independently
verifies and adopts the new registry artifact. No native timers were changed.
Private commands, logs and registry/package verification remain outside Git.

Winnow subsequently accepted and activated the exact registry release at consumer
main `af8599d7bec57dba7385fad35c4b9ae1cf7b4eee`. Its Cargo install completed
successfully; the active global binary is 29,602,528 bytes, SHA-256
`87479b1f737c92a2dd87ad93fbaa8fcc12dd965907325b81a6efa960ea2534db`,
and the managed skill matches the package. This builder independently checked that
binary hash, the consumer's downloaded archive, retained test logs and selected
CLI/MCP receipts; it did not rerun the consumer suite.

The retained evidence records 88 consumer tests and nine final status checks
passing, bounded ICQ/cron failure diagnostics, a spent repeated fixture wake and
19 installed CLI probes plus three MCP responses. Actual project-write-denied
reads return 805 notes / 9,028 passages without gaps. Final refresh records
828 notes / 9,323 passages with all 868 prior objects unchanged. Original settings,
cron and four LaunchAgents remain unchanged according to the retained checks.
Historical lifecycle failures/reviews are preserved; this acceptance adds no new
financial result, market run or provider call. Existing consumer native-research
format failures remain recorded separately and are not patch regressions.
