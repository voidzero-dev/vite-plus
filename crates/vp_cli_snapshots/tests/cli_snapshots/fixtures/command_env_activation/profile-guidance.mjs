import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const mode = process.argv[2];
const home = path.resolve('profiles/user');
const binary = path.join(
  process.env.VP_HOME,
  'current/bin',
  process.platform === 'win32' ? 'vp.exe' : 'vp',
);
const env = {
  ...process.env,
  HOME: home,
  USERPROFILE: home,
  ZDOTDIR: path.join(home, 'zsh'),
  XDG_CONFIG_HOME: path.join(home, '.config'),
  XDG_DATA_HOME: path.join(home, '.local/share'),
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
};
// Emulate a fresh terminal, without tool delegation state from this Node process.
delete env.VP_PATH_INJECTED_TOOLS;
delete env.VP_BYPASS;
delete env.VP_SHELL;
fs.mkdirSync(home, { recursive: true });
fs.writeFileSync('.node-version', '22.18.0\n');

/** @returns {string} */
function captureVp(args, extra = {}) {
  const result = spawnSync(binary, args, {
    env: { ...env, ...extra },
    encoding: 'utf8',
    timeout: 30000,
  });
  assert.equal(result.status, 0, result.error?.message ?? result.stdout + result.stderr);
  return result.stdout.replace(/\u001b\[[0-9;]*m/g, '').replaceAll('\r\n', '\n');
}
const dirs = Object.fromEntries(
  captureVp([], { VP_DUMP_DIRS: '1' })
    .trim()
    .split('\n')
    .map((line) => line.split('\t')),
);

/** @returns {string} */
function setup(shell) {
  if (shell === undefined) delete env.VP_SHELL;
  else env.VP_SHELL = shell;
  console.log(`VP_SHELL=${shell ?? '<unset>'}:`);
  const output = captureVp(['env', 'setup']);
  const heading = output.indexOf('Next Steps:\n');
  assert.ok(heading >= 0, output);
  return output.slice(heading).trimEnd();
}

if (mode === 'powershell') {
  for (const shell of ['pwsh', undefined]) {
    const output = setup(shell);
    assert.match(output, /\. '[^\n]*env\.ps1'/);
    assert.match(output, /\$PROFILE if it is not already there/);
    assert.doesNotMatch(output, /Or open a new terminal/);
    console.log(output.split('\n').find((line) => line.includes('$PROFILE')));
  }
} else {
  captureVp(['env', 'on', 'node']);
  const bash = process.env.PATH.split(path.delimiter)
    .map((dir) => path.join(dir, 'bash'))
    .find((file) => fs.existsSync(file));
  assert.ok(bash);
  env.SHELL = '/bin/bash';
  const system = path.resolve('profiles/system');
  fs.mkdirSync(system);
  fs.writeFileSync(path.join(system, 'node'), '#!/bin/sh\necho system-node\n', { mode: 0o755 });
  env.PATH = system;
  const envPath = path.join(dirs.config, 'env').replace(/[\\$`"]/g, '\\$&');
  const source = `. "${envPath}"\n`;
  const fish = path.join(env.XDG_CONFIG_HOME, 'fish/config.fish');
  fs.mkdirSync(path.dirname(fish), { recursive: true });
  fs.writeFileSync(fish, `source "${path.join(dirs.config, 'env.fish')}"\n`);

  /** @returns {void} */
  function freshBash(expectedNode) {
    console.log("$ bash --noprofile -ic 'command -v node; node --version'");
    const result = spawnSync(
      bash,
      [
        '--noprofile',
        '-ic',
        'command -v node; test "$(command -v node)" = "$EXPECTED_NODE" || exit 1; node --version',
      ],
      { env: { ...env, EXPECTED_NODE: expectedNode }, stdio: 'inherit', timeout: 30000 },
    );
    assert.equal(result.status, 0, result.error?.message);
  }

  console.log('Only Fish is configured:');
  for (const shell of [undefined, 'unrecognized']) {
    const output = setup(shell);
    assert.match(output, /If your shell profile does not already load Vite\+/);
    assert.doesNotMatch(output, /Or open a new terminal/);
    console.log(output);
    freshBash(path.join(system, 'node'));
  }

  console.log('Only the Bash login profile is configured:');
  fs.writeFileSync(path.join(home, '.bash_profile'), source);
  const loginOnly = setup('bash');
  assert.match(loginOnly, /Add the command to ~\/\.bashrc/);
  assert.doesNotMatch(loginOnly, /Or open a new terminal/);
  console.log(loginOnly);
  freshBash(path.join(system, 'node'));

  console.log('Bash .bashrc is configured:');
  fs.writeFileSync(path.join(home, '.bashrc'), source);
  const interactive = setup('bash');
  assert.doesNotMatch(interactive, /Add the command/);
  assert.match(interactive, /Or start an interactive non-login Bash shell/);
  console.log(interactive);
  freshBash(path.join(dirs.bin, 'node'));
}
