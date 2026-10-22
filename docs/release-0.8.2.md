# Paperhands 0.8.2

Released by bboxy450 on 2026-10-22 with the refined PH-5 episode contract.
Source verification and consumer adoption remain separate evidence.

`repeated_refusal` counts the trailing consecutive run of blocked revisions at a
blocked task head, with an identical nonempty decision note. A non-blocked state,
changed note or blank note ends that run. Open and active heads cannot fire.
Reopening then re-blocking starts a fresh episode at one, even if its note exactly
matches an earlier episode. Native sequence order establishes boundaries from all
verified task history before the inclusive `--since` window clips counted events.
The largest eligible current episode remains the signal's active count.

A done or retired head reports only its most recent blocked episode under
`resolved_refusals: {count, evidence}`. The count totals in-window events in those
last episodes across terminal tasks; evidence names their original exact revision
IDs. Resolved episodes never enter the firing count. When task history is
unavailable, both counts stay unknown; resolved evidence is empty rather than
inventing a complete result.

The fixed `paperhands.zoom.verdict/v1` rule is unchanged. Other signals can still
produce stalled or unknown after a task is closed. Lifecycle errors remain
separate. No detector output retires a premise, creates tasks, starts an
experiment or authorizes action. Frozen task chains, source timestamps and exact
evidence IDs are unchanged; this is a derived view.

This addresses Winnow's installed-0.8.1 closed-gate reproduction: four identical
blocked events followed by retirement continued firing the historical-window
signal. Under the new contract they are resolved, while a blocked head with the
same four-event episode still fires. Two blocked events followed by open and one
new blocked event count as one. Note changes and blank notes also prevent older
episodes from reappearing.

Regression fixtures cover both terminal states, reopened episodes, changed and
blank notes, window clipping, mixed eligible/resolved scopes, exact citations and
unknown history. Deliberate fixture transitions append events; inspections retain
all original bytes.

No store migration is required. CLI, stdio MCP, companion and public summaries
use the same detector. Existing schedules, recipients and spent slots are
unchanged. Dependencies remain `clap-how-to =0.1.5` and `pocket-encoder =0.1.2`
with search; the memory parser migration is outside PH-5.

Automation history remains unknown under a concurrent writer. Zoom's nonblocking
shared lock can refuse while an existing tick or bind holds the exclusive lock,
producing a null verdict and an explicit automation gap. A later read verifies
the history again. The CLI guide now documents this existing 0.8.1 behavior;
the lock and detector implementation are unchanged by this clarification.

## Source verification

The focused zoom suite passes 12 tests. `cargo test --locked` passes 101 tests
with zero failed or ignored across 12 result groups. Formatting and strict
all-target Clippy pass. `cargo run --locked --example prepare_help` prepares
300 help sources and five companion routes, including the episode contract and
the concurrent-writer coverage limitation.
Builds use `CARGO_BUILD_JOBS=1`; no cache cleanup was performed.

An owned development-CLI proof executes 22 bounded calls with network access and
live Winnow writes denied. Each zoom call additionally denies writes to its own
project. Four blocked revisions fire; retirement reports four resolved and does
not fire. Open and active heads remain at zero. Re-blocking counts one, two and
then three, firing only at the configured threshold; completion reports only
that latest three-event episode as resolved. All 36 original file/directory
byte-and-mode bindings remain unchanged; deliberate transitions append 14 more.
The final task chain retains all 12 events. This is synthetic state evidence,
not registry or consumer acceptance. Reproducer, exact argv, exits, outputs and
receipts are retained in `.local/ph5-episode-0.8.2/`.

A separate exact requested matrix reaches four new blocked events after reopening
and then completion: the open head counts zero, the new episode counts one through
four, and completion resolves only those new four. All 22 calls exit zero with
the same denial profile and original-byte checks.

Five paired read-only samples per version on the live consumer store reproduce
the automation gap in both installed 0.8.1 and the development candidate. Each
version's coverage sequence is complete, complete, unknown, complete, complete;
all ten operation exits are zero. Both busy samples observe an unavailable shared
lock before inspection, and later reads recover complete coverage. Existing host
report receipts advance during the comparison; the probe invokes no tick or wake.
Pairs start at least 15 seconds apart, spanning 92 seconds from first to last;
slow debug queries make three intervals longer. An initial 20-second probe bound
expired after four complete reads; that incomplete attempt remains retained.
The completed repeat uses a 60-second child bound. Scripts, exact timestamps,
argv, exits and outputs are in `.local/ph5-automation-gap-0.8.2/`. These are live
read observations, separate from source tests and installed-release acceptance.

## Registry and signature record

[Paperhands 0.8.2](https://crates.io/crates/paperhands/0.8.2) was published at
`2026-10-22T23:09:05.766678Z` by `bboxy450` / user `445014`. The independently
fetched version record is unyanked. The downloaded registry archive matches the
approved candidate and upload bytes:

- SHA256: `e8b738126dc07fa25326b4a14ccc59b3d313187b182fe11e448acdb28b1ea741`
- Size: 1,098,859 bytes; 68 package files, with all 66 original payloads matching
  the approved source.
- Clean packaged source: `47ff6a7f3dc417d3f4b85776e20e3a941aa6b50d`
- Signed tag: `v0.8.2`, object `0659fe4acad1851052ff442c076125893f9999b8`,
  targeting that exact source.
- Signing fingerprint: `9B07E849480C68328CA20684844ECC0587DE7AB0`
- Detached signature: 265 bytes, SHA256
  `5d04ed1b467abb009645cf62f88dac09f085038f60e4f37a1439a5d3fd9e5ebe`

The [GitHub release](https://github.com/bboxy450/paperhands/releases/tag/v0.8.2)
contains the registry-identical
[crate](https://github.com/bboxy450/paperhands/releases/download/v0.8.2/paperhands-0.8.2.crate)
and its
[detached signature](https://github.com/bboxy450/paperhands/releases/download/v0.8.2/paperhands-0.8.2.crate.asc).
Both public downloads were independently byte-checked; GPG verifies the downloaded
crate and registry bytes against the signature and declared fingerprint.

Release announcement `c8bbc8c3-bf93-4b7e-bd2d-1963f7b0e937` was sent to
`#bboxy450-crates` with signature suffix `844ECC0587DE7AB0`. Its send receipt does
not establish peer review. Commands, exit statuses, identity checks, downloads
and verification receipts are retained in `.local/release-0.8.2/`.
The builder performed no global installation; consumer activation is separate.
