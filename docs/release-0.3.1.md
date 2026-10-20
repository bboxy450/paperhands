# Paperhands 0.3.1

`--human ask` now prints the companion's concise answer. In 0.3.0 a nested help
object shadowed the answer selector, causing the readable renderer to dump the
whole structured context. The selector now chooses the first textual field.
JSON/MCP results, retained schemas, lifecycle execution and model/dependency pins
are unchanged.

The CLI regression exercises the actual companion and refuses a context dump.
Against the same local project, the observed output changes from 7,752 bytes to
189 bytes. Format, strict all-target Clippy, prepared-index freshness and the full
28-test suite pass. The 0.3.0 exact-registry acceptance remains the engine baseline.

Publication checksum, source and installed-artifact verification follow after
release. Consumer feedback and adoption remain owned by Winnow.
