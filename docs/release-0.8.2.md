# Paperhands 0.8.2 candidate

This candidate implements the refined PH-5 episode contract. Publication requires
separate approval; source verification and consumer adoption are separate evidence.

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

## Source verification

The focused zoom suite passes 12 tests. `cargo test --locked` passes 101 tests
with zero failed or ignored across 12 result groups. Formatting and strict
all-target Clippy pass. `cargo run --locked --example prepare_help` prepares
299 help sources and five companion routes, including the episode contract.
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
