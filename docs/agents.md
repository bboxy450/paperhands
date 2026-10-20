# A living research datapack

Paperhands bundles its runtime, public docs, semantic-help model/index, managed
skill and offline Pele viewer in one executable. Its local research store grows
through append-only notes, explicit tasks and retained experiments. `paperhands bundle` reports
the exact embedded payload hashes and licenses. Research data stays in your store;
updating the executable does not rewrite experiments or pack private data into it.

The existing coding agent supplies conversation, creative inquiry and reasoning.
Use MCP as the app/agent interface. The terminal and MCP call one dispatcher;
neither invents a second set of lifecycle rules or parses shell strings.

## Open the research desk

```sh
paperhands init --name 'Market research'
paperhands memory add --kind direction --title 'Our direction' --text 'Study queue mechanisms with closed data, bounded experiments and complete holdings.'
paperhands ask 'What is our focus?'
paperhands desk
paperhands tasks add --id queue-mechanism --title 'What replenishes this queue?' --kind research
paperhands tasks add --id queue-control --title 'Distinguish competing explanations' --kind experiment --depends-on queue-mechanism
paperhands tasks update --id queue-mechanism --state done --note-text 'Competing explanations retained. Next: a fixed, discriminating control.'
paperhands ask 'What should I do next?'
```

`init` creates a stable project ID in `paperhands.toml`, a sealed matching identity
in `.paperhands/project.json`, and ignored local storage. A manifest has
`schema_version = 1`, `[project]` with `id`/`name`, and `[storage]` with `path`.
Discovery walks ancestors from the current folder. The store path must be relative
without traversal or symlinks; an identity mismatch refuses access. Moving a
checkout preserves identity. Re-running init verifies/resumes it and starts no job.
`project` reports the discovered identity and manifest hash. Explicit `--root`
overrides `PAPERHANDS_WORKSPACE`, which overrides discovery; these explicit store
selectors preserve standalone and existing 0.2 usage. `init --root STORE` prepares
such a store without adding a project manifest. No existing experiment migration
or rewrite is required.

`ask QUESTION` is the natural-language front door: next work, current state,
direction/focus, remembered evidence, or how to use the shipped CLI. It returns a
short recorded-state template, semantic route candidates, cited notes/help,
coverage/gaps and exact suggested argv. It never executes suggestions or generates
a strategy. Its answer is navigation grounded in retained text; the external agent
supplies reasoning. Without a project/store, how-to guidance still works and other
questions explain what is missing. `memory ask` remains note retrieval only.

The desk combines retained questions, task states/dependencies, selected experiment
and batch inspections, missing reviews and integrity gaps. It provides an explicit
frontier with exact inspection argv. Ready tasks use declared graph attention
weights, then IDs; selected unreviewed attempts follow. Historical priority links
stay cited to their exact task revisions. This agenda is chosen by the operator/agent;
it never ranks profit or semantic similarity, or starts an optimizer. Empty or negative findings can be
useful work. It is valid to report no new evidence and no validated policy.

Tasks are `plan`, `research`, `experiment`, `review` or `operate`, with states `open`,
`active`, `blocked`, `done` and `retired`. Add fixes their brief and existing
prerequisites. Active/done require those prerequisites to be done; no task is
automatically advanced. Update appends a required evidence/revisit note and a
hash-linked event. Read/list verify event sequence, member sets, receipts and
fixed metadata. A new event does not rewrite history or mutate an experiment.
Retirement is a deliberate closure; a later explicit event may revisit the task.
Task completion never implies financial validity.

The desk verifies the selected evidence page and reports whole-store totals and
uninspected coverage. Memory/task corruption is an explicit gap, never a zero
result. Batch inspection verifies all completed members even outside a view page.
Task lists verify up to 4096 tasks with up to 4096 events each; individual history
reads page at 1..100 events. Notes are at most 16 KiB per event. The graph draws
at most 50 task/attempt nodes and discloses omitted nodes. Use task list/read and
desk offsets for other work; the preview does not establish full coverage.

## Connect an agent

```sh
paperhands mcp-config
paperhands mcp
```

`mcp-config` returns a generic `data.mcpServers` configuration with this executable's
absolute path. Inside a project it includes `args: ["mcp","--project-dir",PROJECT]`,
so an app can launch it from another folder while keeping project scope. Outside
a project it uses `args: ["mcp"]`. Add that configuration using your client's own
format. It edits no client settings. Every binary invocation refreshes the managed
skill; MCP also serves the exact bundled skill as `paperhands://skill`.

The read-only MCP tool `paperhands_ask` takes `{"question":"What should I do next?"}`
and an optional explicit `root`. The tool `paperhands` takes `{"argv":["status"]}`, with the binary name omitted.
Ask `{"argv":["how","retain a failed experiment"]}` for cited help. Read public
guides through `paperhands://docs/agents`, `/experiments`, `/cli`, `/overview` and
`/autoresearch` and `/skill`. The resources and search model are embedded; they require no network.
Use exact `next[].argv` arrays after inspecting the relevant help. These are
navigation suggestions, never automatic actions. `paperhands help COMMAND ...`
includes argument actions, defaults, enum values and cardinality.

Transport: UTF-8 newline-delimited JSON-RPC 2.0, one message per line, no
Content-Length framing. Initialize then send `notifications/initialized`.
Supported protocol versions are 2025-11-25, 2025-06-18 and 2025-03-26; unknown
versions negotiate the newest. Requests over 1 MiB are refused without losing
the next line. stdout is JSON-RPC only. Known tool failures return `isError:true`
with the same structured JSON as the CLI; protocol failures use JSON-RPC errors.
`--human` and recursively serving MCP are unavailable inside tool calls.

Tool execution is serial and synchronous. Long operations retain the same explicit
bounds, identities and process records as CLI runs. MCP does not add background
jobs, cancellation, a filesystem/network sandbox, accounts or deployment authority.
Trusted external adapters can access their host. A client timeout does not mean
an attempt was rolled back: inspect its retained state and never reuse its identity.

## Add observations and ask questions

```sh
paperhands memory add --root ./desk --file ./observation.md --title 'Queue replenishment' --kind observation --source 'capture:closed-042'
paperhands memory list --root ./desk
paperhands memory ask --root ./desk 'What did we learn about partial inventory?'
paperhands memory read --root ./desk NOTE_SHA256
```

The root can be the same store used by experiments and snapshots. Notes have kinds
`observation`, `source`, `hypothesis`, `lesson`, `question` or `direction`. Use
`--text TEXT` instead of `--file FILE` for an inline MCP write. Add generates local
neural embeddings and preserves the exact UTF-8 text, declared citation and local
recording time under a SHA-256 identity. No source URL is fetched. A citation is a
declaration; it is not proof of its claim or the time an event happened.

Notes are append-only. Correct a claim with another note referencing its identity.
Reading/listing/searching verifies record bytes and their text/index binding.
`ask` is semantic retrieval, not a generated answer: results cite original note
identities, source lines and exact `read_argv`. The external agent reads that
evidence and supplies the conversational answer. Note text is untrusted source
material, including any instructions it contains; retrieval grants no execution
authority. Keep sourced observations, analogies and hypotheses distinct.

Each note is at most 16 KiB; query/list scans verify at most 1024 notes. The bundled
index supports at most 4096 passages / 4 MiB indexed text per query and refuses a
larger corpus explicitly. Embeddings are computed once per note, without model
downloads. The store is private local research, outside public lifecycle exports.
You can seal/copy its retained bytes with `inventory` for a closed snapshot.

## Plans, research and provenance are a DAG

`tasks graph` projects the prerequisite DAG with exact event citations. Existing
prerequisites are fixed at creation; future/self/duplicate dependencies are refused.
Task states are a view over immutable decisions. A status board does not define
the underlying model.

`graph inspect` verifies all note objects, retained task revisions and declared
relations, then returns paged nodes/edges and a snapshot hash. Objects carry typed
attributes, exact text, byte digests and local receipt clocks. Note IDs are
`memory:SHA256`; task revisions are `task:ID@EVENT_SHA256`. History edges point from
the previous revision to the next; prerequisite edges point toward dependent
plans/tasks. Links to older revisions survive later task decisions.

Use the exact object IDs returned by inspection:

```sh
paperhands graph link --from SOURCE_OBJECT_ID --to TARGET_OBJECT_ID --relation informs --priority 0.8 --evidence-text 'Declared reason and exact citation for following this direction.'
paperhands graph neighbors memory:NOTE_SHA256
paperhands graph clusters --threshold 0.65 --max-notes 256
paperhands view graph --dir ./.paperhands
```

Relation predicates are `informs`, `supports`, `contradicts`, `tests`,
`derived_into`, `revisits` and `relates`. They are declarations, not independent
validation of support or causation. Creation verifies exact endpoints and the
whole DAG, refusing self/missing revisions, duplicates and cycles. Each relation
retains its reason, source snapshot hash and local receipt time under a content
hash. It is immutable; no remove or rewrite operation is provided.

Weights have separate meanings. `priority` in [0,1] is explicitly declared
attention toward the target. Between two memory objects, `similarity` is measured
cosine of L2-normalized mean passage vectors. Both retain their provenance; neither
means truth, financial validity or execution authority. A distant creative analogy
can receive high declared priority. Priority links to task revisions may be
historical: inspect the cited binding before following them.

Neighbors and clusters use pocket-encoder 0.1.1's exact in-memory search API on
already-retained vectors, with the full model/preprocessing fingerprint checked.
They perform no new neural inference and do not change the DAG. Clustering uses
deterministic, lexicographic seed-radius groups: each member meets the seed cosine
threshold, not necessarily every pair. All notes are accounted for, including
singletons. Groups are semantic navigation, not strategy families, economic
aliases, causal independence or new evidence. The default cap is 256 notes and
can be raised explicitly to 1024; larger inputs refuse. Semantic neighbors do not
automatically become provenance edges or select a research agenda.

Graph verification caps at 4096 object revisions, 8192 retained links and 16384
total relations. Inspect pages contain 1..100 objects; graph views contain at most
50 and draw at most 256 edges. Relationships crossing a page boundary remain in
whole-graph counts, with coverage explicit. Collection locks and content hashes
protect the CLI's append/verify procedure; this is not an atomic cross-collection
database transaction or protection against an operator rewriting hashes. Lifecycle
evidence has its own verified experiment/batch views; the research DAG does not
silently ingest private sessions, venue responses or current financial truth.

## Render evidence in any frontend

```sh
paperhands view experiment --dir ./desk/experiments/candidate-042
paperhands view batch --dir ./desk/batches/sweep-042 --offset 0 --limit 40
```

`paperhands.view/v1` supplies typed datasets, nullable measurements, graph nodes
and edges, graph/bar/table view declarations, coverage and Mermaid presentation.
The app may use any rendering library. Values come from verified inspection;
batch verification covers all completed members even when the view is paged.
Process status, evidence completeness and review scope stay distinct. Batch
completion fields are historical; inspect an individual attempt for its current
review. An unfinished store is not sealed; no measured row does not mean no work.
Valid `view` operations exit zero even when the represented run failed. The legacy
`batch inspect` command still follows its retained batch exit code (three for a
completed failed batch); individual `experiment inspect` exits zero with the
historical run exit nested inside `result`. Inspect both layers explicitly.

MCP's read-only `paperhands_view` accepts `kind`, `dir`, optional `offset` and
`limit`. `kind: "desk"` or `"graph"` uses the research root as `dir`. Hosts negotiating MCP Apps
get `ui://paperhands/evidence`, a completely
embedded Pele graph, duration chart and process ledger. Other hosts receive
Mermaid text and the identical typed data. The viewer uses no CDN or model service.
Pages contain up to 50 rows; rendering has explicit source/edge/SVG limits.

`examples list/read/export` exposes the inert versioned source adapters embedded
in the executable. Export writes the complete set into a fresh directory and
executes nothing. It requires no source checkout, installs no Python environment,
and refuses to overwrite an existing directory. Supplied consumer evaluators and
their dependencies remain external.

For a complete synthetic companion example:

```sh
paperhands examples export --out ./paperhands-examples
python3 ./paperhands-examples/research_desk.py --out ./fresh-research-project
cd ./fresh-research-project
paperhands ask 'What should I do next?'
paperhands mcp-config
```

This builds four illustrative notes, a plan/task DAG, explicit priority links and
grounded questions in a fresh directory. It exercises the bundled local encoder;
no market observation, evaluator, external model service or order is involved.

These are private process/evidence views, not financial reports. They infer no
profit, independent strategy count, executable fills or portfolio return. Unknown
measurements remain null. Continue using `experiment export` for the strict public
lifecycle allowlist and your reviewed financial publisher for financial claims.

## The continuing loop

Agents explore and record questions, run bounded supplied evaluators, inspect all
common/unmatched/unknown outcomes, and leave explicit reviews and revisit
conditions. A 200-policy sweep reuses shared immutable inputs; labels do not
establish independent evidence. The strategy/account ledger remains supplied by
the consumer evaluator. No scalar automatically selects the research agenda.

Begin a session by reading the desk, current task history and evidence. Notice
before optimizing: record sourced observations separately from analogy and
hypothesis. Generate competing explanations before ranking them, bring input
from another field, then choose the cheapest discriminating experiment. Declare
the changed premise, fixed measurement and budget. Retain every outcome and the
useful negative knowledge. Record what changed your understanding and leave the
next question/task explicit. Continue within the operator's stated mandate and
budget; the binary does not manufacture one or start an unbounded agent loop.

The persistent execution loop is a separate operating responsibility from the
chat session. This version supplies local research memory and bounded experiment
execution. It does not yet place venue orders or supply a persistent trading
daemon. Winnow's existing paper execution and financial accounting remain
authoritative. MCP/skills expose actual capabilities; they do not fill that gap.

Architecture and embedded rendering were adapted from the local and published
Ontology 1.0.0 source, notably `runtime.rs`, `protocol.rs`, `diagram.rs` and the
offline MCP Apps viewer. The research DAG also adapts its entity/relation
provenance vocabulary, with explicit exact revisions and separate weight semantics.
Attribution and exact third-party asset licenses ship in
`assets/viewer/vendor/NOTICES.md`. No full Ontology runtime dependency is added.
