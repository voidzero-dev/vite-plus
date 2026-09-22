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

/** @returns {{ system: string, source: string }} */
function prepareSystemNode() {
  captureVp(['env', 'on', 'node']);
  const system = path.resolve('profiles/system');
  fs.mkdirSync(system, { recursive: true });
  fs.writeFileSync(path.join(system, 'node'), '#!/bin/sh\necho system-node\n', { mode: 0o755 });
  // Keep system startup helpers available, with the fake Node first on PATH.
  env.PATH = [system, '/usr/bin', '/bin'].join(path.delimiter);
  const envPath = path.join(dirs.config, 'env').replace(/[\\$`"]/g, '\\$&');
  return { system, source: `. "${envPath}"\n` };
}

if (mode === 'powershell') {
  // Control executable discovery independently of the shells installed on the runner.
  const shellBin = path.resolve('profiles/shell-bin');
  fs.mkdirSync(shellBin, { recursive: true });
  fs.writeFileSync(
    path.join(shellBin, process.platform === 'win32' ? 'powershell.exe' : 'pwsh'),
    '',
    {
      mode: 0o755,
    },
  );
  env.PATH = shellBin;
  delete env.SHELL;
  for (const shell of ['pwsh', undefined]) {
    const output = setup(shell);
    assert.match(output, /\. '[^\n]*env\.ps1'/);
    assert.match(output, /\$PROFILE if it is not already there/);
    assert.doesNotMatch(output, /Or open a new terminal/);
    assert.doesNotMatch(output, /Fish:|Nushell:/);
    console.log(output.split('\n').find((line) => line.includes('$PROFILE')));
  }
} else if (mode.startsWith('cmd')) {
  if (mode === 'cmd') delete env.VP_SELF_SETUP_NO_MODIFY_PATH;
  console.log(`VP_SELF_SETUP_NO_MODIFY_PATH=${env.VP_SELF_SETUP_NO_MODIFY_PATH ?? '<unset>'}:`);
  const output = setup('cmd');
  const activation = output
    .split('\n')
    .find((line) => line.trimStart().startsWith('set "PATH='))
    ?.trim();
  const fallbackBin = path.join(dirs.data, 'fallback-bin');
  assert.equal(activation, `set "PATH=${dirs.bin};%PATH%;${fallbackBin}"`);
  assert.match(output, /user PATH if missing/);
  assert.ok(output.includes(`At the start: ${dirs.bin}`));
  assert.ok(output.includes(`At the end: ${fallbackBin}`));
  assert.match(output, /System Properties -> Environment Variables -> User variables -> Path/);
  assert.doesNotMatch(output, /open a new terminal to load/i);
  console.log(output);

  if (process.platform === 'win32') {
    const system = path.resolve('profiles/system');
    fs.mkdirSync(system, { recursive: true });
    fs.copyFileSync(process.execPath, path.join(system, 'node.exe'));
    const staleEnv = {
      ...env,
      PATH: [system, path.join(process.env.SystemRoot, 'System32')].join(';'),
    };
    for (const [command, expected] of [
      ['where.exe node', path.join(system, 'node.exe')],
      [`${activation} & where.exe node`, path.join(dirs.bin, 'node.exe')],
    ]) {
      const result = spawnSync(process.env.ComSpec, ['/d', '/v:off', '/s', '/c', `"${command}"`], {
        env: staleEnv,
        windowsVerbatimArguments: true,
        encoding: 'utf8',
        timeout: 30000,
      });
      assert.equal(result.status, 0, result.error?.message ?? result.stdout + result.stderr);
      assert.equal(result.stdout.trim().split(/\r?\n/)[0].toLowerCase(), expected.toLowerCase());
    }
  }
} else if (mode.startsWith('zsh-')) {
  const { system, source } = prepareSystemNode();
  fs.mkdirSync(env.ZDOTDIR, { recursive: true });
  // Ubuntu's global compinit can prompt about the runner's completion directories.
  // Keep normal profile loading, but skip completion setup in this PATH test.
  fs.writeFileSync(path.join(env.ZDOTDIR, '.zshenv'), `skip_global_compinit=1\n${source}`);
  // Model PATH changes made by login startup after .zshenv, such as macOS path_helper.
  const systemPath = system.replace(/[\\$`"]/g, '\\$&');
  fs.writeFileSync(path.join(env.ZDOTDIR, '.zprofile'), `export PATH="${systemPath}:$PATH"\n`);
  const configured = mode === 'zsh-interactive';
  if (configured) fs.writeFileSync(path.join(env.ZDOTDIR, '.zshrc'), source);
  console.log(configured ? 'Zsh .zshrc is configured:' : 'Only Zsh .zshenv is configured:');
  const output = setup('zsh');
  if (configured) {
    assert.doesNotMatch(output, /Add the command/);
    assert.match(output, /Or open a new terminal/);
  } else {
    assert.match(output, /Add the command to your \.zshrc/);
    assert.doesNotMatch(output, /Or open a new terminal/);
  }
  console.log(output);
} else {
  // Suppress Ubuntu's sudo hint while still loading the normal Bash startup files.
  fs.writeFileSync(path.join(home, '.hushlogin'), '');
  const { system, source } = prepareSystemNode();
  const bash = process.env.PATH.split(path.delimiter)
    .map((dir) => path.join(dir, 'bash'))
    .find((file) => fs.existsSync(file));
  assert.ok(bash);
  env.SHELL = '/bin/bash';
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

  if (mode === 'unset' || mode === 'unrecognized') {
    console.log('Only Fish is configured:');
    const output = setup(mode === 'unset' ? undefined : mode);
    assert.match(output, /For Bash, run:/);
    assert.match(output, /If your ~\/\.bashrc does not already load Vite\+/);
    assert.doesNotMatch(output, /Or open a new terminal/);
    console.log(output);
    freshBash(path.join(system, 'node'));
  } else if (mode === 'bash-login') {
    console.log('Only the Bash login profile is configured:');
    fs.writeFileSync(path.join(home, '.bash_profile'), source);
    const output = setup('bash');
    assert.match(output, /Add the command to ~\/\.bashrc/);
    assert.doesNotMatch(output, /Or open a new terminal/);
    console.log(output);
    freshBash(path.join(system, 'node'));
  } else {
    assert.equal(mode, 'bash-interactive');
    console.log('Bash .bashrc is configured:');
    fs.writeFileSync(path.join(home, '.bashrc'), source);
    const output = setup('bash');
    assert.doesNotMatch(output, /Add the command/);
    assert.match(output, /Or start an interactive non-login Bash shell/);
    console.log(output);
    freshBash(path.join(dirs.bin, 'node'));
  }
}
