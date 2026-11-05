import argparse
import csv
import hashlib
import json
import os
import pathlib
import platform
import re
import statistics
import subprocess
import tempfile
import time

import psutil

ENV = pathlib.Path(os.environ['JOB_ENV_DIR'])
OUT = pathlib.Path(os.environ['ARTIFACTS_DIR'])
CONFIG = json.loads((ENV / 'bench-config.json').read_text())
BASE_ENV = {key: value for key, value in os.environ.items()
            if not key.startswith(('OXLINT_TSGOLINT', 'TSRS_'))
            and key not in ('NODE_OPTIONS', 'NODE_PATH', 'GITHUB_ACTIONS')}
EXPECTED = {}
RECORDS = []
SCENARIOS = ('rules', 'full', 'check')
TIMING_PAIRS = 10
MEMORY_PAIRS = 5


def progress(phase, message, completed=0, total=0):
    value = {'phase': phase, 'message': message, 'completed': completed, 'total': total}
    subprocess.run(['python3', os.environ['JOB_WORKER'], 'progress', os.environ['JOB_DIR'], json.dumps(value)], check=True)
    print(json.dumps(value), flush=True)


def environment(project, scenario, backend):
    env = BASE_ENV | {
        'PATH': str(pathlib.Path(project['node']).parent) + ':' + BASE_ENV['PATH'],
        'VP_HOME': str(ENV / 'vp-home'), 'VP_SELF_SETUP_NO_MODIFY_PATH': '1',
    }
    if scenario != 'check' and project['scheduling']:
        env['OXLINT_TSGOLINT_CHECKER_SCHEDULING'] = project['scheduling']
    if backend == 'go':
        env['OXLINT_TSGOLINT_PATH'] = CONFIG['go']
    return env


def lint_config(project):
    code = '''
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const require = createRequire(process.argv[1]);
const { loadConfigFromFile } = await import(pathToFileURL(require.resolve('vite')).href);
const result = await loadConfigFromFile({ command: 'serve', mode: 'development' }, undefined, process.cwd(), 'silent');
if (!result?.config?.lint) throw new Error('Missing lint configuration');
process.stdout.write(JSON.stringify(result.config.lint));
'''
    cli_package = str(pathlib.Path(CONFIG['cli']).parent.parent / 'package.json')
    env = environment(project, 'rules', 'rust') | {'VP_RESOLVING_CONFIG_METADATA': '1'}
    output = subprocess.check_output([project['node'], '--input-type=module', '-e', code, cli_package], cwd=project['root'], env=env, text=True)
    original = json.loads(output)
    (OUT / (project['name'] + '-lint-config.json')).write_text(json.dumps(original, indent=2) + '\n')
    config = json.loads(output)
    config.setdefault('options', {})['typeAware'] = True
    config['options']['typeCheck'] = False
    return config


def signature(project, scenario, stdout, stderr):
    if scenario == 'check':
        combined = stdout + stderr
        formatted = re.search(r'pass: All ([\d,]+) files are correctly formatted', combined)
        linted = re.search(r'pass: Found no warnings, lint errors, or type errors in ([\d,]+) files', combined)
        assert formatted and linted, combined
        files = int(linted[1].replace(',', ''))
        result = {'files': files, 'format_files': int(formatted[1].replace(',', ''))}
    else:
        data = json.loads(stdout)
        assert not data['diagnostics'], data['diagnostics'][:5]
        files = data['number_of_files']
        assert data['number_of_rules'] == project['rules'], data
        result = {key: data[key] for key in ('number_of_files', 'number_of_rules', 'threads_count', 'diagnostics')}
    assert files == project['files'], (project['name'], files, project['files'])
    key = (project['name'], scenario)
    if key in EXPECTED:
        assert result == EXPECTED[key], (key, result, EXPECTED[key])
    else:
        EXPECTED[key] = result
    return result


def sample(child, start):
    root = psutil.Process(child.pid)
    seen = {child.pid: root.create_time()}
    trace = []
    processes = {}
    while child.poll() is None:
        try:
            for process in root.children(recursive=True):
                seen[process.pid] = process.create_time()
        except psutil.Error:
            pass
        alive = []
        for pid, born in list(seen.items()):
            try:
                process = psutil.Process(pid)
                with process.oneshot():
                    if process.create_time() != born or process.status() == psutil.STATUS_ZOMBIE:
                        continue
                    rss = process.memory_info().rss
                    exe = process.exe()
                elapsed = time.perf_counter() - start
                key = f'{pid}:{exe}'
                record = processes.setdefault(key, {'pid': pid, 'exe': exe, 'first': elapsed, 'peak_rss': 0})
                record['last'] = elapsed
                record['peak_rss'] = max(record['peak_rss'], rss)
                alive.append({'pid': pid, 'exe': exe, 'rss': rss})
            except psutil.Error:
                pass
        trace.append({'seconds': time.perf_counter() - start, 'rss': sum(p['rss'] for p in alive), 'processes': alive})
        time.sleep(0.02)
        if time.perf_counter() - start > 300:
            child.kill()
            raise TimeoutError('Benchmark command exceeded 300 seconds')
    return trace, list(processes.values())


def run(project, scenario, backend, pair, phase, config=None):
    args = ['check'] if scenario == 'check' else ['lint', '--type-aware', '--threads', '4', '--format', 'json']
    if scenario == 'full':
        args += ['--type-check']
    if config:
        args += ['-c', str(config)]
    command = [project['node'], CONFIG['cli'], *args]
    directory = OUT / phase / project['name']
    directory.mkdir(parents=True, exist_ok=True)
    stem = directory / f'{scenario}-{backend}-{pair:02d}'
    with stem.with_suffix('.stdout').open('w') as stdout, stem.with_suffix('.stderr').open('w') as stderr:
        start = time.perf_counter()
        child = subprocess.Popen(command, cwd=project['root'], env=environment(project, scenario, backend), stdout=stdout, stderr=stderr)
        trace, processes = sample(child, start) if phase in ('validate', 'memory') else ([], [])
        # Blocking wait avoids the polling latency of wait(timeout=...).
        # The Codespaces worker enforces the overall task deadline.
        status = child.wait()
        seconds = time.perf_counter() - start
    stdout = stem.with_suffix('.stdout').read_text()
    stderr = stem.with_suffix('.stderr').read_text()
    assert status == 0, (project['name'], scenario, backend, status, stdout[-5000:], stderr[-5000:])
    result = signature(project, scenario, stdout, stderr)
    record = {'project': project['name'], 'scenario': scenario, 'backend': backend,
              'phase': phase, 'pair': pair, 'seconds': seconds, 'exit': status,
              'peak_rss': max((x['rss'] for x in trace), default=None), 'signature': result}
    if trace:
        native = [p for p in processes if pathlib.Path(p['exe']).name in ('tsgolint', 'tsrs')]
        assert len(native) == 1, native
        assert pathlib.Path(native[0]['exe']).name == ('tsgolint' if backend == 'go' else 'tsrs')
        intervals = [b['seconds'] - a['seconds'] for a, b in zip(trace, trace[1:])]
        record.update({'native': native[0], 'median_sample_interval_ms': statistics.median(intervals) * 1000})
        stem.with_suffix('.trace.json').write_text(json.dumps(trace))
        stem.with_suffix('.processes.json').write_text(json.dumps(processes, indent=2))
    stem.with_suffix('.json').write_text(json.dumps(record, indent=2) + '\n')
    RECORDS.append(record)
    (OUT / 'samples.json').write_text(json.dumps(RECORDS, indent=2) + '\n')
    print(project['name'], scenario, backend, phase, pair, f'{seconds:.3f}s', record['peak_rss'], flush=True)
    return record


def measure(project, scenario, phase):
    handle = None
    if scenario == 'rules':
        handle = tempfile.NamedTemporaryFile(mode='w', suffix='.json', prefix='.tsrs-bench-', dir=project['root'])
        json.dump(lint_config(project), handle)
        handle.flush()
    try:
        pairs = 0 if phase == 'validate' else TIMING_PAIRS if phase == 'time' else MEMORY_PAIRS
        for pair in range(-1, pairs):
            backends = ('go', 'rust') if pair < 0 or pair % 2 == 0 else ('rust', 'go')
            for backend in backends:
                run(project, scenario, backend, pair, phase, handle.name if handle else None)
    finally:
        if handle:
            handle.close()


def metadata():
    info = {'vp_commit': CONFIG['vp_commit'], 'python': platform.python_version(), 'psutil': psutil.__version__,
            'platform': platform.platform(), 'cpu_affinity': psutil.Process().cpu_affinity(),
            'memory_bytes': psutil.virtual_memory().total, 'timing_pairs': TIMING_PAIRS,
            'memory_pairs': MEMORY_PAIRS, 'sample_sleep_ms': 20, 'projects': CONFIG['projects'],
            'versions': json.loads((ENV / 'setup-receipt.json').read_text())['versions'],
            'selection': vars(options)}
    for name, args in [('cpu', ['lscpu']), ('git_diff', ['git', 'diff', '--stat']), ('source', ['git', 'rev-parse', 'HEAD'])]:
        info[name] = subprocess.check_output(args, text=True)
    info['node_versions'] = {p['name']: subprocess.check_output([p['node'], '--version'], text=True).strip() for p in CONFIG['projects']}
    info['cpu_percent_before'] = psutil.cpu_percent(interval=1)
    (OUT / 'metadata.json').write_text(json.dumps(info, indent=2) + '\n')


def summarize():
    rows = []
    for project in selected_projects:
        for scenario in selected_scenarios:
            row = {'project': project['name'], 'scenario': scenario}
            for backend in ('go', 'rust'):
                for phase, metric in [('time', 'seconds'), ('memory', 'peak_rss')]:
                    if phase not in phases:
                        continue
                    values = [r[metric] for r in RECORDS if r['project'] == project['name'] and r['scenario'] == scenario and r['backend'] == backend and r['phase'] == phase and r['pair'] >= 0]
                    row[f'{backend}_{metric}'] = statistics.median(values)
                    row[f'{backend}_{metric}_range'] = [min(values), max(values)]
            if 'time' in phases:
                row['time_saved_percent'] = 100 * (1 - row['rust_seconds'] / row['go_seconds'])
            if 'memory' in phases:
                row['rss_saved_percent'] = 100 * (1 - row['rust_peak_rss'] / row['go_peak_rss'])
            rows.append(row)
    (OUT / 'summary.json').write_text(json.dumps(rows, indent=2) + '\n')
    with (OUT / 'samples.csv').open('w') as file:
        keys = ['project', 'scenario', 'backend', 'phase', 'pair', 'seconds', 'peak_rss', 'exit']
        writer = csv.DictWriter(file, fieldnames=keys, extrasaction='ignore')
        writer.writeheader()
        writer.writerows(RECORDS)
    print('RESULTS', json.dumps(rows), flush=True)


parser = argparse.ArgumentParser()
parser.add_argument('--validate-only', action='store_true')
parser.add_argument('--project', choices=[p['name'] for p in CONFIG['projects']])
parser.add_argument('--scenario', choices=SCENARIOS)
parser.add_argument('--phase', choices=('time', 'memory'))
options = parser.parse_args()
selected_projects = [p for p in CONFIG['projects'] if not options.project or p['name'] == options.project]
selected_scenarios = (options.scenario,) if options.scenario else SCENARIOS
metadata()
phases = ('validate',) if options.validate_only else (options.phase,) if options.phase else ('time', 'memory')
total = len(phases) * len(selected_projects) * len(selected_scenarios)
completed = 0
for phase in phases:
    for project in selected_projects:
        for scenario in selected_scenarios:
            progress(phase, f"{project['name']}: {scenario}", completed, total)
            measure(project, scenario, phase)
            completed += 1
            progress(phase, f"Completed {project['name']}: {scenario}", completed, total)
if not options.validate_only:
    summarize()
expected_runs = len(selected_projects) * len(selected_scenarios) * sum(
    2 * (1 + (0 if phase == 'validate' else TIMING_PAIRS if phase == 'time' else MEMORY_PAIRS))
    for phase in phases)
assert len(RECORDS) == expected_runs, (len(RECORDS), expected_runs)
progress('validated' if options.validate_only else 'measured', 'All output assertions passed', total, total)
