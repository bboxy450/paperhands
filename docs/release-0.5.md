# Paperhands 0.5.0

Native harness setup and bounded research wakes build on the existing local
research datapack. The experiment runner, financial comparison and accounting
contract remain unchanged; S3/R2 replication is still a separate storage design.

- `init --research TEXT` retains user-owned research instructions, records direction
  and seeds a task DAG. Repeated initialization preserves task decisions and the
  original brief. Normal init registers project/user MCP; isolated setup is explicit.
- `setup` merges Paperhands into native Codex TOML and Claude JSON, preserving other
  settings and TOML comments, retaining private backups and sharing one managed
  skill through Codex/Claude discovery links. It preserves existing approval and
  inbound policy. Configured MCP and discovered/connected tools remain separate facts.
- `automation` creates its own ICQ sender, pins a recipient binding per finite
  schedule, coalesces missed slots and retains exclusive slot claims and receipts.
  Rebinding changes defaults for new schedules; historical schedules keep their
  original recipient. Uncertain sends are not automatically retried. Owned cron
  blocks install/repair/remove without replacing unrelated jobs or LaunchAgents.
- Memory queries above one index's limit use bounded segments and one Pocket Encoder
  query vector. Below-cap hybrid ranking stays identical; larger corpora explicitly
  use globally comparable semantic cosine and stable note-hash/passage ties.
  All selected passages are considered. Retrieval failure appears in companion
  context gaps rather than becoming an empty archive.

The [automation guide](automation.md) and updated managed skill explain actual
commands, interval/lifetime/wake budgets, ICQ prerequisites and native-delivery
limits. No new harness/model, collector, provider call or trading engine is
automatically launched by initialization or cron registration. Waking an existing
harness does not establish that it performed the research.

Source verification:

- `cargo test --locked`: 48 tests pass: 14 unit, 9 agent integration, 5 automation,
  5 CLI, 4 judge and 11 lifecycle. Checks include native config preservation and
  private backups, repeated bootstrap, recipient revision pinning, concurrent wake
  claims, failed sends, real pipe/deadline behavior, expiry/coalescing, cron
  install/repair/remove against an isolated native-command fixture, segmented search,
  below-cap rank parity and companion retrieval gaps.
- Strict Clippy, format, managed-skill validation and package dry run pass. The
  regenerated semantic-help index contains 239 sources and five companion routes.
- Read-only consumer query over the current Winnow archive considers 651 notes /
  7,066 passages in two segments, returns cited hits and complete coverage, and
  leaves every note's bytes unchanged. It does not establish financial evidence.
- A one-wake native ICQ fixture uses a separate owned sender, targets this harness,
  retains its message UUID/recipient-stored receipt and exhausts its one-wake budget.
  The receiver inbox records Codex queue admission. This establishes storage/admission,
  not autonomous research completion. No recurring native cron job was installed
  during this transport fixture.

Private command logs, query byte verification and wake receipts are retained outside
Git and the crate. Exact registry activation is recorded after publication. This
file is repository release evidence, not a bundled operational guide.
