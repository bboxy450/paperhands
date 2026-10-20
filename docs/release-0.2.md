# Local experiment release evidence

Scope: compose existing trusted evaluators, retain closed inputs and failures,
schedule bounded batches, compare supplied cases and record explicit reviews.
No consumer source, running capture, account or trading operation was changed.

Verified on 2026-10-20:

- `cargo clippy --locked --all-targets -- -D warnings` — exit 0.
- `cargo fmt --check` and `git diff --check` — exit 0.
- `cargo test --locked` — index freshness and five CLI tests pass. The expanded
  negative fixture hit its initial short startup deadline under concurrent load;
  after separating ordinary-command and timeout budgets,
  `cargo test --locked --test lifecycle` passes all eight lifecycle tests.
- Lifecycle negatives exercise source/input/plan changes, member/path/link
  refusal, spent/concurrent identities, crash, timeout including descendants,
  simultaneous stdout/stderr pressure, partial output, missing/nonfinite/duplicate
  or incompatible metrics, and retained-output mutation. Timeout leaves an
  independently started sleep process alive.
- `quick_validate.py skills/paperhands`, using cached PyYAML — skill valid.
- `cargo package --list --locked --allow-dirty` — expected source, lockfile,
  managed skill, prepared help, docs, examples and tests; no local evidence/data.

`examples/winnow_closed.py` created fresh plans over a consumer-supplied closed
696-file, 70,370,456-byte development capture. Both unchanged existing evaluators
ran under macOS network denial; a socket-bind negative control returned EPERM.
Both raw candidate catalogs are byte-identical to the independently supplied
reference hashes. `examples/verify_winnow.py` verifies exact partition totals,
common/unmatched anchors, changed cash/quantities and all unknown holdings.
Raw artifacts, process evidence and reference assertions remain local.

The retained 200-member synthetic example used four workers and three shared
snapshots: 197 complete results, one crash, one timeout and one command-success
with missing required evidence. Execution took 8,342 ms; child CPU totaled
5,819 ms; maximum child peak RSS was 19,562,496 bytes. This is not simultaneous
batch RSS or a market-policy capacity benchmark. Store bytes were 2,910,054;
charged evaluator artifacts were 123,744 bytes. All members remain inspectable.

Source mechanisms: `src/research/inventory.rs` hashes/copies all retained members;
`plan.rs` reserves exclusive identities and freezes exact plan bytes;
`runner.rs` owns process groups and captures actual exits/resources/artifacts;
`batch.rs` records every member under shared budgets; `comparison.rs` preserves
supplied common/unmatched cases, private reviews and allowlisted public summaries.

Remaining scope: prospective paper-account ledgers, venues/orders, ACP/MCP and
rendering are proposed in `docs/design.md`. Commands are trusted external code;
this runner does not provide a security sandbox or independent financial audit.
CPU/file limits are per process; artifact aggregate limits are sampled, not
atomic reservations. Python standard libraries are not packaged or frozen.

Consumer review found that batch inspection returned historical member results
without rechecking their artifacts. Inspection now verifies original plan/queue
bindings, member receipts and current retained inventories before reporting
`integrity_status: verified`; any gap refuses with exit 2 and names the member.
`batch_inspection_refuses_changed_member_output_and_preserves_completion_receipts`
reproduces output mutation through the actual CLI and checks the whole original
batch record tree remains unchanged. Historical review fields are labeled as such.

The reviewer also reproduced an overdue child being classified as successful after
the runner was temporarily stopped. Reaped-child completion now checks the elapsed
deadline; preparation also checks the budget before spawn. The lifecycle regression
stops/resumes only its freshly launched runner and verifies a real child exit 0
is retained alongside timed_out/operation exit 3.

After both fixes, `cargo test --locked` passes 17 tests (2 unit, 5 CLI, 10 lifecycle);
format, Clippy and diff checks pass. Inspection of a retained 200-member store
verifies all members in 0.50 seconds; a mutated copy refuses with exit 2 and leaves
its complete historical batch tree byte-identical. A separately retained clock
control observes a 450 ms child after pausing its owned runner for 700 ms: actual
exit 0, observed step wall 717 ms, 300 ms limit, timed_out and operation exit 3.
