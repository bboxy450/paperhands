# Consumer feedback

Winnow accepted and integrated the exact 0.3.1 registry artifact at
`521fe63d27545e0c0f7480c2dc772d1d8186c13a`. These are follow-up research questions,
not release blockers or capabilities already implemented.

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
patterns without waiting for a separate live-datapack runtime. Model decisions
and evals await the operator's next brief; the released companion stays offline.
