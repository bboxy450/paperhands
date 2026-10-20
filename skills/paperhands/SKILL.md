---
name: paperhands
description: Run and inspect local Paperhands research experiments, bounded evaluator batches, retained evidence and explicit reviews. Use for Paperhands capability discovery and semantic help.
---

# Paperhands

Start with `paperhands status` and `paperhands docs read experiments`.
Version 0.2 runs trusted supplied evaluators over frozen closed inputs. The
consumer owns strategy logic, causal data interpretation and financial accounting.
Venue access, account simulation, orders, MCP and ACP app integration are proposed.

Ask the binary for relevant commands and source excerpts:

```sh
paperhands how "what can this version do?"
paperhands cli search "show installed capabilities"
paperhands docs search "how are agent skills updated?"
paperhands docs list
paperhands docs read cli
paperhands docs read experiments
```

JSON is the default for results, help, version and errors. Use `--human` only
when readable text is requested. Search is offline semantic retrieval using
`clap-how-to`; inspect the cited source or command help before choosing an action.
Search never executes a result and does not generate an answer.

Let observations generate several explanations before ranking them. Bring in
mechanisms from outside the current strategy family. Name what would change your
mind, then run the cheapest discriminating experiment. An informative failure or
an unresolved question belongs in the research record. Retiring an attempt closes
that test; record when the idea deserves revisiting.

Use `inventory seal`, `verify` and `copy` for retained trees. Declare closing
evidence, exact argv, fixed code/config/accounting/input hashes, data-use labels
and explicit resource limits in a plan. Validate or create with `--dry-run`
before execution. Create reserves a fresh identity; run spends it once. Never
reuse, resume or overwrite a spent attempt, reset Git, synthesize missing input,
or replace retained bytes with a current API response.

`experiment run` records real PIDs, exit/timeout, bounded stdout/stderr and
artifacts. `batch run` schedules already-created members with a bounded worker
pool and total budgets. A supplied evaluator may itself evaluate many logical
policies. Shared data and 200 labels do not imply 200 independent outcomes or a
combined funded portfolio. Separate account ledgers remain the evaluator's job.

Inspect process state, evidence status and review separately. Exit success is
not financial validation. Comparison requires the same fixed inputs, evaluator,
accounting and data-use labels; preserve common/unmatched cases, unknown metrics,
raw costs/holdings, economic aliases and supplied diagnostics. Null is unknown,
never zero. Report unsupported financial assertions explicitly. No scalar result
automatically promotes a strategy.

Use `batch inspect` to verify every member's current retained evidence and original
batch binding. Mutation or unverifiable evidence refuses with exit 2. Completion
and review fields in the batch receipt remain historical; inspect individual
attempts for current review state.

Finish with an explicit `experiment review` disposition and private evidence/revisit
note. `experiment export` writes allowlisted lifecycle JSON/Markdown only, excluding
raw notes, names, paths, metrics and diagnostics. Financial publication belongs to
the consumer's reviewed publisher. Commands are trusted; process groups and hash
checks do not provide a network or filesystem sandbox. Apply a separate sandbox
when the experiment requires it. Do not touch another session's collectors or files.

The binary installs and refreshes this managed file on every invocation.
Local edits are overwritten. Keep personal additions in separate files.
`PAPERHANDS_SKILLS_DIR` overrides the complete skill directory; the default is
`~/.agents/skills/paperhands`. Run `paperhands skills` to inspect installation.
