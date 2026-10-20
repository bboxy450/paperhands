---
name: paperhands
description: Discover and use the Paperhands CLI for autonomous trading research. Use for Paperhands commands, bundled docs and capability discovery.
---

# Paperhands

Start with `paperhands status` to inspect this binary's actual capabilities.
Version 0.1.0 is a scaffold; trading, backtesting and venue integrations await
the operator's specification. Do not report planned capabilities as implemented.

Ask the binary for relevant commands and source excerpts:

```sh
paperhands how "what can this version do?"
paperhands cli search "show installed capabilities"
paperhands docs search "how are agent skills updated?"
paperhands docs list
paperhands docs read cli
```

JSON is the default for results, help, version and errors. Use `--human` only
when readable text is requested. Search is offline semantic retrieval using
`clap-how-to`; inspect the cited source or command help before choosing an action.
Search never executes a result and does not generate an answer.

The binary installs and refreshes this managed file on every invocation.
Local edits are overwritten. Keep personal additions in separate files.
`PAPERHANDS_SKILLS_DIR` overrides the complete skill directory; the default is
`~/.agents/skills/paperhands`. Run `paperhands skills` to inspect installation.
