# Paperhands 0.8.2 candidate

This candidate implements PH-5. Publication requires separate approval; source
verification and consumer adoption are separate evidence.

`repeated_refusal` now classifies historical blocked events using the task's
verified current head. Only `open` and `blocked` heads participate in the
same-task, identical-nonempty-note groups that can fire the signal. A `done` or
`retired` head moves its in-window nonempty blocked events into the signal's new
`resolved_refusals: {count, evidence}` field. Its count totals those events across
closed tasks and reasons; evidence names the original exact revision IDs. These
resolved events never enter the firing count. An `active` head contributes to
neither count.

The head comes from all verified task history, while event eligibility still uses
the inclusive `--since` window. Reopening a task makes its eligible retained
refusals count again. This is a head-state rule, not a fresh-episode counter:
leaving and reentering `blocked` does not erase or reset retained events. Blank
decision notes remain excluded. When task history is unavailable, resolved count
is null and evidence is empty, alongside the unknown active measurement.

The fixed `paperhands.zoom.verdict/v1` rule is unchanged. Other signals can still
produce a stalled or unknown verdict after one task is closed. Lifecycle errors
remain separately reported. No detector output creates tasks, retires a premise,
starts an experiment or authorizes action.

This addresses Winnow's installed-0.8.1 closed-gate reproduction: four identical
blocked events followed by a retired head continued to fire the former
historical-window signal. Original task chains, source timestamps and evidence
remain intact; only the derived classification changes.

Regression fixtures cover blocked heads, both terminal states, reopening,
active-head exclusion, mixed eligible and resolved groups, exact resolved
citations, event-window and blank-note exclusion, and unavailable task history.
Inspection preserves every retained fixture byte; deliberate test transitions
append native events rather than replacing prior revisions.

No store migration is required. Readers of `repeated_refusal.count` now receive
only the largest eligible open/blocked group; readers wanting closed history can
inspect `resolved_refusals`. CLI, stdio MCP, companion and public summaries use
the same detector. Existing schedules, recipients and spent slots are unchanged.
Dependencies remain `clap-how-to =0.1.5` and `pocket-encoder =0.1.2` with search;
the memory parser migration is outside PH-5.

## Source verification

The focused zoom suite passes all 12 tests. The full `cargo test --locked` run
passes 101 tests with zero failures or ignored tests across 12 result groups.
The first full-suite compilation exited 101 with ENOSPC before tests began;
its log is retained. After available capacity increased, the successful retry
used `CARGO_BUILD_JOBS=1`. No cache cleanup was performed.

`cargo run --locked --example prepare_help` prepared 299 help sources and five
companion routes. Formatting and strict all-target Clippy pass. The prepared
help index includes the new current-head and resolved-history contract.

An owned development-CLI proof runs 18 bounded calls with network access and live
Winnow writes denied. Four blocked events produce `stalled`; retirement moves
all four to resolved history and produces `moving`; reopening restores the four
eligible events and `stalled`. An active head contributes to neither count. A
later blocked event raises the eligible count to five; completion resolves all
five. All 36 original file/directory byte-and-mode bindings remain unchanged,
while deliberate task transitions append ten more bindings. This is synthetic
state-transition evidence, not registry or consumer acceptance. Exact scripts,
argv, outputs and receipts are retained in `.local/ph5-0.8.2/`.
