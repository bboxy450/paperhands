# Paperhands 0.3.0

This release adds a project research companion around the accepted 0.2 lifecycle.
It reuses Ontology's shared-dispatch, bounded MCP, provenance vocabulary and offline
Pele presentation patterns. It pins registry clap-how-to 0.1.4 and pocket-encoder
0.1.1 with exact search enabled. There is no full Ontology runtime dependency.

Implemented surface:

- `project.rs`: nearest-ancestor `paperhands.toml`, stable project/store binding,
  safe relative storage, relocation and explicit standalone-store compatibility.
- `companion.rs`, `runtime.rs`, `mcp.rs`: grounded `ask`, exact shared CLI/MCP
  dispatch, project-scoped server launch, bounded JSON-RPC and bundled resources.
- `memory.rs`, `tasks.rs`, `graph.rs`: retained text and typed task revisions,
  append-only weighted links, whole-DAG validation, priority-aware ready work,
  cited semantic neighbors and bounded deterministic clustering.
- `view.rs`, `assets/viewer`: renderer-neutral typed datasets and graphs, offline
  light/dark MCP Apps presentation, revision labels and separate weight meanings.
- `examples.rs`: versioned inert adapters inside the binary, including a fresh
  synthetic research-desk example; managed skill and generated help ship together.

Source verification:

- `cargo fmt --check`, strict all-target Clippy and viewer JavaScript syntax pass.
- `cargo test --locked`: 28 tests pass: 3 unit, 9 agent integration, 5 CLI and
  11 lifecycle. Includes real stdio MCP parity, inline writes, project identity,
  DAG cycles/old revisions, mutated retained data, bounded clustering, failed
  views and the 200-member partial-failure fixture. The first-use companion path
  also receives a focused agent-suite check after its project-init correction.
- The fresh synthetic companion example passes with OS network denial. It
  creates four illustrative notes, four tasks/five revisions, three declared
  links, cited focus/next-work answers and a project-scoped MCP configuration.
- Local browser bridge checks render desk and weighted research graphs, switch
  table/duration views and verify actual DOM rows in light/dark themes. Graph
  edges use the theme palette. This is a local MCP Apps protocol fixture, not
  acceptance by every native app or an implemented ACP application.
- Retained closed-data baseline/candidate and 200-member stores from 0.2 are read
  with both exact installed 0.2 and new 0.3 binaries under network denial. Data and
  exit semantics match after excluding only `integrity_checked_at_ms`; all 4,430
  retained files and member sets remain byte-identical. No attempt is rerun.

The lifecycle engine source remains unchanged. `batch inspect` retains its
historical batch exit semantics; valid `view` operations return zero while
preserving failed run state. New project/memory/graph data is private local state.
No existing attempt migration, private-session ingestion or automatic promotion
is performed. Venue orders, a persistent trading daemon and the ACP app remain
outside this release; consumer execution/accounting stays authoritative.

Registry release and activation:

- Version **0.3.0**, publisher **bboxy450** / user445014, unyanked.
- Registry archive: 744,772 bytes, SHA-256
  `741f55303f33f6a621bca00bf765e9127a73838cda07529b71776c3a8cf6bd56`.
- Clean source and pushed tag `v0.3.0`:
  `ef3902c9d86d6c73cf692012b4aa051165afd63c`.
- The downloaded archive matches the preflight package. All 28 tests also pass
  from that exact extracted registry source. A fresh isolated `cargo install
  paperhands --version =0.3.0 --locked --root PREFIX` succeeds.
- The installed binary passes network-denied embedded-example extraction,
  project companion, real stdio MCP, DAG, managed-skill and retained closed-data /
  200-member inspection checks. The actual registry viewer passes light/dark
  browser bridge checks, including all nine object rows and themed edge strokes.
- The global installation is updated to the same binary: 22,995,424 bytes,
  SHA-256 `df9538dd16f6db2b7e09c03b573d851b704d6311ed3a9e285ca0326264a6d96c`.
  Its default shared managed skill matches the registry payload. Binary hashes
  describe this machine/build; the crate checksum is the portable release pin.

This repository now has its own `paperhands.toml`. Its private ignored store
records the operator direction, completed implementation/publication, exact
release evidence and the Winnow consumer-review task as a real DAG.
`paperhands ask 'What should I do next?'` works from this checkout and subfolders.

The exact version/checksum/source and runnable embedded example were sent through
ICQ to both authorized topics and directly to the Winnow consumer. Winnow accepted
and integrated 0.3.0 at `0a166ab26af2f83f4260ecde09b9f765a73be5f1`.
Its source, committed controller hooks and retained receipts were inspected:
closed-data catalogs reproduced byte-for-byte, a fresh four-worker batch retained
all 200 members including failures, offline MCP passed, and 80 consumer tests
passed. Curated research-memory refresh is connected to its existing controllers.
This is software/adapter acceptance; financial interpretation remains with Winnow.
The subsequent accepted formatting patch is recorded in [0.3.1](release-0.3.1.md).
