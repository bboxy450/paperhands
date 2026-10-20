# Paperhands CLI contract

## Discover commands by intent

`paperhands how-to QUESTION` and its alias `how` search the real Clap command
tree and bundled guides. `paperhands cli search QUESTION` searches commands;
`paperhands docs search QUESTION` searches documentation. All three use
clap-how-to 0.1.3: local neural cosine similarity plus BM25 with reciprocal-rank
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
text selected by ID: `overview`, `cli`, `experiments` or `skill`. These are versioned snapshots
inside the executable, not arbitrary filesystem paths or live websites.

## Output

Every successful invocation writes one JSON object to stdout with `schema`,
`version`, `ok`, `command` and `data`. Help and version follow the same contract.
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

## Bundled model attribution

The semantic encoder comes from [clap-how-to](https://github.com/Gonzih/clap-how-to)
and [pocket-encoder](https://crates.io/crates/pocket-encoder). Encoder code is MIT;
model and tokenizer are Apache-2.0, with upstream provenance, license and NOTICE
bundled in that dependency. The upstream model is
[sentence-transformers/paraphrase-MiniLM-L3-v2](https://huggingface.co/sentence-transformers/paraphrase-MiniLM-L3-v2).
Search JSON includes the exact encoder fingerprint and model identity.
