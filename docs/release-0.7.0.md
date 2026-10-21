# Paperhands 0.7.0

Published to crates.io on 2026-10-21 at 18:47:03 UTC by **bboxy450**
(user 445014). The registry archive matches the approved source and package
byte-for-byte. Consumer installation and acceptance are separate.

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

Source verification passes all 75 tests with `cargo test --locked`,
including public schema/redaction/refusal, reviewed failures and binding checks,
newest-task caps, Markdown equality, real stdio MCP parity and scheduled report
freshness/failure receipts. Strict all-target Clippy, formatting and managed-skill
validation pass. Prepared help contains 269 sources and five companion routes.
The requested selected-store report also succeeds under native network and
project-write denial. Registry and signature verification are recorded below;
consumer acceptance is separate.

The coordinated dependency addendum pins `clap-how-to =0.1.5` and
`pocket-encoder =0.1.2` with search, resolving a single encoder. Both registry
archives were independently downloaded and matched to their announced SHA-256,
unyanked status and bboxy450 publisher. The upstream patch selects a bounded F32
backend for generic Linux ARM64 to avoid unused FP16 compilation; other targets
retain Candle. These checks ran on macOS; independent Linux execution and
cross-platform vector byte equality were not tested here.

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

## Published artifact and signatures

- [crates.io 0.7.0](https://crates.io/crates/paperhands/0.7.0): publisher
  `bboxy450` / user `445014`, unyanked when independently checked.
- Registry archive: **1,005,429 bytes**, SHA-256
  `74796eca7889f738cd5e34fd36b2ddb6c400e03a09e87198ac5d736ec5da2651`.
- Clean packaged source and signed tag `v0.7.0`:
  `b24187592e2d2d076d75e996476544a43aeb54d6`. Tag object:
  `c35cad07bf633ac9d3503cf594b4ea27605fb65a`.
- All 63 original payload files match the clean reviewed source. Normalized
  dependency pins remain clap-how-to `=0.1.5` and pocket-encoder `=0.1.2/search`.
- [GitHub release and signed package](https://github.com/bboxy450/paperhands/releases/tag/v0.7.0) contains
  `paperhands-0.7.0.crate` and `paperhands-0.7.0.crate.asc`.
- The tag and artifact are signed by
  `9B07E849480C68328CA20684844ECC0587DE7AB0`
  (`Boxy <bboxy450@gmail.com>`). The pre-upload detached signature verifies both
  the independently downloaded crates.io bytes and the downloaded GitHub assets.

```sh
gpg --verify paperhands-0.7.0.crate.asc paperhands-0.7.0.crate
```

Before upload, authenticated owner-only registry metadata for user 445014 returned
HTTP 200, establishing the restored Cargo credential's approved identity.
`/api/v1/me` returned HTTP 403; that response was retained rather than treated as
identity proof. `cargo login`, signed-tag verification/push,
`cargo package --locked` and `cargo publish --locked` all exited 0. No credential
value was printed or included in source.

Release announcement was sent to `#bboxy450-crates` after registry and signature
verification in the publishing turn, receipt
`90cfb762-413d-44db-ba13-5b19283a45bb`, with signing suffix `844ECC0587DE7AB0`.
The receipt records local storage and scheduled forwarding; remote/model delivery
remains unproven. No global binary was installed; Winnow adoption follows its
separate acceptance brief.
