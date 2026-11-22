import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';

const binary = join(process.env.VP_HOME, 'current/bin/vp');
const user = mkdtempSync(join(process.cwd(), 'unsafe-root-'));
const data = join(user, 'data');
const personal = join(data, 'personal');
const links = join(user, 'links');
const bin = join(user, 'shared-bin');
const config = join(user, '.config/vite-plus');
const env = {
  ...process.env,
  HOME: user,
  USERPROFILE: user,
  ZDOTDIR: user,
  XDG_CONFIG_HOME: join(user, '.config'),
  XDG_DATA_HOME: join(user, '.local/share'),
  XDG_STATE_HOME: join(user, '.local/state'),
  VP_DATA_DIR: data,
  // Keep '..' intact so the CLI, not node:path, interprets the override.
  VP_CACHE_DIR: `${links}/route/../personal`,
  VP_BIN_DIR: bin,
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
  CI: '1',
  NO_COLOR: '1',
};
delete env.VP_HOME;

try {
  for (const directory of [join(data, 'child'), personal, links, bin, config])
    mkdirSync(directory, { recursive: true });
  writeFileSync(join(data, '.vp-owned'), 'vite-plus-owned-directory\n');
  writeFileSync(join(personal, 'notes.txt'), 'keep my notes\n');
  symlinkSync(join(data, 'child'), join(links, 'route'));
  symlinkSync(binary, join(bin, 'vp'));
  const profile = `# personal config\n# Vite+ bin\n. "${config}/env"\n`;
  writeFileSync(join(user, '.bashrc'), profile);

  const result = spawnSync(binary, ['implode', '--yes'], {
    env,
    encoding: 'utf8',
    timeout: 30000,
  });
  assert.equal(result.status, 1, `${result.error ?? ''}\n${result.stdout}\n${result.stderr}`);
  const output = result.stdout + result.stderr;
  assert.match(output, /Refusing to normalize directory/);
  assert.match(output, /Specify the directory without '\.\.'/);
  assert.equal(readFileSync(join(personal, 'notes.txt'), 'utf8'), 'keep my notes\n');
  assert.equal(readFileSync(join(user, '.bashrc'), 'utf8'), profile);
  assert.equal(readlinkSync(join(bin, 'vp')), binary);
  assert.equal(existsSync(join(data, '.vp-owned')), true);
  assert.equal(existsSync(join(links, 'personal')), false);
  console.log('A symlink followed by parent traversal is rejected even with --yes.');
  console.log('The marked data root, personal files, shell profile, and shim remain unchanged.');
} finally {
  rmSync(user, { recursive: true, force: true });
}
