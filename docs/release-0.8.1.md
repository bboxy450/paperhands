# Paperhands 0.8.1

Published to crates.io on 2026-10-22 at 00:32:30 UTC by **bboxy450**
(user 445014). The independently downloaded registry archive matches the approved
candidate byte-for-byte. Consumer installation and acceptance remain separate.

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

## Verification and published artifact

All five source gates exited 0: `cargo run --locked --example prepare_help`
(297 sources and five companion routes), `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --locked`
(99 passed, zero failed or ignored across 12 result groups), and
`cargo publish --dry-run --locked`. The Oracle independently repeated the gates
on the exact approved commit after the population-label addendum.

The checks cover native rollover beyond 1,024 retained notes, exact selected-note
citations and population bindings, invariance after unrelated note additions,
missing or ambiguous mapping refusal, unchanged whole-root cap, segmented vector
ties, strict task age and state history, separate backlog components and unknown
coverage. The owned over-cap fixture establishes synthetic cardinality behavior;
it does not establish independent evidence, throughput or economics.

- [crates.io 0.8.1](https://crates.io/crates/paperhands/0.8.1): publisher
  `bboxy450` / user `445014`, unyanked when independently checked.
- Registry archive: **1,091,202 bytes**, SHA-256
  `5e2d48a233e7f5b1bae612d2963e3c96a3b646fa9d21dbc8a4eb259ed5eb5fe1`.
- Clean packaged source and signed tag `v0.8.1`:
  `80680294ef581f53a94147596bb889c92f4e0c2b`. Tag object:
  `57f800929cd85d90363b361ba6a44fb0b1ecb58e`.
- All 66 original payload files match the approved source. Normalized dependency
  pins remain clap-how-to `=0.1.5` and pocket-encoder `=0.1.2/search`.
- [GitHub release and signed package](https://github.com/bboxy450/paperhands/releases/tag/v0.8.1)
  contains `paperhands-0.8.1.crate` and `paperhands-0.8.1.crate.asc`. Both downloaded
  assets match their uploaded bytes and server SHA-256 digests. The package also
  matches the independently downloaded registry bytes.
- Tag and artifact signer:
  `9B07E849480C68328CA20684844ECC0587DE7AB0`
  (`Boxy <bboxy450@gmail.com>`). Both independent downloads verify with that key.

```sh
gpg --verify paperhands-0.8.1.crate.asc paperhands-0.8.1.crate
```

Before upload, the authenticated registry endpoint bound to user 445014 returned
HTTP 200 both before and after `cargo login`, verifying the approved identity.
No credential value was printed or included in source. Packaging, artifact/tag
signing and verification, tag push, `cargo publish --locked`, registry verification,
GitHub asset upload/publication and download verification all exited 0.
Cargo reported the unchanged locked `tokio 1.54.0` yank warning; package
verification and upload still completed. No dependency changed during release.

The release announcement was sent to `#bboxy450-crates` in the publishing turn
after byte and signature verification, receipt
`9e5369d1-af12-4b83-91d1-deb3f8059cc3`, with suffix `sig 844ECC0587DE7AB0`.
The receipt proves local storage and scheduled forwarding, not remote or model
receipt. No global binary was installed; consumer adoption follows its own brief.
