import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vite-plus/test';
import { createCLI } from 'vitest/node';

import { resolveCompletionConsumer } from '../../../tools/src/completion-consumer.ts';
import { extractCompletionMetadata } from '../../build/completion-metadata.ts';
import { VITEST_VERSION } from '../../src/utils/constants.ts';

const temporary: string[] = [];
afterEach(() => {
  for (const root of temporary.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});
const binaryName = process.platform === 'win32' ? 'vp.exe' : 'vp';
function fixture() {
  const root = mkdtempSync(path.join(os.tmpdir(), 'vp-completion-startup-'));
  temporary.push(root);
  const home = path.join(root, 'home'),
    project = path.join(root, 'project'),
    raw = path.join(root, 'external', binaryName);
  mkdirSync(home);
  mkdirSync(project);
  mkdirSync(path.dirname(raw));
  copyFileSync(resolveCompletionConsumer(), raw);
  if (process.platform === 'win32') {
    copyFileSync(raw, path.join(path.dirname(raw), 'vp-shim.exe'));
  }
  writeFileSync(path.join(project, 'package.json'), '{"private":true}');
  writeFileSync(
    path.join(project, 'vite.config.ts'),
    "throw Error('Completion must not load config')",
  );
  const canary = path.join(root, 'canary');
  mkdirSync(canary);
  const log = path.join(root, 'node-called');
  writeFileSync(
    path.join(canary, 'node'),
    `#!/bin/sh\nprintf called >> '${log.replaceAll("'", "'\\''")}'\nexit 93\n`,
  );
  chmodSync(path.join(canary, 'node'), 0o755);
  writeFileSync(
    path.join(canary, 'node.cmd'),
    `@echo off\r\necho called >> "${log}"\r\nexit /b 93\r\n`,
  );
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    HOME: home,
    USERPROFILE: home,
    VP_HOME: path.join(home, '.vite-plus'),
    PATH: canary + path.delimiter + (process.env.PATH ?? ''),
    CI: 'true',
    VP_SKIP_DEPS_INSTALL: '1',
    VP_NODE_MANAGER: 'no',
  };
  for (const key of [
    'VP_VERSION',
    'VP_GLOBAL_CLI_JS_SCRIPTS_DIR',
    'VP_SELF_SETUP_SHELL',
    'VP_SELF_SETUP_REPLACE_EXISTING',
    'VP_COMPLETE',
  ]) {
    delete env[key];
  }
  return { root, home, project, raw, env, log };
}
function run(f: ReturnType<typeof fixture>, binary: string, args: string[], completion = true) {
  return spawnSync(binary, args, {
    cwd: f.project,
    encoding: 'utf8',
    timeout: 15000,
    env: {
      ...f.env,
      ...(completion ? { VP_COMPLETE: 'bash', _CLAP_COMPLETE_INDEX: '2', _CLAP_IFS: '\n' } : {}),
    },
  });
}
function packageMetadata(pkg: string) {
  mkdirSync(path.join(pkg, 'dist', 'completion'), { recursive: true });
  writeFileSync(
    path.join(pkg, 'package.json'),
    JSON.stringify({ name: 'vite-plus', version: '1.3.0' }),
  );
  writeFileSync(path.join(pkg, 'dist', 'bin.js'), "throw Error('Completion must not execute JS')");
  const cli = createCLI();
  cli.commands.splice(0);
  cli.globalCommand.options.splice(0);
  cli.command('run [filters...]').option('--contract-global', 'Controlled completion flag');
  writeFileSync(
    path.join(pkg, 'dist/completion/test.json'),
    JSON.stringify(extractCompletionMetadata(cli, '1.3.0', VITEST_VERSION)),
  );
}
function vitestPackage(dir: string) {
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    path.join(dir, 'package.json'),
    JSON.stringify({ name: 'vitest', version: VITEST_VERSION }),
  );
}
function snapshot(root: string): Record<string, string> {
  const result: Record<string, string> = {};
  function walk(dir: string) {
    for (const name of readdirSync(dir).toSorted()) {
      const file = path.join(dir, name),
        st = lstatSync(file);
      const key = path.relative(root, file);
      if (st.isSymbolicLink()) {
        result[key] = `link:${readlinkSync(file)}`;
      } else if (st.isDirectory()) {
        result[key] = 'directory';
        walk(file);
      } else {
        result[key] = createHash('sha256').update(readFileSync(file)).digest('hex');
      }
    }
  }
  walk(root);
  return result;
}
const request = ['--', 'vp', 'test', '--contract-glo'];

describe('read-only completion before first-start setup', () => {
  it.each(['flat', 'pnpm-link'])(
    'finds the global package without local dependencies in %s layout',
    (layout) => {
      const f = fixture(),
        prefix = path.join(f.root, 'global'),
        logical = path.join(prefix, 'node_modules/vite-plus');
      const pkg =
        layout === 'flat' ? logical : path.join(prefix, '.pnpm/cli/node_modules/vite-plus');
      packageMetadata(pkg);
      vitestPackage(path.join(path.dirname(pkg), 'vitest'));
      if (layout === 'pnpm-link') {
        mkdirSync(path.dirname(logical), { recursive: true });
        symlinkSync(pkg, logical, 'junction');
      }
      const binary = path.join(prefix, 'bin', binaryName);
      mkdirSync(path.dirname(binary));
      copyFileSync(f.raw, binary);
      writeFileSync(path.join(path.dirname(binary), '.vp-setup-complete'), '');
      const before = snapshot(f.root),
        result = run(f, binary, request);
      expect(result.error).toBeUndefined();
      expect(result.status, result.stderr).toBe(0);
      expect(result.stderr).toBe('');
      expect(result.stdout.split('\n').map((l) => l.split('\t')[0])).toContain('--contract-global');
      expect(snapshot(f.root)).toEqual(before);
      expect(existsSync(f.log)).toBe(false);
    },
  );

  it.each([
    'valid',
    'missing',
    'malformed',
    'stale-source',
    'missing-target',
    'missing-marker',
    'self-reference',
    ...(process.platform === 'win32' ? [] : ['target-alias-to-source', 'non-executable-target']),
  ])('uses only a completed installation for %s receipt', (mode) => {
    const f = fixture();
    // Create the receipt using the real installer, offline; completion must never repeat it.
    const setup = run(f, f.raw, ['--help'], false);
    expect(setup.error).toBeUndefined();
    expect(setup.status, setup.stderr).toBe(0);
    const current = path.join(f.env.VP_HOME!, 'current'),
      deployed = path.join(current, 'bin', binaryName);
    const pkg = path.join(current, 'node_modules/vite-plus');
    packageMetadata(pkg);
    vitestPackage(path.join(pkg, 'node_modules/vitest'));
    const records = path.join(f.env.VP_HOME!, 'self-setup'),
      file = path.join(records, readdirSync(records)[0]);
    const receipt = JSON.parse(readFileSync(file, 'utf8'));
    if (mode === 'missing') {
      rmSync(file);
    }
    if (mode === 'malformed') {
      writeFileSync(file, '{');
    }
    if (mode === 'stale-source') {
      receipt.source.len += 1;
      writeFileSync(file, JSON.stringify(receipt));
    }
    if (mode === 'non-executable-target') {
      chmodSync(deployed, 0o600);
    }
    if (mode === 'missing-target') {
      rmSync(deployed);
    }
    if (mode === 'missing-marker') {
      rmSync(path.join(path.dirname(deployed), '.vp-setup-complete'));
    }
    if (mode === 'self-reference') {
      receipt.binary = f.raw;
      writeFileSync(file, JSON.stringify(receipt));
    }
    if (mode === 'target-alias-to-source') {
      rmSync(deployed);
      symlinkSync(f.raw, deployed);
    }
    const before = snapshot(f.root),
      result = run(f, f.raw, request);
    expect(result.error).toBeUndefined();
    expect(result.status, result.stderr).toBe(0);
    expect(result.stderr).toBe('');
    if (mode === 'valid') {
      expect(result.stdout.split('\n').map((l) => l.split('\t')[0])).toContain('--contract-global');
    } else {
      expect(result.stdout).toBe('');
    }
    const registration = run(f, f.raw, []);
    expect(registration.status, registration.stderr).toBe(0);
    expect(registration.stderr).toBe('');
    expect(registration.stdout).toContain('VP_COMPLETE');
    expect(snapshot(f.root)).toEqual(before);
    expect(existsSync(f.log)).toBe(false);
  });
});
