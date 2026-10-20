"""Check a fresh replay against consumer-provided reference evidence, offline.

Checks reproduction and supplied accounting values; not an independent financial audit.
"""
import argparse
from collections import Counter
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import subprocess


def read(path):
    return json.loads(path.read_text())


def sha(path):
    return hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()


p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, required=True, help='Fresh winnow_closed staging directory')
p.add_argument('--reference', type=Path, required=True)
p.add_argument('--paperhands', required=True)
p.add_argument('--out', type=Path, required=True)
p.add_argument('--direct-verification', type=Path)
a = p.parse_args()
reference = read(a.reference)
comparison = read(a.root / 'comparison.json')
store = a.root / 'store'
receipt = dict(scope='offline reproduction of supplied evaluator evidence', arms={})
for arm, catalog_name in [('baseline', 'join-taker32'), ('candidate', 'inside-taker16')]:
    folder = store / 'experiments' / ('winnow-' + arm)
    status = json.loads(subprocess.check_output([a.paperhands, 'experiment', 'inspect', '--dir', str(folder)]))['data']
    assert status['process_status'] == 'command_succeeded'
    assert status['evidence_status'] == 'unreviewed'
    assert len(status['result']['steps']) == 2
    assert all(s['pid'] > 0 and s['exit_code'] == 0 for s in status['result']['steps'])
    catalog = read(folder / 'output/raw/candidates.json')
    cases = read(folder / 'output/cases.json')
    expected = reference['expected_catalogs'][catalog_name]
    assert len(catalog) == expected['labels']
    for part, totals in expected['partitions'].items():
        for metric in ['n', 'priced', 'unpriced', 'net', 'stress_net', 'conservative_net',
                       'stress_conservative_net', 'open_scenario_inventory', 'partial', 'maker_sold']:
            actual = sum((Decimal(str(r['partitions'][part][metric])) for r in catalog), Decimal(0))
            assert actual == Decimal(totals[metric]), (arm, part, metric, actual, totals[metric])
    unknown = [r for r in cases['cases'] if r['metrics']['net'] is None]
    assert all(r['context']['raw_episode']['net'] is None for r in unknown)
    assert len(unknown) == (6 if arm == 'candidate' else 0)
    signature_count = len({r['context']['economic_signature'] for r in cases['cases']})
    assert signature_count == (18 if arm == 'candidate' else 3)
    digest = sha(folder / 'output/raw/candidates.json')
    if a.direct_verification:
        direct = next(v for v in read(a.direct_verification)['closed_input_offline_reproduction'] if v['arm'] == arm)
        assert digest == direct['sha256'], (arm, 'raw candidates are not byte-identical')
    receipt['arms'][arm] = dict(raw_candidate_sha256=digest, cases=len(cases['cases']),
                               economic_signatures=signature_count, unknown_cases=len(unknown),
                               overlapping_held_shares=cases['diagnostics']['overlapping_held_shares'],
                               process_steps=status['result']['steps'])
for part, expected in reference['expected_common_unmatched'].items():
    common = [r for r in comparison['common'] if r['group'] == part]
    assert len(common) == expected['common_quote_anchors']
    for field in ['baseline_only', 'candidate_only']:
        assert sum(r['group'] == part for r in comparison[field]) == expected[field]
    def raw(row, arm):
        return row[arm]['context']['raw_episode']
    for metric, expected_field in [('shares', 'changed_quantity'), ('entry_cash', 'changed_cash')]:
        assert sum(raw(r, 'candidate')[metric] != raw(r, 'baseline')[metric] for r in common) == expected[expected_field]
    for metric, field in [('conservative_net', 'common_conservative_delta'),
                          ('stress_conservative_net', 'common_stress_delta')]:
        delta = sum((Decimal(raw(r, 'candidate')[metric]) - Decimal(raw(r, 'baseline')[metric]) for r in common), Decimal(0))
        assert delta == Decimal(expected[field])
inventory = read(a.root / 'input.inventory.json')
receipt.update(input_files=len(inventory['files']), input_bytes=sum(m['bytes'] for m in inventory['files']),
               common=Counter(r['group'] for r in comparison['common']),
               candidate_only=Counter(r['group'] for r in comparison['candidate_only']),
               baseline_only=len(comparison['baseline_only']), promotion=comparison['promotion'],
               financial_validation='supplied evaluator assertions; unsupported independent checks remain explicit')
with a.out.open('x') as handle:
    json.dump(receipt, handle, indent=2)
    handle.write('\n')
print(json.dumps({k: v for k, v in receipt.items() if k != 'arms'}))
