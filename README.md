# Paperhands

An agent-first Rust binary for an autonomous trading research system, inspired
by Andrej Karpathy's [autoresearch](https://github.com/karpathy/autoresearch).
Mine observations, test mechanisms and retain what each experiment teaches.
Many policy names can describe one economic outcome; keep that distinction visible.

Paperhands is a living local research datapack: one executable carries its agent
interface, docs, skill, offline semantic encoder and Pele evidence viewer. Its
append-only objects, exact text and research DAG grow beside retained experiments.
Agents converse through MCP, add observations/questions, retrieve cited notes and
run experiments through the same operations as the CLI. `paperhands bundle`
identifies what ships; `paperhands ask "What should I do next?"` follows recorded
direction, ready work and cited evidence. `paperhands.toml` binds a stable project
identity across subfolders. Plans, tasks and research are a DAG; state boards are
projections. Provenance links retain exact revisions, measured similarity and
separate declared attention weights.

Version 0.3.0 runs trusted external evaluators over frozen local inputs: exclusive
experiment identities, bounded execution and batches, retained process evidence,
explicit case comparison, review and sanitized lifecycle export. A 200-member
synthetic fixture exercises four workers and partial failures. This is not a
200-strategy live performance claim. Existing consumers supply strategies,
financial accounting and diagnostics. Venue connections, account simulation,
orders, a persistent trading daemon and the ACP app remain future work.

## Install

```sh
cargo install paperhands --locked
paperhands status
paperhands how "what can this version do?"
paperhands docs list
paperhands docs read cli
paperhands docs read experiments
paperhands docs read agents
paperhands mcp-config
paperhands init --name 'Market research'
paperhands memory add --kind direction --title 'Research direction' --text 'Explore mechanisms; use closed inputs and retain unknown holdings.'
paperhands ask 'What is our focus?'
paperhands desk
paperhands graph inspect
paperhands graph clusters
paperhands view graph --dir ./.paperhands
paperhands --human --help
```

## CLI principles

- Semantic help uses our `clap-how-to` crate, following the discovery pattern
  used by Cloudflare's `cf` CLI, Ontology and ICQ.
- All output is JSON by default, including help, version and errors.
  `--human` explicitly selects readable text.
- Commands, documentation, prepared embeddings, MCP, the Paperhands agent skill
  and offline Pele viewer travel inside one binary. Semantic help and research
  memory use `clap-how-to` and `pocket-encoder` locally.
- Every invocation installs or repairs the bundled skill at
  `~/.agents/skills/paperhands/SKILL.md`. Changed bundled files are replaced even
  if edited locally or installed by a different version. Keep personal additions
  in separate files. `PAPERHANDS_SKILLS_DIR` overrides the skill directory itself.

See [agent integration and research memory](docs/agents.md),
[the experiment protocol](docs/experiments.md), [CLI contract](docs/cli.md)
and [bundled skill](skills/paperhands/SKILL.md). Runnable examples are under
[examples](examples). The broader ACP app and persistent trading architecture
are described in [design notes](docs/design.md).

## Develop

```sh
cargo run --example prepare_help
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo publish --dry-run --locked
```

Regenerate the help and companion route indexes after changing commands, guides,
skills or route sources. Source-freshness tests catch stale indexes. The query encoder and model
attribution are supplied by `clap-how-to` and its `pocket-encoder` dependency
(MIT code; Apache-2.0 model/tokenizer). Use `paperhands docs read cli` for details.
