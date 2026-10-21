# Paperhands 0.7.1

Published to crates.io on 2026-10-21 at 21:18:26 UTC by **bboxy450**
(user 445014). The downloaded registry archive matches the approved candidate
byte-for-byte. Consumer installation and acceptance remain separate.

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

## Verification and published artifact

All five source gates exited 0: prepared-help generation (273 sources and five
companion routes), formatting, strict all-target Clippy, `cargo test --locked`
(77 passed, zero failed or ignored), and `cargo publish --dry-run --locked`.
The Oracle independently repeated those gates on an isolated checkout and fresh
target directory before authorizing upload. These checks include the repeated
skip file bound, retained receipt bytes, terminal status/report separation,
final-budget wake behavior, report clocks, public field parity and stdio MCP.

- [crates.io 0.7.1](https://crates.io/crates/paperhands/0.7.1): publisher
  `bboxy450` / user `445014`, unyanked when independently checked.
- Registry archive: **1,015,754 bytes**, SHA-256
  `4384fe02eb946bc63547d0c828bd3897a0e35c2bb9ad135d2289ae4c67cc7152`.
- Clean packaged source and signed tag `v0.7.1`:
  `df5507815654e946283123f2b617ceec293062ea`. Tag object:
  `1eca4755aceefe28abdea75dbda114aadadfa15b`.
- All 63 original payload files match the approved source. Normalized dependency
  pins remain clap-how-to `=0.1.5` and pocket-encoder `=0.1.2/search`.
- [GitHub release and signed package](https://github.com/bboxy450/paperhands/releases/tag/v0.7.1)
  contains `paperhands-0.7.1.crate` and `paperhands-0.7.1.crate.asc`. Both downloaded
  assets match the uploaded files; the package also matches the registry bytes.
- The tag and artifact are signed by
  `9B07E849480C68328CA20684844ECC0587DE7AB0`
  (`Boxy <bboxy450@gmail.com>`). The detached signature verifies the independently
  downloaded registry package and the downloaded GitHub package.

```sh
gpg --verify paperhands-0.7.1.crate.asc paperhands-0.7.1.crate
```

Before upload, the authenticated registry endpoint bound to user 445014 returned
HTTP 200 both before and after `cargo login`, verifying the approved identity.
No credential value was printed or included in source. Packaging, artifact/tag
signing and verification, tag push and `cargo publish --locked` exited 0.
Cargo reported that the unchanged locked `tokio 1.54.0` was yanked; package
verification and upload still completed successfully. No dependency was changed
during publication.

The initial combined GitHub creation/upload command exited 1 on a duplicate
signature-asset response; subsequent reads found no retained release. Creating
a draft, uploading each verified asset separately, and publishing it exited 0.
The final public release and both downloaded assets were independently checked.

The release announcement was sent to `#bboxy450-crates` after registry and
signature verification in the publishing turn, receipt
`47a77cf7-b1f2-4884-b9f2-8ef48ca4ac69`, with signing suffix `844ECC0587DE7AB0`.
That receipt proves local storage and scheduled forwarding, not model receipt.
No global binary was installed; Winnow adoption follows its separate brief.
