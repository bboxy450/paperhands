# Paperhands 0.6.0

Explicit local memory volumes let a project retain more than one 1024-note
partition without moving or re-embedding old notes. This is opt-in bounded local
storage; it does not implement the separate storage-v2/S3/R2/merge design.

`memory volumes --root STORE` verifies complete coverage and returns
`catalog_head`, `note_set_sha256`, active/closed volume counts and exact
`rollover_argv`. Preview that argv with `--dry-run` before explicit rollover.
Both expected digests are checked under the original memory lock. Each rollover
retains a hash-linked member seal and starts a fresh volume. All existing note
bytes, IDs, text, embeddings and receipt clocks remain in place.

The original root still selects the entire memory through list/read/search,
desk, companion and MCP. Metadata includes `retained_relative_path` for consumers
that project exact original JSON/vector bytes. Compare note-set digests across
pagination. A rollout head identifies closed-volume history; the separate note-set
digest includes current active members. Neither is a distributed transaction.

Multi-volume search uses one query vector and global semantic cosine with stable
note-ID/passage ties. Single-volume legacy hybrid ranks remain unchanged. Each
partition is processed separately; per-partition BM25/RRF scores are not globally
compared. Counts/model/coverage remain explicit, and any invalid or missing member
refuses successful complete retrieval. Desk/companion disclose the gap.

Admission now checks capacity before publishing an append, including on legacy
roots. Bounds are 16 volumes, 1024 notes per volume, 65,536 combined passages and
32 MiB indexed text. Rollover does not increase combined query budgets. Desk
attention validates linked memory endpoints and task history while verifying all
notes, avoiding the full graph's separate object limit for unlinked notes.
Full graph and clustering operations retain their explicit caps.

Migration is deliberate: unmodified legacy roots stay readable by old versions.
Enabling rollover adds a marker that makes old whole-store readers refuse instead
of silently omitting later volumes. Upgrade every writer first, retain a verified
private copy and keep the head/note-set receipts. Old writers can still alter a
closed set, which new readers refuse. Interrupted catalog creation may leave an
explicit gap needing inspected manual recovery; there is no automatic cleanup or
replay. No existing task, experiment, review, schedule or harness setting changes.

Consumers with a soft note guard must retain a per-active-volume admission guard,
scan every declared volume, check combined budgets and use returned relative paths
rather than assuming every note lives directly under `memory/`. This does not
authorize silently increasing an old global guard, pruning revisions, creating
historical executions or changing financial evidence.

`cargo test --locked` passes all 61 tests: 17 unit, 10 agent, 8 automation, 5 CLI,
4 judge, 11 lifecycle and 6 memory-volume tests. Strict all-target Clippy, format,
diff and managed-skill validation pass. Prepared help contains 256 sources and
five companion routes. The new witnesses cross 1024 notes, preserve global cosine
ties/citations, refuse concurrent/stale/over-cap writes, verify sixteen volumes,
exercise valid notes at the combined 32 MiB admission boundary, retain corrupt or
partial evidence as gaps, and run read-only MCP over 5120 notes. Passage-count
boundary checks are a separate unit witness; no 65,536-passage throughput is claimed.

An owned copy of current consumer memory/tasks/graph starts with 895 notes. Explicit
rollover plus one fixture note yields 896 notes / 10,121 passages over two volumes,
complete coverage and no gaps. All 963 copied original files remain byte-identical;
original source collections were read/copied under separate shared locks, not
modified. The actual global 0.5.1 executable refuses the rolled copy's marker.
This is development integration evidence, not financial validation or migration of
the consumer's original store. The global executable/consumer pin remains unchanged.

The clean packaged tree also passes all 61 tests with locked offline resolution.
`cargo publish --dry-run --locked` and the actual upload succeed. An independent
download matches that tested archive exactly and all 61 source payloads match
the clean source; dependency pins remain `clap-how-to =0.1.4` and
`pocket-encoder =0.1.1` with search.

- Registry version `0.6.0`, publisher **bboxy450 / user 445014**, unyanked.
- Clean packaged source/tag `v0.6.0`:
  `56ca9b1930b762a8d37f5e3c4e43eff75e5e03be`.
- Archive: 964,820 bytes, SHA-256
  `0fbce34a53ec90280950c3e6531af95d81499c16bff9c7b03517e4bae965f5b5`.

The builder left global 0.5.1 active and did not enable rollover in any consumer
store. Registry availability, consumer acceptance and opt-in format adoption
are separate. Private command logs, source-copy bytes and verification receipts
stay outside Git and the crate.
