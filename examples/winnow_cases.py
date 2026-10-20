"""Adapt supplied Winnow cases; no independent fill or financial validation.

Canonical anchors and economic signatures follow the consumer-owned adapter.
Exact Decimal strings and complete episodes remain in context and raw artifacts.
"""
import argparse
from collections import Counter
from decimal import Decimal
import json
import math
from pathlib import Path


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'))


def number(value):
    if value is None:
        return None
    decimal = Decimal(str(value))
    result = float(decimal)
    if not decimal.is_finite() or not math.isfinite(result):
        raise ValueError('nonfinite supplied metric')
    return result


def normalize(folder, capture):
    catalog = json.loads((folder / 'candidates.json').read_text())
    audit = json.loads((folder / 'audit.json').read_text())
    parameters = json.loads((folder / 'parameters.json').read_text())
    manifest = json.loads((capture / 'manifest.json').read_text())
    provenance = json.loads(Path(manifest['provenance']['path']).read_text())
    events = {v['token']: str(v['event_id']) for v in provenance['markets']}
    rows, identities, aliases = [], set(), set()
    counts = Counter()
    excluded = 0
    metrics = ['net', 'stress_net', 'conservative_net', 'stress_conservative_net']
    for rule in catalog:
        if rule['offset_ticks'] != 0:
            excluded += 1
            continue
        policy = rule['base_id'].removeprefix('inside-entry:')
        for partition in ['train', 'development']:
            for index, episode in enumerate(rule['partitions'][partition]['episodes']):
                anchor = [policy, partition, episode['token'], str(Decimal(str(episode['quote_ts'])))]
                identity = canonical(anchor)
                if identity in identities:
                    raise ValueError('duplicate supplied case anchor')
                identities.add(identity)
                signature = canonical([episode[k] for k in [
                    'token', 'quote_ts', 'modeled_last_fill_ts', 'shares', 'entry_cash',
                    'transactions', 'exit_ts']] + [episode.get('failure_ts')] + [
                    episode[k] for k in ['net', 'stress_net', 'open_scenario_inventory']])
                aliases.add(signature)
                event = events[episode['token']]
                counts[(partition, event)] += 1
                rows.append(dict(id=identity, group=partition,
                                 metrics={k: number(episode[k]) for k in metrics},
                                 context=dict(policy=policy, event_id=event,
                                              economic_signature=signature,
                                              source_episode=dict(rule_id=rule['id'], index=index),
                                              raw_episode=episode)))
    unknown = [r for r in rows if r['metrics']['net'] is None]
    limitations = [
        'Supplied evaluator assertions; no independent fill, cash conservation or chain-truth validation.',
        'Overlapping hypothetical references are not a funded portfolio or independent trials.',
        'Unpriced net stays null; conservative zero salvage retains actual cost and held shares.',
        'Offset1 sensitivity labels remain in raw catalog, outside the matched offset0 comparison.',
        'Common quote anchors may have different cash and quantities; deltas are not whole-policy causal effects.',
        'Opposite-action evidence is limited to supplied raw transactions; this adapter does not synthesize inverse trades.',
        'Spread, depth, fee and causal receipt assumptions remain those of the unchanged supplied evaluator.',
    ]
    diagnostics = dict(
        adapter='consumer-anchor-normalization/v1', input_use='posthoc_development',
        raw_catalog_labels=len(catalog), matched_offset0_labels=len(catalog) - excluded,
        excluded_offset1_labels=excluded, case_count=len(rows),
        unique_economic_signatures=len(aliases), priced=len(rows) - len(unknown),
        unpriced=len(unknown), unknown_cases=unknown,
        overlapping_held_shares=str(sum((Decimal(str(r['context']['raw_episode']['open_scenario_inventory']))
                                         for r in rows), Decimal(0))),
        event_reference_counts={canonical(k): v for k, v in sorted(counts.items())},
        raw_parameters=parameters, raw_audit=audit,
        raw_partition_summaries=[dict(rule_id=r['id'], offset_ticks=r['offset_ticks'],
                                     partitions={part: {k: v for k, v in values.items() if k != 'episodes'}
                                                 for part, values in r['partitions'].items()})
                                 for r in catalog],
        financial_validation='supplied_evaluator_only', promotion='none')
    return dict(schema_version=1, accounting='winnow-passive-exit-supplied/v1',
                units={k: 'USD modeled scenario' for k in metrics}, cases=rows,
                diagnostics=diagnostics, limitations=limitations)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--catalog', type=Path, required=True)
    parser.add_argument('--capture', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    result = normalize(args.catalog, args.capture)
    with args.out.open('x') as handle:
        json.dump(result, handle, indent=2, allow_nan=False)
        handle.write('\n')
    print(json.dumps({k: result['diagnostics'][k] for k in [
        'case_count', 'unique_economic_signatures', 'priced', 'unpriced', 'overlapping_held_shares']}))
