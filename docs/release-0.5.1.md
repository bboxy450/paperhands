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

Exact registry identity/checksum is recorded after packaging. Availability does
not replace a consumer's accepted global executable pin.
