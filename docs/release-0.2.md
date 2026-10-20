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
