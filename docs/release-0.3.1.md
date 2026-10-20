# Paperhands 0.3.1

`--human ask` now prints the companion's concise answer. In 0.3.0 a nested help
object shadowed the answer selector, causing the readable renderer to dump the
whole structured context. The selector now chooses the first textual field.
JSON/MCP results, retained schemas, lifecycle execution and model/dependency pins
are unchanged.

The CLI regression exercises the actual companion and refuses a context dump.
Against the same local project, the observed output changes from 7,752 bytes to
189 bytes. Format, strict all-target Clippy, prepared-index freshness and the full
28-test suite pass. The 0.3.0 exact-registry acceptance remains the engine baseline.

Published and activated as **bboxy450**, unyanked:

- Registry archive: 744,844 bytes, SHA-256
  `c2b2c7093bef7b8bcb9bfff40733347290807d7c83e2f6d55ff66797694a1c0f`.
- Clean source and pushed tag `v0.3.1`:
  `c62d894a3676714821cd27f8585ac6d0764b0765`.
- Independent registry download matches the preflight archive and source. Fresh
  isolated and global exact-version Cargo installations succeed. The active
  machine binary is 22,994,816 bytes, SHA-256
  `eac1a6c9c6adfbf6fd91614d52beba8453f3423741df84d8cb642d3b348e3cce`.
  The managed skill matches the package. The binary hash identifies this build;
  the registry checksum is the portable release pin.
- The installed binary passes network-denied concise human output, retained JSON
  context and real stdio MCP checks. No model request is needed for the companion.

Winnow independently accepted the exact registry artifact and pinned it at
`521fe63d27545e0c0f7480c2dc772d1d8186c13a`. Its actual pin, source diff,
artifact identity, installed binary bytes and acceptance receipts were inspected.
Its human reproduction changes from 7,907 to 527 bytes and equals the JSON answer
plus newline. Paired JSON differs only in inspection/generation clocks; five
offline MCP responses pass and 4,858 retained files remain unchanged. Earlier
0.3.0 execution evidence carries forward because the engine source is unchanged.
The consumer reports all 80 version-aware tests passing and refreshed curated
memory with no track issues. Its financial evaluator remains authoritative.

Nonblocking integration feedback is retained in
[consumer feedback](consumer-feedback.md) and the private project task DAG.
