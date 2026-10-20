"""Prepare fresh plans from a consumer reference; never runs collectors or evaluators.

All writes stay under --out. Source, capture, venv and provenance are read-only.
Reference JSON fields: capture_copy, runtime, evaluator_hashes and source_commit.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def sha(path):
    return hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--reference', type=Path, required=True)
parser.add_argument('--python', required=True, help='Absolute existing consumer interpreter')
parser.add_argument('--paperhands', required=True)
parser.add_argument('--out', type=Path, required=True)
args = parser.parse_args()
reference = json.loads(args.reference.read_text())
capture, runtime = Path(reference['capture_copy']), Path(reference['runtime'])
manifest = json.loads((capture / 'manifest.json').read_text())
report = json.loads((capture / 'report.json').read_text())
if manifest['run_id'] != report['run_id'] or report['run_evidence']['stop_reason'] != 'window_complete':
    raise ValueError('consumer capture is not closed by matching completion report')
if subprocess.check_output(['git', '-C', str(runtime), 'rev-parse', 'HEAD'], text=True).strip() != reference['source_commit']:
    raise ValueError('consumer source commit differs')
for source, expected in reference['evaluator_hashes'].items():
    if sha(Path(source)) != expected:
        raise ValueError('consumer evaluator source differs: ' + source)
root = args.out.absolute()
root.mkdir(parents=True)
for name in ['evaluator', 'accounting', 'config', 'plans']:
    (root / name).mkdir()
for package in ['research', 'paper', 'recorder']:
    target = root / 'evaluator' / package
    target.mkdir()
    for source in sorted((runtime / package).glob('*.py')):
        if source.is_symlink():
            raise ValueError('consumer source link refused')
        shutil.copyfile(source, target / source.name)
shutil.copyfile(Path(__file__).with_name('winnow_cases.py'), root / 'accounting' / 'winnow_cases.py')
config = Path(manifest['policy']['_config_path'])
if sha(config) != manifest['config_sha256']:
    raise ValueError('original manifest config changed')
shutil.copyfile(config, root / 'config' / 'config.json')
(root / 'config' / 'source.json').write_text(json.dumps(dict(source_commit=reference['source_commit'],
                                                         data_use='posthoc_development')))
bundles = []
for name, role, source in [('input', 'input', capture), ('evaluator', 'evaluator', root / 'evaluator'),
                           ('accounting', 'accounting', root / 'accounting'), ('config', 'config', root / 'config')]:
    inventory = root / (name + '.inventory.json')
    cmd = [args.paperhands, 'inventory', 'seal', '--source', str(source), '--out', str(inventory)]
    if role == 'input':
        cmd += ['--kind', 'capture', '--closed-by', 'report.json']
    result = json.loads(subprocess.check_output(cmd))['data']
    bundles.append(dict(name=name, role=role, source=str(source), inventory=str(inventory), sha256=result['sha256']))
provenance = Path(manifest['provenance']['path'])
if sha(provenance) != manifest['provenance']['sha256']:
    raise ValueError('external original provenance changed')
pins = [dict(path=args.python, role='executable', sha256=sha(Path(args.python).resolve())),
        dict(path=str(provenance), role='provenance', sha256=sha(provenance))]
for arm in ['baseline', 'candidate']:
    argv = [args.python, '-m', 'research.passive_exit_miner', '--run-dir', '{snapshot:input}',
            '--out', '{output}/raw', '--exit-style', 'taker', '--rpc-truth-guard', '--max-seconds', '60']
    if arm == 'candidate':
        argv += ['--improve-entry']
    plan = dict(schema_version=1, id='winnow-' + arm,
                question='Closed retained input: existing best-bid versus inside-one-tick taker entry',
                data_use='development', bundles=bundles, external_files=pins,
                env={'PYTHONPATH': '{snapshot:evaluator}', 'PYTHONDONTWRITEBYTECODE': '1'},
                commands=[dict(argv=argv, cwd='{snapshot:evaluator}'),
                          dict(argv=[args.python, '{snapshot:accounting}/winnow_cases.py',
                                     '--catalog', '{output}/raw', '--capture', '{snapshot:input}',
                                     '--out', '{output}/cases.json'], cwd='{work}')],
                limits=dict(wall_time_ms=90000, cpu_time_seconds=60, stdout_bytes=1048576,
                            stderr_bytes=1048576, artifact_bytes=67108864, artifact_files=32),
                required_artifacts=['raw/candidates.json', 'raw/parameters.json', 'raw/audit.json',
                                    'raw/report.md', 'cases.json'], cases='cases.json')
    (root / 'plans' / (arm + '.json')).write_text(json.dumps(plan, indent=2))
print(json.dumps(dict(plans=str(root / 'plans'), writes=str(root), executed=False,
                      data_use='posthoc_development', external_provenance_pinned=True)))
