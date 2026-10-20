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

Publication and consumer handoff receipts are recorded after the source cut.
