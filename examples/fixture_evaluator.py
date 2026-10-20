"""Deterministic file-only external evaluator for Paperhands lifecycle acceptance."""
import argparse
import json
import os
from pathlib import Path
import sys
import subprocess
import threading
import time

p = argparse.ArgumentParser()
p.add_argument('--out', type=Path, required=True)
p.add_argument('--mode', default='ok')
p.add_argument('--value', type=float, default=0)
p.add_argument('--delay', type=float, default=0.01)
p.add_argument('--literal', default='')
args = p.parse_args()
args.out.mkdir(exist_ok=True)
(args.out / 'partial.txt').write_text('retained before completion\n')
print('started', flush=True)
print('diagnostic stderr', file=sys.stderr, flush=True)
if args.mode == 'crash':
    sys.exit(7)
if args.mode == 'timeout':
    time.sleep(10)
if args.mode == 'descendant':
    child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(20)'])
    (args.out / 'child.json').write_text(json.dumps({'pid': child.pid}))
    time.sleep(10)
if args.mode == 'overflow':
    print('x' * 100000, flush=True)
    time.sleep(1)
if args.mode == 'pressure':
    writers = [threading.Thread(target=lambda stream=stream: print('x' * 100000, file=stream, flush=True))
               for stream in [sys.stdout, sys.stderr]]
    for writer in writers:
        writer.start()
    for writer in writers:
        writer.join()
    time.sleep(1)
if args.mode == 'symlink':
    os.symlink('/etc/hosts', args.out / 'unsafe')
time.sleep(args.delay)
if args.mode == 'missing':
    sys.exit(0)
net = float('nan') if args.mode == 'nonfinite' else args.value
metrics = {'net': net, 'open_inventory': 0}
if args.mode == 'missing_metric':
    del metrics['net']
cases = [
    {'id': 'known', 'group': 'development', 'metrics': metrics,
     'context': {'quantity': 2, 'literal': args.literal}},
    {'id': 'unknown', 'group': 'development',
     'metrics': {'net': None, 'open_inventory': 3},
     'context': {'reason': 'missing exit', 'retained_cash_cost': '1.25'}}
]
if args.value > 0:
    cases.append({'id': 'new', 'group': 'development',
                  'metrics': {'net': -0.2, 'open_inventory': 0}, 'context': {}})
if args.mode == 'duplicate':
    cases.append(cases[0])
result = {'schema_version': 1, 'accounting': 'synthetic-fixture/v1',
          'units': {'net': 'different' if args.mode == 'units' else 'test-currency',
                    'open_inventory': 'test-shares'},
          'cases': cases, 'diagnostics': {'synthetic': True},
          'limitations': ['Synthetic lifecycle evidence; no market observations or fill model.']}
(args.out / 'cases.json').write_text(json.dumps(result))
