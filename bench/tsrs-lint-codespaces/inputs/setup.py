import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import urllib.request

ROOT = pathlib.Path.cwd()
ENV = pathlib.Path(os.environ['JOB_ENV_DIR'])
ENV.mkdir(parents=True, exist_ok=True)
INPUT = pathlib.Path(os.environ['JOB_INPUT_DIR'])
OUT = pathlib.Path(os.environ['ARTIFACTS_DIR'])
VP_COMMIT = '22b07bd150aa57b3e04cc52e0b81d43294ffe982'
VINEXT_COMMIT = '249088af307c0a852fa6dc59c08fc6ec80e1620e'


def run(args, cwd=ROOT, env=None):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), cwd=cwd, env=env, check=True)


def progress(message):
    run(['python3', os.environ['JOB_WORKER'], 'progress', os.environ['JOB_DIR'],
         json.dumps({'phase': 'setup', 'message': message})])


def fetch_git(url, revision, destination):
    if not (destination / '.git').exists():
        destination.mkdir(parents=True, exist_ok=True)
        run(['git', 'init', destination])
        run(['git', 'remote', 'add', 'origin', url], cwd=destination)
        run(['git', 'fetch', '--depth', '1', 'origin', revision], cwd=destination)
        run(['git', 'checkout', '--detach', 'FETCH_HEAD'], cwd=destination)
    actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=destination, text=True).strip()
    assert actual == revision, (destination, actual, revision)


def node(version):
    target = ENV / f'node-v{version}-linux-x64'
    if not (target / 'bin/node').exists():
        filename = target.name + '.tar.xz'
        base = f'https://nodejs.org/dist/v{version}/'
        with urllib.request.urlopen(base + 'SHASUMS256.txt') as response:
            checksums = response.read().decode()
        (OUT / f'node-{version}-SHASUMS256.txt').write_text(checksums)
        expected = next(line.split()[0] for line in checksums.splitlines() if line.split()[-1] == filename)
        archive = ENV / filename
        urllib.request.urlretrieve(base + filename, archive)
        assert hashlib.sha256(archive.read_bytes()).hexdigest() == expected
        run(['tar', '-xJf', archive, '-C', ENV])
    assert subprocess.check_output([target / 'bin/node', '--version'], text=True).strip() == 'v' + version
    return target


def npm(prefix, packages, runtime):
    prefix.mkdir(parents=True, exist_ok=True)
    env = os.environ | {'PATH': str(runtime / 'bin') + ':' + os.environ['PATH']}
    run([runtime / 'bin/npm', 'install', '--prefix', prefix, '--no-audit', '--no-fund',
         '--ignore-scripts', '--package-lock=false', *packages], env=env)


def copy_dir(source, destination):
    if destination.is_symlink():
        destination.unlink()
    shutil.copytree(source, destination, dirs_exist_ok=True)


progress('Installing pinned Node and package-manager versions')
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip() == VP_COMMIT
patch = INPUT / 'rfcs.patch'
if patch.stat().st_size:
    reverse = subprocess.run(['git', 'apply', '--reverse', '--check', str(patch)], capture_output=True)
    if reverse.returncode:
        run(['git', 'apply', patch])
node24 = node('24.22.2')
node26 = node('26.13.0')
npm(ENV / 'pnpm12', ['pnpm@12.11.2'], node24)
# pnpm 12 ships a shebang-less placeholder until its native installer runs.
run([node24 / 'bin/node', ENV / 'pnpm12/node_modules/pnpm/install.js'])
npm(ENV / 'pnpm11', ['pnpm@11.1.1'], node26)
for prefix, version in [('pnpm12', '12.11.2'), ('pnpm11', '11.1.1')]:
    pm_env = os.environ | {'PATH': str(node26 / 'bin') + ':' + os.environ['PATH']}
    assert subprocess.check_output([ENV / prefix / 'node_modules/.bin/pnpm', '--version'], cwd=ENV, env=pm_env, text=True).strip() == version
if not (ENV / 'venv/bin/python').exists():
    # The repository's minimal Ubuntu image can omit ensurepip.
    check = subprocess.run(['python3', '-c', 'import ensurepip'], capture_output=True, text=True)
    if check.returncode:
        if 'No module named' not in check.stderr or 'ensurepip' not in check.stderr:
            raise RuntimeError(check.stderr)
        progress('Installing python3-venv because the container lacks ensurepip')
        run(['sudo', 'apt-get', 'update'])
        run(['sudo', 'env', 'DEBIAN_FRONTEND=noninteractive', 'apt-get', 'install', '-y', 'python3-venv'])
    run(['python3', '-m', 'venv', ENV / 'venv'])
run([ENV / 'venv/bin/python', '-m', 'pip', 'install', 'psutil==7.0.0'])
base_env = os.environ | {
    'PATH': str(node24 / 'bin') + ':' + str(ENV / 'pnpm12/node_modules/.bin') + ':' + os.environ['PATH'],
    'VP_HOME': str(ENV / 'vp-home'), 'VP_SELF_SETUP_NO_MODIFY_PATH': '1',
    'CARGO_HOME': str(ENV / 'cargo'), 'CARGO_TARGET_DIR': str(ENV / 'cargo-target'),
}
progress('Fetching exact source dependencies and published native artifacts')
for name, spec in json.loads((ROOT / 'packages/tools/.upstream-versions.json').read_text()).items():
    fetch_git(spec['repo'], spec['hash'], ROOT / name)
run([node24 / 'bin/node', 'packages/tools/src/vendored-vitest.ts'], env=base_env)
npm(ENV / 'published', [
    'vite-plus@1.5.0', '@voidzero-dev/vite-plus-core@1.5.0',
    '@voidzero-dev/vite-plus-linux-x64-gnu@1.5.0',
    '@voidzero-dev/vite-plus-prompts@1.5.0', 'rolldown@1.2.16',
], node24)
run([ENV / 'pnpm12/node_modules/.bin/pnpm', 'install', '--frozen-lockfile', '--ignore-scripts',
     '--store-dir', ENV / 'pnpm-store'], env=base_env)
modules = ENV / 'published/node_modules'
for source, destination in [
    (modules / 'vite-plus/dist', ROOT / 'packages/cli/dist'),
    (modules / '@voidzero-dev/vite-plus-core/dist', ROOT / 'packages/core/dist'),
    (modules / '@voidzero-dev/vite-plus-prompts/dist', ROOT / 'packages/prompts/dist'),
    (modules / 'rolldown/dist', ROOT / 'rolldown/packages/rolldown/dist'),
    (modules / '@rolldown/binding-linux-x64-gnu', ROOT / 'rolldown/packages/rolldown/node_modules/@rolldown/binding-linux-x64-gnu'),
    (modules / '@voidzero-dev/vite-plus-linux-x64-gnu', ROOT / 'packages/core/node_modules/@voidzero-dev/vite-plus-linux-x64-gnu'),
]:
    copy_dir(source, destination)
native = modules / '@voidzero-dev/vite-plus-linux-x64-gnu/vite-plus.linux-x64-gnu.node'
shutil.copy2(native, ROOT / 'packages/cli/binding' / native.name)
progress('Building CLI JavaScript; using the published 1.5.0 Linux native binding and core')
run([ENV / 'pnpm12/node_modules/.bin/pnpm', '--filter', 'vite-plus', 'build-ts'], env=base_env)
progress('Preparing the pinned vinext benchmark corpus and generated types')
vinext = ENV / 'vinext'
fetch_git('https://github.com/fengmk2/vinext.git', VINEXT_COMMIT, vinext)
vinext_env = base_env | {'PATH': str(node26 / 'bin') + ':' + str(ENV / 'pnpm11/node_modules/.bin') + ':' + os.environ['PATH']}
run([ENV / 'pnpm11/node_modules/.bin/pnpm', 'install', '--frozen-lockfile', '--ignore-scripts',
     '--store-dir', ENV / 'pnpm-store-vinext'], cwd=vinext, env=vinext_env)
run([ENV / 'pnpm11/node_modules/.bin/pnpm', 'run', 'postinstall'], cwd=vinext, env=vinext_env)
config = {
    'vp_commit': VP_COMMIT, 'cli': str(ROOT / 'packages/cli/bin/vp'),
    'go': str(ROOT / 'packages/cli/node_modules/.bin/tsgolint'),
    'tsrs': str(ROOT / 'packages/cli/node_modules/.bin/tsrs-lint'),
    'projects': [
        {'name': 'vite-plus', 'root': str(ROOT), 'node': str(node24 / 'bin/node'), 'files': 385, 'rules': 174, 'scheduling': None},
        {'name': 'vinext', 'root': str(vinext), 'node': str(node26 / 'bin/node'), 'files': 1828, 'rules': 151, 'scheduling': 'sorted', 'commit': VINEXT_COMMIT},
    ],
}
(ENV / 'bench-config.json').write_text(json.dumps(config, indent=2) + '\n')
versions = {}
for package in ['tsrs', 'oxlint', 'oxlint-tsgolint']:
    versions[package] = json.loads((ROOT / 'packages/cli/node_modules' / package / 'package.json').read_text())['version']
assert versions == {'tsrs': '0.22.0-ts7.1.0-beta', 'oxlint': '1.91.0', 'oxlint-tsgolint': '7.0.2005'}, versions
files = [ROOT / 'packages/cli/dist/bin.js', native, node24 / 'bin/node', node26 / 'bin/node', ENV / 'bench-config.json']
receipt = {'versions': versions, 'hashes': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}}
(ENV / 'setup-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
print('SETUP_COMPLETE', json.dumps(versions), flush=True)
