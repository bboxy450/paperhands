# Paperhands

An agent-first Rust binary for an autonomous trading research system, inspired
by Andrej Karpathy's [autoresearch](https://github.com/karpathy/autoresearch).
Planned venues include Polymarket, Robinhood and other platforms. The trading
specification is pending.

Version 0.1.0 is a working CLI scaffold: semantic command discovery, bundled
documentation and an automatically maintained agent skill. It has no market
connections, credentials, backtests, experiment runner or order execution.

## Install

```sh
cargo install paperhands --locked
paperhands status
paperhands how "what can this version do?"
paperhands docs list
paperhands docs read cli
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

See [the CLI contract](docs/cli.md) and [the bundled skill](skills/paperhands/SKILL.md).

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
