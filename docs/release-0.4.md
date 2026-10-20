# Paperhands 0.4.0

Decision-model evals are an instrument beside the existing research lifecycle.
The project companion remains offline. The experiment runner, financial adapter
and review/export source are unchanged.

- `judge validate` verifies a pinned TypeSafe plan and exact dataset without
  credentials, writes or network. `judge run` reserves a fresh identity and
  performs bounded Noul scoring with no retry, redirect, proxy or model fallback.
  It retains every case, request, response prefix, partial failure and unknown.
  Credentials stay outside the project and reflected keys are redacted.
- `judge analyze` accepts provider-neutral saved scores. Identical fixed grouped
  folds control training-only constrained F-beta threshold selection and
  out-of-fold logistic calibration. Infeasible thresholds remain null, failed
  cases remain unsupported, and worsened recalibration remains visible. Token
  costs use declared rates; unknown cost does not become zero.
- `judge inspect`, the project desk and CLI/MCP views verify retained evidence.
  Offline threshold/calibration plots and the eval ledger share renderer-neutral
  typed datasets. The managed skill, bundled decisions guide, inert synthetic
  example and 211-source semantic help index describe this actual surface.

Verification:

- `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
  `node --check assets/viewer/app.js` pass.
- `cargo test --locked`: 38 tests pass: 9 unit, 9 agent integration, 5 CLI,
  4 judge integration and 11 lifecycle. New checks exercise training-label
  isolation, infeasible thresholds, worsening calibration, missing/nonfinite/
  duplicated/mismatched scores, group leakage, input mutation/symlinks, pinned
  provider contracts, duplicate/concurrent identities, overdue successful
  responses, credential redaction, HTTP response caps/redirect refusal, and
  actual CLI/stdio-MCP view parity. Existing 200-member failure checks pass.
- The embedded example executes offline under OS network denial: 20 synthetic
  cases, five groups/folds and three illustrative saved-score models. The
  low-range model's 0.50 recall is zero; training-only threshold 0.40 yields the
  expected validation result. The uninformative model never qualifies. These
  are fixture assertions, not real provider or financial performance claims.
- Local browser bridge checks in light and dark themes verify three threshold
  plots, three calibration plots, the correct three-row eval ledger, visible
  Pele lineage, finite SVG coordinates and no document overflow. Screenshots
  were inspected. This does not establish acceptance by every native MCP host.

The adapter follows TypeSafe's official Noul/OpenAPI contract. Provider tests use
mock and loopback responses; no live paid call or real judged corpus was used.
Fresh reviewed labels and a suitable causal split remain consumer work. Grouped
cross-validation is not a forward-time/purged market test. Model scores, eval
eligibility and cost frontiers never change trading authority or policy status.

Published and activated as **bboxy450**, unyanked:

- Registry archive: **838,775 bytes**, SHA-256
  `5f4db9e9933f1dd379f018011b0a18ad223917697632cacee0f5298e86402a3b`.
- Clean source and pushed tag `v0.4.0`:
  `bb375e7e0913a1ffb63c16f73772c22ffc17f4c9`.
- The independent registry download matches the preflight archive exactly. Its
  57 members contain 55 byte-matched source payloads plus generated package/VCS
  metadata, with no private datastore, local evidence or credential.
- Fresh isolated exact-registry installation succeeds. Its installed binary
  passes the embedded 20-case/five-fold/three-model fixture, plan validation,
  retained inspection and five real stdio MCP responses under OS network denial.
  The renderer-neutral threshold dataset has 303 rows. Managed skill bytes match.
  Light/dark browser bridge checks also pass using this exact installed binary.
- Global activation matches the isolated build: **28,778,560 bytes**, SHA-256
  `39c1fc7aba45e9fe8665519054a2161d7bfe5205bd9317ce007ee921a850e2c9`.
  This is a machine/build hash; the registry checksum is the portable release pin.
- A separate read-only authenticated TypeSafe `GET /v1/models` returns HTTP 200.
  It lists aliases, as the [official model reference](https://docs.typesafe.ai/models)
  documents; absence of exact version IDs in that list is not a pin failure.
  No `POST /v1/systemone` inference or paid model experiment was performed.

The exact version/checksum/source and runnable offline fixture were sent through
ICQ to the Winnow consumer for independent review. This is not yet consumer
acceptance of 0.4.0. Its accepted 0.3.1 archive integration remains a separate pin;
this release does not add legacy-result registration or increase archive bounds.
