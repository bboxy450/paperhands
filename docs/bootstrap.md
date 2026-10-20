# Initial publication

Direct operator request; role: Executor. Worktree: `/Users/feral/mydev/paperhands`.
Scope: initialize this empty repository, publish a dummy binary crate and public
`bboxy450/paperhands` repository, then record research on Karpathy's autoresearch.

Required: clap-how-to semantic help; JSON default and --human; embedded skills
installed on first run and aggressively refreshed; no trading implementation
before the operator supplies its specification. Default shared skill home is
`~/.agents/skills`. Publishing identities: GitHub bboxy450; crates.io boxy.

Checks: format, Clippy, integration tests for output/search/skill repair,
package inspection, cargo publish dry run, remote visibility/commit verification,
crates.io version and owner verification. Research follows the initial publication.

Verified 2026-10-20:

- `gh api user --jq .login` — `bboxy450`, exit 0.
- Crates.io settings/profile — Boxy, linked GitHub account bboxy450, verified
  email bboxy450@gmail.com. The signed-in account's token usage timestamp advances
  with authenticated local-credential probes. Tokens are never recorded here.
- `src/lib.rs:15` — real Clap command tree supplies semantic help documents.
- `src/lib.rs:95` — clap-how-to prepared vectors, scoped hybrid search, exact sources.
- `src/lib.rs:138` — first invocation installs the embedded skill; later runs
  compare bytes and atomically replace changed contents without touching other files.
- `src/main.rs:85` — errors are JSON on stderr unless --human; nonzero exits.
- `src/main.rs:97` — help/version also install skills and use JSON output.
- `cargo fmt --check` — exit 0.
- `cargo clippy --all-targets --locked -- -D warnings` — exit 0.
- `cargo test --locked` — exit 0, 1 index-freshness test and 5 integration tests passed.
- `uv run --with pyyaml python .../skill-creator/scripts/quick_validate.py skills/paperhands`
  — exit 0, skill valid. The system Python lacked PyYAML; isolated uv supplied it.
- `cargo package --list --locked --allow-dirty` — exit 0; source, lockfile,
  prepared help, CLI docs, skill, license, example and tests included.
- Manual semantic queries — skill maintenance ranks `paperhands skills` and
  `Agent skill lifecycle` first; capability discovery ranks `paperhands skills`
  first and `paperhands status` second.

No trading, broker access or strategy evaluation is implemented or tested.

Publication:

- `gh repo create bboxy450/paperhands --public --source=. --remote=origin ... --push`
  — exit 0; `gh repo view` confirms PUBLIC and remote main points to `31207a5`.
- `cargo publish --dry-run --locked` — exit 0; packaged crate rebuilt successfully.
- `cargo publish --locked` — exit 0; registry reports paperhands 0.1.0 published.
- Crates.io version API — binary name paperhands; publisher Boxy/bboxy450,
  user 445014; version 0.1.0 is not yanked.
- Crates.io owners API — sole owner bboxy450 (Boxy).
- Crate SHA-256 — `853c492f247dd3658f4196840f1d571dc8f2d81f691ff7865f11971668a51081`.
- `cargo install paperhands --version 0.1.0 --locked` — exit 0, release binary
  installed from crates.io at `~/.cargo/bin/paperhands`.
- Installed-binary smoke checks — version/status JSON, root/nested help,
  semantic command/docs search and repair of a deliberately stale skill all pass.
- Research source check — every GitHub source line anchor exists in the inspected
  immutable upstream snapshot; no source or benchmark was executed.

Post-publication research is in `docs/autoresearch.md`, citing immutable upstream
source at `228791fb499afffb54b46200aca536f79142f117`. No upstream training was run;
trading adaptations are marked as inferences, with specification gaps retained.
This research was written after the release and is repository documentation;
it is not part of the already published 0.1.0 binary's bundled guides.
