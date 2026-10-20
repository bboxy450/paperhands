"""Create fresh file-only plans; never executes an experiment or accesses a venue."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--out', type=Path, required=True)
p.add_argument('--paperhands', required=True)
p.add_argument('--members', type=int, default=2)
p.add_argument('--include-failures', action='store_true')
args = p.parse_args()
if not 1 <= args.members <= 4096:
    p.error('members must be 1..4096')
root = args.out.absolute()
root.mkdir(parents=True)
for name in ('input', 'evaluator', 'config', 'plans'):
    (root / name).mkdir()
(root / 'input' / 'report.json').write_text('{"closed":true,"synthetic":true}')
(root / 'input' / 'data.json').write_text('[1,2,3]')
(root / 'config' / 'config.json').write_text('{"fixture":1}')
shutil.copyfile(Path(__file__).with_name('fixture_evaluator.py'), root / 'evaluator' / 'fixture_evaluator.py')
bundles = []
for name, role in [('input', 'input'), ('evaluator', 'evaluator'), ('config', 'config')]:
    inventory = root / (name + '.inventory.json')
    cmd = [args.paperhands, 'inventory', 'seal', '--source', str(root / name), '--out', str(inventory)]
    if role == 'input':
        cmd += ['--closed-by', 'report.json']
    r = subprocess.run(cmd, capture_output=True, text=True, check=True)
    sha = json.loads(r.stdout)['data']['sha256']
    bundles.append(dict(name=name, role=role, source=str(root / name), inventory=str(inventory), sha256=sha))
python = sys.executable
pin = dict(path=python, sha256=hashlib.sha256(Path(python).resolve().read_bytes()).hexdigest(), role='executable')
ids = []
for i in range(args.members):
    identity = f'policy-{i:03}'
    ids.append(identity)
    mode = {31: 'crash', 47: 'timeout', 193: 'missing'}.get(i, 'ok') if args.include_failures else 'ok'
    plan = dict(schema_version=1, id=identity, question='Synthetic independent policy fixture', data_use='development',
                bundles=bundles, external_files=[pin], env={},
                commands=[dict(argv=[python, '{snapshot:evaluator}/fixture_evaluator.py', '--out', '{output}', '--mode', mode,
                                     '--value', str(i % 2)], cwd='{work}')],
                limits=dict(wall_time_ms=500, cpu_time_seconds=2, stdout_bytes=4096, stderr_bytes=4096,
                            artifact_bytes=2*1024*1024, artifact_files=16),
                required_artifacts=['cases.json'], cases='cases.json')
    path = root / 'plans' / (identity + '.json')
    path.write_text(json.dumps(plan, indent=2))
batch = dict(schema_version=1, id='fixture-batch', attempts=ids, max_parallel=4, wall_time_ms=120000,
             total_output_bytes=16*1024*1024, total_artifact_bytes=128*1024*1024)
(root / 'batch.json').write_text(json.dumps(batch, indent=2))
print(json.dumps(dict(plans=str(root/'plans'), batch=str(root/'batch.json'), members=len(ids), executed=False)))
