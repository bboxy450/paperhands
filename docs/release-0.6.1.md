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
