import os
import pathlib
import subprocess

root = pathlib.Path.cwd()
env_root = pathlib.Path(os.environ['JOB_ENV_DIR'])
env = os.environ | {'PATH': str(env_root / 'node-v24.22.2-linux-x64/bin') + ':' + os.environ['PATH']}
subprocess.run([
    env_root / 'pnpm12/node_modules/.bin/pnpm', 'install', '--frozen-lockfile',
    '--ignore-scripts', '--store-dir', env_root / 'pnpm-store',
], cwd=root / 'docs', env=env, check=True)
