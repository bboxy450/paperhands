# Consumer feedback

Winnow initially accepted 0.3.1, and subsequently reports acceptance/integration of
the exact global 0.4.0 registry artifact at
`c5698848113ebfcf78c19e9c14a7e7fe642d0530`: 83 passing consumer tests, exact document
restoration and complete retrieval through six explicitly bounded native stores.
These are follow-up questions, not claims of financial validation.

- **Desk shape:** `desk` returns a view envelope. Summary fields are under
  `data.desk`; datasets and graphs remain beside that summary. Consumer adapters
  should inspect the schema and select that explicit path.
- **Source revisions:** notes retain at most 16 KiB of exact UTF-8. Winnow currently
  supplies lossless partitions with full-source hashes, byte ranges and stable
  citations, then reuses existing note IDs on refresh. A native idempotent source
  revision/import operation would reduce adapter bookkeeping.
- **Multiple stores:** a read-only desk spanning explicitly selected historical
  stores would help inspect old cohorts without copying attempts, changing their
  identities or treating separate evidence as a combined portfolio.
- **Managed skills:** consumers running historical binaries can set a separate
  `PAPERHANDS_SKILLS_DIR` per version. Winnow does this so an older inspection
  cannot downgrade current shared instructions.

Paperhands owns its project store, research DAG, provenance, companion and MCP
interface. It uses shared encoder/help crates and borrows useful Ontology
patterns without waiting for a separate live-datapack runtime. Narrow decision-model
eval instruments shipped in 0.4; a reviewed consumer label corpus and actual judge
accuracy remain open. Source/score annotations must distinguish supported,
contradicted and insufficient evidence; one event or one synthetic batch cannot
establish calibration. The companion remains offline.
