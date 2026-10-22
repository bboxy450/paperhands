# Paperhands 0.8.1 candidate

This is an unpublished candidate. This document does not establish completed
source gates, package verification, release approval, publication or consumer
adoption.

Zoom monoculture now clusters only the exact notes explicitly referenced by the
selected native retirement cohort. Original note IDs, vectors, model identity
and citations remain bound; all retained note bytes and source/vector bindings
are still verified. No note is rewritten or embedded again.

The monoculture signal declares `population: "referenced-notes"`, its
`referenced_notes` count and the selection's `snapshot_sha256`.

The selected population uses lexicographic greedy seed-radius clustering. Its
groups are not equivalent to a whole-root clustering result: omitted notes cannot
serve as seeds. Zoom retains the 8,192-link bound and caps its selected population
at 8,192 unique referenced notes, using exact vector segments of at most 4,096
rows. The whole-root `graph clusters` CLI keeps its existing 1,024-note cap.
Graph object/revision and memory limits still apply independently; missing
mappings or exceeded limits leave the signal and required coverage unknown.
Semantic groups remain navigation, not causal strategy families or independent
evidence.

`unreviewed_output` now retains two separately reported components:

- `native_experiments`: all completed native outputs without a bound durable
  review, preserving the existing execution-backlog definition.
- `open_untouched_tasks`: tasks currently open whose entire state history remains
  open, with age greater than `--unreviewed-age-ms`. Same-state revisions are
  allowed; age is measured from the latest event, not initial creation.

Each component has `count`, `oldest_age_ms` and `evidence`. The aggregate count is
their sum and the aggregate oldest age is their maximum available age. Known
empty components have count zero and no oldest age. If either component cannot
be verified, the aggregate remains null rather than treating that component as
zero; the known component is retained. A task's prose is not execution or review
proof. Adapters needing only native execution backlog must read
`native_experiments` instead of the aggregate.

Backlog inspection still covers all verified retained history, independently of
the event lookback window. The fixed `paperhands.zoom.verdict/v1` rule and its
unknown-coverage precedence are unchanged. No new model, evaluator, order, task
decision or wake is started by inspection.

The owned installed-0.8.0 cardinality proof is retained at
`.local/wn6-cluster-proof-0.8.0/verification.json`: 1,024 notes permit whole-root
clustering and complete zoom coverage; at 1,025 notes, whole-root clustering
refuses and zoom reports null with a coverage gap. Five synthetic retirements
all reference one original note, and duplicated note receipts reuse its vector.
This is evidence of the old boundary, not semantic diversity, independent
observations, financial performance or completed 0.8.1 acceptance.

Existing note bytes, identities, frozen execution bindings, schedules, recipients
and spent slots require no rewrite. Dependency pins remain `clap-how-to =0.1.5`
and `pocket-encoder =0.1.2` with search; the help-parser/schema migration remains
deferred.
