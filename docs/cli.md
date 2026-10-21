# Paperhands CLI contract

## Discover commands by intent

`paperhands how-to QUESTION` and its alias `how` search the real Clap command
tree and bundled guides. `paperhands cli search QUESTION` searches commands;
`paperhands docs search QUESTION` searches documentation. All three use
clap-how-to 0.1.5: local neural cosine similarity plus BM25 with reciprocal-rank
fusion, backed by pocket-encoder's bundled English MiniLM-L3 sentence transformer.
Prepared document vectors are embedded at build time; only the question is
encoded at runtime. No API key, network, model download or query log is needed.

Search accepts one quoted question, 1..512 UTF-8 bytes without control characters,
and returns at most five source matches. Scores rank sources; they are not
probabilities. The model has a 128-token context and can return unrelated matches.
Search retrieves existing help and documentation; it does not generate answers
or execute suggested commands. Command results include an exact help invocation;
document results identify a bundled source, section and one-based line.

## Read documentation

`paperhands docs list` returns guide IDs. `paperhands docs read ID` reads bundled
text selected by ID: `overview`, `cli`, `experiments`, `agents`, `autoresearch` or `skill`. These are versioned snapshots
inside the executable, not arbitrary filesystem paths or live websites.
Use `--start-line N --lines 60` to page through a source. Follow `next[].argv` for
the remaining text; omitting `--lines` preserves whole-guide output.

## Output

Every successful invocation writes one JSON object to stdout with `schema`,
`version`, `ok`, `command`, `data` and `next`. `next[].argv` contains exact argument
arrays, never an executable shell string. Help and version follow the same contract.
`paperhands help COMMAND ...` includes argument actions, defaults, enum choices and
cardinality for agent discovery.
Refused operations write one JSON object to stderr, leave stdout empty and exit 2.
Completed experiment/batch runs with failures write retained results to stdout with
`ok:false` and exit 3; inspect their process and evidence states. Startup/encoder
failures exit 1. Add global `--human`
before or after a subcommand for readable output. No command launches a pager.

## Agent skill lifecycle

On every invocation, before command execution, Paperhands compares its embedded
skill with `~/.agents/skills/paperhands/SKILL.md` and atomically replaces missing
or different contents. This includes first run, help and version. The executable
is authoritative even after a downgrade. Unchanged files are not rewritten;
unrelated files are retained. The managed skill file may not be a symlink.
An installation failure exits with a structured error instead of claiming success.

Set `PAPERHANDS_SKILLS_DIR` to override the complete destination directory.
`paperhands skills` returns the installed directory, bundled content and refresh
result. No harness settings or external skill files are modified. Harnesses that
discover `~/.agents/skills` directly or through symlinks can load this skill.

## Current capabilities

`paperhands status` reports `stage: local-experiments`, `trading_enabled: false`
and planned venues. `inventory` retains/verifies closed data; `experiment`
freezes plans, runs bounded external commands, compares cases and records reviews;
`batch` provides a bounded worker pool. Read `experiments` for exact schemas,
limits, state transitions and runnable examples. Polymarket and Robinhood are
intended integrations only. This release has no venue connections or order engine.
`init` creates `paperhands.toml` and a project-bound `.paperhands` store;
`project` verifies discovery and identity. Explicit `--root` preserves standalone
store use and existing 0.2 lifecycle paths. `ask` is the natural-language front door
for next work, state, direction, notes and shipped how-to guidance. Answers are
recorded-state templates and cited retrieval, with suggested argv that never run
automatically. The external agent supplies creative reasoning. `desk` shows open questions, task prerequisites,
the research frontier and selected experiment integrity/state. `tasks` retains
explicit work and its decision history; `memory` retains and questions original
research notes. `graph` exposes the verified research DAG, immutable weighted
relations, semantic neighbors and bounded local clusters. `mcp` exposes these same operations over stdio; `mcp-config` prints
a launch config without editing a client. `setup` merges native Codex/Claude MCP
entries and shared skill discovery; normal project init registers project-bound
MCP as well. `init --research TEXT` seeds research instructions/direction and tasks.
`automation` binds an owned ICQ sender and finite wake schedules, with durable slot
receipts and an isolated cron block. Read `automation` before enabling recurring work.
`view` returns verified renderer-neutral
graph/chart/table data, with an embedded Pele viewer for MCP Apps. `bundle`
identifies embedded payloads by hash and license. `examples` extracts the inert
versioned adapters carried by the executable. Read `agents` for exact limits.

## Public research reports

`paperhands report --root ROOT --out DIR` creates
`DIR` if needed and writes `report.json` (`paperhands.report/v1`) and `report.md`
with visibility `public-allowlist`. Existing files are replaced atomically per
file; the pair is not a transaction. Check each generation time when consuming it.
The Markdown represents the same report fields as the JSON.

Reports contain aggregate research-frontier counts, at most 100 sanitized task
titles with state/readiness, at most 200 reviewed experiments passing the existing
export binding checks, and finite automation status. Task and experiment rows
are newest first. Unknown/incomplete evidence remains visible through gap counts,
inspection state and limitations. A written report is not proof of completed
research or financial validity; unreviewed experiments are excluded.

`frontier.memory_coverage` projects verified counts, bounds and explicitly named
SHA-256 fields; private volume IDs, directories and rollover commands are omitted.
Unknown memory or task totals refuse generation, preserving existing outputs.
Other inspection gaps contribute only their count. `questions_open` counts retained
question notes; resolving a question is not inferred from later prose.

The 0.7.1 candidate names the execution-catalog pagination flag
`frontier.catalog_fully_paginated`. It means that the selected root's experiment,
batch and judge-run catalog was fully included in the inspection page. It does not
mean that integrity checks passed, source notes were substantively reviewed or
financial claims were validated. In 0.7.1 only,
`frontier.all_evidence_inspected` remains a deprecated same-value alias. Report
consumers should use `catalog_fully_paginated`; gap counts and explicit reviews
remain separate evidence.

The allowlist excludes raw notes, plans, argv, paths, messages, keys, recipients
and task/project/experiment/schedule IDs. Task titles are capped at 120 characters
after removing URLs, paths, long hexadecimal strings and token-like text. A final
serialized-output check refuses surviving private-looking data instead of
publishing it. Hashes belong only in declared `*_sha256` fields.

Report generation uploads nothing and starts no provider or evaluator. The MCP
`paperhands` tool accepts the same exact argv, omitting the binary name.
Read `automation` for opt-in scheduled report refresh and its private receipts.

## Bundled model attribution

The semantic encoder comes from [clap-how-to](https://github.com/Gonzih/clap-how-to)
and [pocket-encoder](https://crates.io/crates/pocket-encoder). Encoder code is MIT;
model and tokenizer are Apache-2.0, with upstream provenance, license and NOTICE
bundled in that dependency. The upstream model is
[sentence-transformers/paraphrase-MiniLM-L3-v2](https://huggingface.co/sentence-transformers/paraphrase-MiniLM-L3-v2).
Search JSON includes the exact encoder fingerprint and model identity.
