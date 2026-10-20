# Paperhands product design

Proposed direction, 2026-10-20. Paperhands ships a trading research practice inside
one Rust executable: CLI, MCP server, semantic help, documentation, skills and
working examples released together. An app sits above this engine, connects its
agent through ACP and presents research as an interactive workspace. Paperhands
supplies the instruments, durable state and execution.

[Portable storage design](storage-v2.md) defines the next object/revision/snapshot
format for S3/R2 and explicit merges across research instances. It separates shared
evidence from local execution authority; this backend is not shipped in 0.4.0.

The 0.3.0 source implements a local slice: frozen inputs, external evaluator
execution, bounded batches, explicit comparisons and reviews, semantic discovery,
JSON output and automatic skill maintenance. Existing evaluators remain
authoritative. It adds growing research memory, a task frontier, shared CLI/stdio
MCP and private renderer-neutral evidence views with embedded Pele presentation.
A project manifest binds the growing object/text store; the companion retrieves
direction, evidence and next work. Tasks, plans, research and provenance form a
DAG, with immutable revisions, declared attention weights, measured semantic
similarity and bounded local clusters. The status display is a projection.
The ACP app, persistent venue engines and broader research behavior below remain
proposed, awaiting detailed implementation briefs.

## Creative shapes

First pass: ten rough possibilities, before choosing a shape.

1. Observatory — notice changes before deciding what they mean.
2. Sketchbook — give incomplete observations somewhere to live.
3. Studio — make several drafts before selecting one to develop.
4. Workshop — assemble small, inspectable strategy components.
5. Garden — grow ideas, prune failures and revisit them when conditions change.
6. Field station — keep collecting evidence while the researcher is away.
7. Flight deck — make current positions, commitments and operating authority visible.
8. Laboratory — give every comparison a controlled, reproducible experiment.
9. Archive — recover abandoned work when a new question makes it useful.
10. Expedition — spend a bounded research budget on an unfamiliar mechanism.

Second pass: use **the studio** as the product shape, with **the laboratory** for
measurement and **the field station** for persistent operation. These give creative
work room to change direction while the running system remains inspectable.

The Censor to watch: “Every idea must become a profitable strategy immediately.”
That rule would discard observations before they can become testable questions.
Research can earn its cost by ruling something out or exposing a bad assumption.

## The research practice

Julia Cameron's practices include Morning Pages, Artist Dates, walks and writing
for guidance. Her guidance also gives early writing space before it is shared or
reread. The practices below are engineering adaptations, not a literal implementation
of the book. [Cameron's essential tools](https://juliacameronlive.com/the-essential-tools-video-course/),
[Cameron's group guidelines](https://juliacameronlive.com/faq/).

**Notice freely.** Begin a session with a sketchbook of surprises, contradictions,
missing information and unfinished questions. Keep sourced observations distinct
from speculation. No trading conclusion is required at this stage.

**Bring back outside input.** Spend a bounded part of the research budget on an
unfamiliar mechanism: auction design, inventory management, queues, forecasting or
how information moves through a community. Record the mechanism and where the
analogy breaks before proposing a transfer to a market.

**Generate before selecting.** Explore several materially different explanations.
Do not rank them while producing them. Review them in a separate pass and choose
the cheapest experiment that could distinguish the explanations.

**Preserve the useful remainder.** A rejected strategy may contain a useful signal,
data transformation or explanation. Archive those fragments with the conditions
under which they deserve another look. An archive item can return to exploration;
it has no direct path to deployment.

## What the binary owns

The operating philosophy is an empirical research desk. Mine observations,
mechanisms, executable conditions, accounting failures and useful negative
knowledge. The researcher chooses questions and may challenge any assumption.
Changing an evaluator creates a new measurement question. Retirement closes an
attempt and preserves the conditions under which another attempt could be useful.

Aim for capacity for 200 concurrent logical paper policies, sharing retained causal
observations with separate cash, inventory, pending orders, limits and ledgers.
Use bounded worker concurrency and shared immutable datasets. Measure wall time,
CPU, memory and retained bytes before promising a scaling rate. Strategy labels,
economic executions and independent market/time evidence have separate counts.
Counterfactual policies can reuse observed liquidity; a combined funded portfolio
requires its own capital/allocation/liquidity evaluator.

| Responsibility | Binary behavior |
| --- | --- |
| Market access | Fetch and timestamp observations; expose each venue's actual capabilities |
| Experiments | Run a pinned candidate against identified data and evaluation assumptions |
| Measurement | Compute results using explicit cost, fill and accounting models |
| Operation | Run deployed strategy versions, translate intentions into orders and reconcile account state |
| Memory | Preserve observations, hypotheses, trial results, decisions and their relationships |
| Agent interface | Return structured state, evidence references and concrete recovery actions |
| App interface | Expose the same operations, datasets, views and events to a visual client |

Use ordinary files for strategy source and large artifacts, Git for source history,
and a local SQLite journal for jobs and events. Keep this a local tool with a
persistent process when trading requires it. An interrupted agent session must not
erase an experiment or stop management of existing positions.

A strategy receives a defined market snapshot and produces desired positions or
order intentions. The engine holds venue credentials, evaluates the operating
mandate and manages submission. The candidate runs in an isolated worker with a
bounded data interface. Strategy code can evolve without rewriting account logic.

Record an intention before submitting it. If submission times out, retain an
unknown outcome and reconcile before retrying. On restart, reconcile positions,
fills and pending orders. Pausing stops new entries while continuing management
of existing exposure; closing positions is a separate explicit operation.

## The app and agent interfaces

Build the CLI, MCP and app API over one set of domain operations. They use the same
runtime, journal, job identities, validation and authorization. An app action and
an agent tool call must resolve to the same underlying research record.

```text
User <-> App <----------- ACP -----------> Agent
          |                                |
          | typed queries and events       | MCP tools and resources
          v                                v
                  Paperhands engine <------ CLI
                         |
            journal, jobs, datasets and views
                         |
               market and source adapters
```

ACP carries the conversation, session activity, tool-call updates and cancellation
between app and agent. The app supplies the Paperhands MCP connection during agent
session setup. This follows ACP's documented client/agent model and MCP server
configuration. [ACP overview](https://agentclientprotocol.com/protocol/v1/overview),
[ACP session setup](https://agentclientprotocol.com/protocol/v1/session-setup).

MCP exposes typed Paperhands operations to the agent: discover markets and sources,
collect evidence, start or inspect research jobs, compare experiments, retrieve
artifacts and create views. Resources make bundled skills, schemas, evidence and
datasets discoverable. The UI reads those same datasets directly through the app
API; refreshing a chart does not require an agent turn.

Expose a local stdio MCP entry point from the binary, with Streamable HTTP when
needed by the app or agent adapter. MCP returns schema-described structured data
and resource references. Its stdout contains protocol messages; diagnostics and
skill-refresh notices use stderr. Domain results remain consistent with CLI JSON
while each transport uses its required envelope.
[MCP tools and structured results](https://modelcontextprotocol.io/specification/2026-07-28/server/tools),
[MCP transports](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports).

An MCP connection attaches to the selected engine and workspace. Sessions do not
create competing copies of the journal or trading process. ACP Stop ends the agent
turn; explicit job cancellation or trading controls determine what happens to
durable work. Resuming the app reloads job state and events after its saved cursor.
Report ACP connection and actual MCP tool discovery separately.

The first app experience should automate market digging: a user defines a question
and watchlist, the agent gathers sourced evidence, and a recurring campaign returns
changes, comparisons and new questions. These workflows work before connecting a
trading account. The engine holds schedules and budgets; an available ACP worker
executes research turns. If no worker is available, a due run remains visibly queued.

## What the skills contain

Ship one entry skill and five focused skills. These are work modes that one agent
can use; they do not require six agents or a new orchestration framework.

| Skill | Practice | Durable result |
| --- | --- | --- |
| `paperhands` | Read campaign state, resume work, route to the next useful mode | Current question, budget and checkpoint |
| `paperhands-observe` | Gather evidence and notice surprises without forcing explanations | Observations with sources and times |
| `paperhands-explore` | Bring outside input, generate alternatives, then select a question | Hypotheses and a distinguishing experiment |
| `paperhands-experiment` | Implement a candidate and run a registered comparison | Candidate revision, result and artifacts |
| `paperhands-review` | Challenge explanations, compare evidence and retain useful fragments | Disposition, lesson and revisit conditions |
| `paperhands-operate` | Inspect deployments and investigate changes in live behavior | Operating decision linked to evidence |

Each skill explains when to use it, the questions to ask, which CLI operations
supply facts, the record to leave and how to continue after failure. Command
syntax and schemas come from bundled references generated from the actual CLI.
The skills teach judgment and research craft without copying the command manual.
Each operation has CLI and MCP instructions from the same reference. In-app skills
also teach the agent to retrieve datasets, produce view records, cite evidence and
connect a user's selection in a chart to the next research question. Reuse these
skills in the app and standalone harnesses.

The exploration prompt should ask: “What observation does our explanation fail to
explain? Produce distinct mechanisms, then choose a test that would separate them.”
Review should ask: “What did this failure teach us? What would make it worth another
attempt?” A lack of evidence remains an open question in the journal.

## Two connected loops

```text
Agent and skills
  observe -> explore -> candidate -> experiment -> review -> next question
                                       |             |
                                       v             v
                                  evidence journal and deployment decision
                                                     |
                                                     v
Persistent engine
  observations -> pinned strategy -> intentions -> orders -> fills -> observations
```

The research loop explores new versions. The trading loop operates a specific
deployed version. Deployment records what happens to existing positions and open
orders when the strategy changes. Research, data, evaluator, engine and skill
versions remain attached to every experiment and deployment.

The operator's mandate defines venues, instruments, budgets, exposure, expiry and
which transitions can happen autonomously. It can authorize recurring research,
promotion and trading without asking about each order. The binary enforces the
mandate and reports any unmet condition through structured state.

Historical replay, forward paper operation and live operation remain distinct
evidence types. Pin both the market period and compute budget for comparisons.
Record every attempt, evaluation assumptions and the validation periods already
used to guide edits. Define promotion criteria before selecting a winner. Exact
metrics and thresholds belong to the forthcoming specification.

## The central research record

An idea should remain readable as a compact research card:

| Field | Meaning |
| --- | --- |
| Observation | What happened, where, when and according to which source |
| Hypothesis | The proposed mechanism |
| Prediction | What we expect if the mechanism is useful |
| Experiment | Candidate, comparison, data and assumptions |
| Evidence | Measurements and artifact references |
| Disposition | Continue, revise, archive or nominate for deployment |
| Revisit condition | What new evidence or market condition would change the decision |

The journal connects these records to experiments, deployments, orders and fills.
An explanation of a position should trace back to its strategy revision, triggering
observation, deployment authority and supporting experiments. Sketches can remain
messy; a deployment decision needs resolvable evidence.

## Renderable data and visual research

Make every visual a saved view over identified data. Paperhands supplies values,
field types, units, provenance and presentation intent. The frontend chooses the
rendering library, layout and interaction. Preserve a renderer-neutral dataset
alongside optional library-specific projections or cached exports.

| View family | Data supplied | Research use |
| --- | --- | --- |
| Time series | Timestamped series with units and observation times | Prices, probabilities, volume, returns and drawdown |
| Market structure | OHLC bars or book levels with venue, side, price, size and timestamps | Candles, depth, spread and liquidity views |
| Network | Typed nodes and edges with stable IDs and evidence references | Markets, entities, sources, hypotheses and experiment lineage |
| Matrix or heatmap | Row/column identities, values and missing-data meaning | Relationships, parameter sweeps and experiment comparisons |
| Distribution | Samples or bins with counts and weighting | Returns, uncertainty and execution outcomes |
| Table and metrics | Typed columns, sortable records and metric definitions | Market screening, source inspection and campaign reports |
| Diagram | Mermaid source, title and linked evidence | Research workflows, explanations and decision paths |

Pele renders Mermaid to SVG and supports CSS-variable theming. Bundle a pinned
Pele version in the app for diagrams. Its adapter can mount, update and clean up
views as the workspace changes. [Pele introduction](https://pele.run/),
[Pele API](https://pele.run/api).

For an evidence network, retain canonical nodes and edges and offer a Mermaid
projection for a compact Pele explanation. An interactive network renderer can
consume the same graph. Numeric charts consume their underlying series or matrix.
That keeps data usable across frontend libraries and future export formats.

A view records its schema version, identity, title, dataset revision, filters,
transformations, field encodings and evidence references. Include freshness,
coverage, missing values, aggregation and downsampling information. Large results
return a bounded preview and a reference to paged or downloadable data. Exact
field names remain part of the forthcoming schema specification. Preserve source
precision and explicit currency, instrument units and timezone through export;
display conversions do not replace the underlying values.

Mark computed views and agent-authored illustrations distinctly. A relationship
edge identifies its evidence and whether it is observed or hypothesized. Computed
values come from engine results; selecting a chart mark or graph node resolves to
its source records. A visible connection alone does not establish causation.

The app provides linked selections, time brushes, comparison overlays, clear units,
consistent colors, theme support and source drill-down. Users can move from a
market chart to an event, from the event to its evidence, and from that evidence
to a follow-up question for the agent. Keep raw data and source accessible from
every view, with SVG/image export as a convenience.

Bound agent-authored diagram source and output, sanitize generated SVG, and control
links and external images in the app. Display rendering errors as text with the
source still available. [Pele's rendering guidance](https://pele.run/security).

## CLI and release behavior

Keep the established contract: JSON by default for results, help, version and
errors; `--human` selects readable text. Offline `clap-how-to` search finds real
commands and bundled documentation. Discovery never executes a retrieved command.

Long operations return a job identity with inspectable progress, final outcome and
resume behavior. Observation, candidate, experiment, deployment and order records
have stable identities. Errors distinguish failure from an operation whose outcome
is still unknown. Commands expose state without silently starting trading.

The binary installs its embedded skill bundle on first invocation and repairs
changed managed files on every run. A new binary refreshes skills, references and
schemas from the same release. Personal research direction lives in a separate
user-owned file. Inspection commands can list or display skills; manual installation
is not required for the normal path.

CI verifies the release as one instrument: build the binary, validate skill
references, regenerate the semantic index, check schemas and exercise the bundled
example against a deterministic fake venue. Publish the crate and release binaries
with the same bundle identity. A release is usable without separately fetching
prompts, documentation or its help model.

Include CLI/MCP result parity, ACP-to-MCP discovery, reconnecting job streams and
renderer fixtures in that release check. The app negotiates engine and view-schema
capabilities and makes unsupported features explicit. Bundle frontend assets with
the app release; the engine remains usable through CLI and MCP independently.

The first complete vertical slice can use one venue, one account and one strategy
interface through replay, paper and live operation. Reports should expose what
surprised the system, which questions it tested, what it learned, what is running
and which question deserves the next experiment.

Existing local implementation references: `../ontology/src/chat.rs:206` configures
ACP sessions with HTTP/stdio MCP; `../ontology/src/diagram.rs:36` returns Mermaid
and rendering metadata; `../ontology/src/dashboard/diagrams.js:73` mounts a local
Pele viewer. These demonstrate the integration shape; shared package boundaries
and reusable code require a separate implementation decision.

Gap: first venue, market data, strategy interface, evaluator, deployment policy,
capital limits, local versus hosted app deployment, ACP adapters, frontend library,
view schemas and release targets — no detailed operator specification yet.
