# Paperhands 0.6.1

Prepared `automation status` works when the project store denies writes. It opens
the existing automation lock read-only, takes a fail-fast shared lock, then
validates existing control directories. It does not create directories or locks,
repair permissions, send messages or initialize empty automation state. Missing,
invalid or busy state remains an error. Other automation operations retain their
existing writer setup, exclusive locking and spent/ambiguous-slot behavior.

No store format, output schema, dependency, recipient, schedule, memory volume,
experiment or financial-evidence migration is required. An unprepared project
must be explicitly configured with the existing automation commands before status
can inspect it. Managed-skill refresh still requires its separate writable
location or an already-current skill; this is project-scoped read-only access.

Three new regression tests cover populated status under native macOS project
write/network denial, exact member/byte/mode preservation, actual held shared and
exclusive locks, and missing/symlink/wrong-type control paths without repair.
The existing wake and cron fixtures retain original spent slots and job bytes.
All tests use owned temporary stores and fake transports; no consumer store,
timer, global activation or provider call is part of this patch.

`cargo test --locked` passes all 64 tests, including all 11 automation tests.
Strict all-target Clippy, format/diff checks and managed-skill validation pass.
Prepared offline help contains 257 sources and five companion routes. A direct
owned-fixture comparison under the same native write-denial profile reproduces
installed 0.6.0's exit 2 and patched 0.6.1's exit 0 with the original waiting
schedule, one spent wake and 59 remaining attempts. Exact file bytes, member sets
and permissions remain unchanged. Memory, task, graph, companion, MCP, lifecycle,
judge sources and the managed skill are byte-identical to 0.6.0.

The clean packaged tree also passes all 64 tests with locked offline resolution.
Publish dry-run and the actual upload succeed. Independent registry download
verification establishes an exact match to the tested upload archive and its
61 source payloads:

- Registry version `0.6.1`, publisher **bboxy450 / user 445014**, unyanked.
- Clean packaged source/tag `v0.6.1`:
  `e99f728ba8da00c072638ede0d7262bd4a32b65a`.
- Archive: 968,682 bytes, SHA-256
  `aad6382aa451c57bd2aa72ff0b0ef74e8b0d66ca8dad5cd19968f691bf2dc0ff`.
- Dependency pins remain `clap-how-to =0.1.4` and
  `pocket-encoder =0.1.1` with search.

The builder left the accepted global 0.6.0 binary unchanged. Consumer installation,
resident MCP refresh and independent acceptance remain consumer-owned steps.
Private command logs and receipts are retained outside Git and the crate.

Winnow independently inspected `e99f728` and passed all eleven focused source
automation tests: ten under outer network/live-settings-write denial, then the
native sandbox test separately using its built-in profile. This is not a consumer
rerun of all 64 tests. Its exact canonical-store reproduction retains installed
0.6.0's exit 2 and the source-built 0.6.1 binary's exit 0 under the same project
write/network denial. All seventeen automation byte/member/mode bindings remain
unchanged, with one spent wake, 59 unused attempts and the original nominal due
time preserved. The accepted global binary remains 0.6.0 at that check. This
source acceptance does not establish consumer registry or installed-artifact
acceptance of the already-published 0.6.1 release.
