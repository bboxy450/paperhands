# Paperhands

An agent-first Rust binary for an autonomous trading research system, inspired
by Andrej Karpathy's [autoresearch](https://github.com/karpathy/autoresearch).
Mine observations, test mechanisms and retain what each experiment teaches.
Many policy names can describe one economic outcome; keep that distinction visible.

Version 0.2.0 runs trusted external evaluators over frozen local inputs: exclusive
experiment identities, bounded execution and batches, retained process evidence,
explicit case comparison, review and sanitized lifecycle export. A 200-member
synthetic fixture exercises four workers and partial failures. This is not a
200-strategy live performance claim. Existing consumers supply strategies,
financial accounting and diagnostics. Venue connections, account simulation,
orders, MCP and the ACP app remain future work.

## Install

```sh
cargo install paperhands --locked
paperhands status
paperhands how "what can this version do?"
paperhands docs list
paperhands docs read cli
paperhands docs read experiments
paperhands --human --help
```

## CLI principles

- Semantic help uses our `clap-how-to` crate, following the discovery pattern
  used by Cloudflare's `cf` CLI, Ontology and ICQ.
- All output is JSON by default, including help, version and errors.
  `--human` explicitly selects readable text.
- Commands, documentation, prepared embeddings and the Paperhands agent skill
  travel inside one binary. Semantic help runs offline.
- Every invocation installs or repairs the bundled skill at
  `~/.agents/skills/paperhands/SKILL.md`. Changed bundled files are replaced even
  if edited locally or installed by a different version. Keep personal additions
  in separate files. `PAPERHANDS_SKILLS_DIR` overrides the skill directory itself.

See [the experiment protocol](docs/experiments.md), [CLI contract](docs/cli.md)
and [bundled skill](skills/paperhands/SKILL.md). Runnable examples are under
[examples](examples). The proposed research studio, ACP/MCP app boundary and
renderable evidence graphs are described in [design notes](docs/design.md).

## Develop

```sh
cargo run --example prepare_help
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo publish --dry-run --locked
```

Regenerate `assets/how-to.json` after changing commands, README, CLI docs or the
skill. Source-freshness tests catch stale indexes. The query encoder and model
attribution are supplied by `clap-how-to` and its `pocket-encoder` dependency
(MIT code; Apache-2.0 model/tokenizer). Use `paperhands docs read cli` for details.
