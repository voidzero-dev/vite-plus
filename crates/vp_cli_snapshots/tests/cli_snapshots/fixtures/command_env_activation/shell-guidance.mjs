import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const mode = process.argv[2];
const home = path.resolve('shell-guidance/user');
const shellBin = path.resolve('shell-guidance/bin');
const binary = path.join(process.env.VP_HOME, 'current/bin/vp');
const env = { ...process.env, HOME: home, ZDOTDIR: home, PATH: shellBin };
delete env.VP_SHELL;
delete env.SHELL;
fs.mkdirSync(home, { recursive: true });
fs.mkdirSync(shellBin, { recursive: true });

/** @returns {string} */
function capture(args, extra = {}) {
  const result = spawnSync(binary, args, {
    env: { ...env, ...extra },
    encoding: 'utf8',
    timeout: 30000,
  });
  assert.equal(result.status, 0, result.error?.message ?? result.stdout + result.stderr);
  return result.stdout.replace(/\u001b\[[0-9;]*m/g, '').replaceAll('\r\n', '\n');
}

const dirs = Object.fromEntries(
  capture([], { VP_DUMP_DIRS: '1' })
    .trim()
    .split('\n')
    .map((line) => line.split('\t')),
);

/** @returns {string} */
function setup() {
  const output = capture(['env', 'setup']);
  const heading = output.indexOf('Next Steps:\n');
  assert.ok(heading >= 0, output);
  const instructions = output.slice(heading).trimEnd();
  assert.doesNotMatch(instructions, /Or open a new terminal/);
  console.log(instructions);
  return instructions;
}

if (mode === 'hints') {
  // Even matching profiles must not turn a login-shell hint into a current-shell guarantee.
  for (const profile of ['.zshrc', '.bashrc']) {
    fs.writeFileSync(path.join(home, profile), `. "${dirs.config}/env"\n`);
  }
  for (const [shell, label, file] of [
    ['zsh', 'Zsh', 'env'],
    ['bash', 'Bash', 'env'],
    ['sh', 'sh', 'env'],
    ['fish', 'Fish', 'env.fish'],
    ['nu', 'Nushell', 'env.nu'],
    ['pwsh', 'PowerShell', 'env.ps1'],
  ]) {
    env.SHELL = `/usr/bin/${shell}`;
    console.log(`SHELL=${env.SHELL}, VP_SHELL=<unset>:`);
    const output = setup();
    assert.ok(output.includes(`For ${label}, run:`));
    assert.ok(output.includes(path.join(dirs.config, file)));
    assert.equal(output.split('\n').filter((line) => line.includes(dirs.config)).length, 1);
    assert.doesNotMatch(output, /Activate Vite\+ in this terminal/);
  }
  env.SHELL = '/bin/zsh';
  env.VP_SHELL = 'unrecognized';
  console.log('SHELL=/bin/zsh, VP_SHELL=unrecognized:');
  assert.match(setup(), /For Zsh, run:/);
  env.VP_SHELL = 'fish';
  console.log('SHELL=/bin/zsh, VP_SHELL=fish (Fish is not on PATH):');
  const output = setup();
  assert.match(output, /Activate Vite\+ in this terminal:/);
  assert.match(output, /env\.fish/);
  assert.doesNotMatch(output, /For Zsh|env\.ps1|env\.nu/);
} else {
  assert.equal(mode, 'available');
  env.SHELL = '/bin/unrecognized';
  for (const shell of ['fish', 'nu', 'pwsh']) {
    fs.writeFileSync(path.join(shellBin, shell), '', { mode: 0o644 });
  }
  console.log('Unknown shell; optional shell files are not executable:');
  assert.doesNotMatch(setup(), /Fish:|Nushell:|PowerShell:|\$PROFILE/);
  for (const [shell, label] of [
    ['fish', 'Fish'],
    ['nu', 'Nushell'],
    ['pwsh', 'PowerShell'],
  ]) {
    fs.chmodSync(path.join(shellBin, shell), 0o755);
    console.log(`Unknown shell; only ${label} is available:`);
    const output = setup();
    assert.ok(output.includes(`${label}:`));
    for (const other of ['Fish', 'Nushell', 'PowerShell'].filter((name) => name !== label)) {
      assert.ok(!output.includes(`${other}:`));
    }
    assert.equal(output.includes('$PROFILE'), shell === 'pwsh');
    fs.chmodSync(path.join(shellBin, shell), 0o644);
  }
}
