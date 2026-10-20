# Karpathy's autoresearch

Researched 2026-10-20, after publishing Paperhands 0.1.0. Source snapshot:
[`karpathy/autoresearch` at `228791f`](https://github.com/karpathy/autoresearch/tree/228791fb499afffb54b46200aca536f79142f117),
the upstream HEAD inspected on that date. This is source analysis; no training
experiment was run for this report.

## What it is

Autoresearch gives a coding agent a compact language-model training experiment
and lets it repeatedly change the implementation, measure a result and retain
improvements. It adapts nanochat to a single GPU. The human shapes the research
instructions; the agent searches within that brief. The upstream setup targets
NVIDIA hardware, with H100 listed as tested, and uses Python and uv.
[`README.md:7`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/README.md#L7),
[`README.md:23`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/README.md#L23).

The reusable mechanism is a short feedback loop with a restricted edit surface,
a repeatable measurement and retained experimental history.

## The three boundaries

| Source | Responsibility | Who changes it during a run |
| --- | --- | --- |
| `program.md` | Research instructions and experiment procedure | Human defines the brief |
| `train.py` | Model, optimizer, hyperparameters and training loop | Agent experiments here |
| `prepare.py` | Data preparation, loading, tokenizer, evaluation and fixed constants | Held fixed |

These boundaries are documented in
[`README.md:11`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/README.md#L11)
and reinforced in
[`program.md:25`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/program.md#L25).

## How an experiment proceeds

The instructions establish a fresh branch and an unchanged baseline. Each trial
edits `train.py`, commits the candidate, runs training with logs redirected,
extracts measurements and records the outcome. Better candidates advance the
branch; worse candidates are reset. Crashes are recorded and either repaired or
abandoned. The instructions set a ten-minute failure timeout and continue the
loop until the operator stops it.
[`program.md:9`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/program.md#L9),
[`program.md:94`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/program.md#L94).

The experiment ledger is an untracked `results.tsv`: candidate commit, validation
score, memory usage, disposition and description. The brief also favors simpler
code when gains are negligible. There is a small tension: that simplicity rule
allows equivalent results, while the numbered loop rejects equal scores. The
agent must interpret that choice.
[`program.md:37`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/program.md#L37),
[`program.md:64`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/program.md#L64).

## What is measured

The objective is lower validation bits per byte (`val_bpb`). The data loader
excludes the pinned validation shard from training. Evaluation adds token losses,
normalizes by target byte lengths and converts to bits; zero-byte special tokens
are excluded. Sequence length and evaluation budget are constants. The evaluation
step count uses integer division by batch size times sequence length, so its
realized token count can vary slightly with batch size.
[`prepare.py:254`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/prepare.py#L254),
[`prepare.py:344`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/prepare.py#L344).

The training budget is 300 seconds. The implementation starts accumulating step
time only when `step > 10`, excluding the initial steps used for compilation and
warmup. It stops at a step boundary, then evaluates. Consequently, a full process
takes longer than five minutes and can overshoot the measured training budget by
a step. The printed summary distinguishes training time from total time and
includes validation score, memory, throughput-related measurements and model size.
[`prepare.py:27`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/prepare.py#L27),
[`train.py:578`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/train.py#L578),
[`train.py:602`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/train.py#L602).

## Where the guarantees end

The inspected training script seeds CPU and CUDA RNGs, runs one candidate and
prints one evaluation. It does not perform repeated-seed comparisons, uncertainty
estimation or a separate final-test gate. Its model and evaluation call live in
the editable training file.
[`train.py:458`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/train.py#L458),
[`train.py:610`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/train.py#L610).

Inference: the edit boundary relies on agent adherence. A fixed evaluator file
alone cannot prove the editable caller used it honestly. Repeated selection on
one validation set can also select noise. The approximate experiment throughput
advertised upstream omits variable reasoning, compilation and evaluation costs;
equal training budgets mainly support comparison on the same hardware.
[`README.md:63`](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/README.md#L63).

## Transfer to Paperhands

These are design implications, pending the operator's specification:

| Autoresearch mechanism | Trading-research analogue |
| --- | --- |
| Human research brief | Hypothesis space, objective, allowed changes and budgets |
| Editable training candidate | Strategy or research implementation |
| Fixed evaluator and data | Versioned market replay, accounting and execution assumptions |
| Bounded experiment | Fixed historical window and resource budget |
| Candidate commit and ledger | Reproducible strategy revision, data identity, metrics and disposition |
| Keep/discard decision | Explicit promotion rule with independent validation |

Inference: keeping a fixed market horizon matters separately from keeping a fixed
compute budget. A faster strategy must not receive an easier test period. A replay
score also depends on fees, spread, latency, partial fills and available information
at decision time. Those assumptions belong to the evaluator, outside the candidate's
authority.

Further inference: repeated historical search needs an untouched evaluation period
and recorded trial counts. Candidate promotion and authority to place live orders
are separate decisions. This loop can organize research; it does not establish a
profitable strategy or grant a broker connection.

Gap: venue API support, data sources, instruments, execution model, objective,
promotion criteria and capital limits — no operator specification yet. Polymarket,
Robinhood and other platforms remain intended integrations. Paperhands 0.1.0 ships
CLI discovery, documentation and skill maintenance only.
