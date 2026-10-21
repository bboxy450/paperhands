# Paperhands 0.7.0 candidate

This is an unreleased candidate. Source checks, package verification, Oracle
release approval, registry publication and consumer adoption are not established
by this document.

`paperhands report --root ROOT --out DIR` writes `report.json` with schema
`paperhands.report/v1` and matching `report.md`, both marked `public-allowlist`.
The directory is created if missing. Each file replaces its predecessor by an
atomic rename; the two-file set is not an atomic snapshot.

The report exposes aggregate frontier and memory-coverage state, up to 100
sanitized task titles/state/readiness rows, up to 200 reviewed lifecycle summaries
passing the existing export binding checks, and finite automation status. Rows
are newest first. Unreviewed attempts are omitted. Gap counts, unknown inspection
state and limitations remain explicit; a report is not a new review, execution
receipt, validated trading policy or financial result.

Memory coverage is a public projection of counts, bounds and declared digests;
private volume names, directories and rollover commands never enter it. Unknown
memory or task totals refuse generation. The shared experiment export projection
also checks that a review binds the current plan and cannot refer to a missing
result. Existing valid reviews and exports require no rewrite.

The allowlist excludes raw notes/plans, argv, private paths, messages, keys,
recipient identities and task/project/experiment/schedule IDs. Titles remove
URLs, paths, long hexadecimal strings and token-like text, then cap at 120
characters. A final serialized-output gate refuses surviving private-looking
data. Hashes appear only in declared `*_sha256` fields.

`automation add --report-out DIR` persists an optional target. Non-dry ticks
refresh missing reports or those older than one hour, retaining a private
`written`, `skipped` or `failed` report receipt with its reason. Status and dry-run
ticks do not publish. Existing schedules without that target keep their behavior.

MCP uses the same command through `paperhands` with an exact argv array. Report
generation does not upload content, call a provider or start an evaluator. The
consumer owns dashboard rendering, publication and exact-artifact acceptance.
Existing store identities, memory volumes, frozen execution bindings, recipients
and spent wake slots require no migration for this optional output.

Candidate source verification passes all 75 tests with `cargo test --locked`,
including public schema/redaction/refusal, reviewed failures and binding checks,
newest-task caps, Markdown equality, real stdio MCP parity and scheduled report
freshness/failure receipts. Strict all-target Clippy, formatting and managed-skill
validation pass. Prepared help contains 269 sources and five companion routes.
The requested selected-store report also succeeds under native network and
project-write denial. These checks establish neither publication nor consumer
acceptance of a released 0.7.0 artifact.

The coordinated dependency addendum pins `clap-how-to =0.1.5` and
`pocket-encoder =0.1.2` with search, resolving a single encoder. Both registry
archives were independently downloaded and matched to their announced SHA-256,
unyanked status and bboxy450 publisher. The upstream patch selects a bounded F32
backend for generic Linux ARM64 to avoid unused FP16 compilation; other targets
retain Candle. This candidate was checked on macOS, without claiming independent
Linux execution or cross-platform vector byte equality.

All six model assets, the full model identity and the search index API remain
unchanged. Both prepared-index freshness tests pass against the retained pair;
no memory migration or existing-note reindexing is performed.

The Oracle deferred `clap-how-to 0.2.1` after a compatibility check: schema-v1
memory verifies its retained passages against Markdown extraction
(`src/memory.rs:143–175`). The 0.2.1 parser changes frontmatter and example
chunking, so a direct upgrade can reject valid existing notes. No local legacy
parser is vendored. Adoption requires a separate memory schema-v2 migration;
0.7.0 keeps the verified 0.1.5/0.1.2 pair and existing note bytes/IDs unchanged.
Re-running prepared generation after correcting the bundled dependency
description produces byte-identical 269-source help and five-route companion
assets; their full model identity remains unchanged.

The companion now answers gracefully when a configured project store is absent,
while existing stores retain identity checks. CLI tests use a temporary working
directory and isolated workspace environment; the missing-store regression no
longer depends on an ignored checkout store. Neither querying an absent store nor
reading an existing one initializes it.
