// Run only in a disposable prefix. This never uses the user's Homebrew installation.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import { once } from 'node:events';
import fs from 'node:fs/promises';
import { createServer } from 'node:http';
import { homedir, tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { targets, updateFormula } from './update-homebrew-formula.mjs';

const { values } = parseArgs({
  options: {
    binary: { type: 'string' },
    'packages-dir': { type: 'string' },
    'replacement-binary': { type: 'string' },
    'replacement-version': { type: 'string' },
    'brew-source': { type: 'string', default: 'https://github.com/Homebrew/brew' },
  },
});
assert.ok(values.binary, '--binary is required');
const repository = fileURLToPath(new URL('../../', import.meta.url));
const version = (
  await fs.readFile(path.join(repository, 'crates/vp_global_cli/Cargo.toml'), 'utf8')
).match(/^version = "([^"]+)"/m)[1];
const replacementVersion = values['replacement-version'] ?? version;
// Linux's build sandbox permits writes under /tmp, so keep Homebrew itself outside it.
const testParent = process.platform === 'linux' ? homedir() : tmpdir();
const root = await fs.realpath(await fs.mkdtemp(path.join(testParent, 'vp-homebrew-e2e-')));
const prefix = path.join(root, 'brew');
const brew = path.join(prefix, 'bin/brew');
const formulaName = 'viteplus/e2e/vp';
const platform = `${process.platform}-${process.arch}${process.platform === 'linux' ? '-gnu' : ''}`;
assert.ok(targets.some(([, , , suffix]) => suffix === platform));
const baseEnv = Object.fromEntries(
  Object.entries(process.env).filter(
    ([name]) => !/^(VP_|XDG_|HOMEBREW_|npm_|NPM_|PNPM_)/.test(name),
  ),
);
const children = [];
const servers = [];
const token = randomBytes(24).toString('hex');

async function run(command, args, env = baseEnv, cwd = root, expected = 0) {
  const child = spawn(command, args, { env, cwd, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  for (const stream of [child.stdout, child.stderr]) {
    stream.setEncoding('utf8').on('data', (data) => {
      output += data;
    });
  }
  const [code, signal] = await once(child, 'close');
  assert.equal(signal, null, output);
  assert.ok(!output.includes(token), 'Registry credentials appeared in command output');
  assert.equal(code, expected, `${command} ${args.join(' ')}\n${output}`);
  return output;
}

async function registry(packagesDir) {
  const args = [path.join(repository, 'packages/tools/src/local-npm-registry.ts'), '--serve'];
  if (packagesDir) {
    args.push('--packages-dir', packagesDir);
  }
  const child = spawn(process.execPath, args, {
    env: baseEnv,
    cwd: root,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  children.push(child);
  return await new Promise((resolve, reject) => {
    let output = '';
    const timer = setTimeout(
      () => reject(new Error(`Registry failed to start: ${output}`)),
      30_000,
    );
    child.once('error', reject);
    child.once('exit', (code) => {
      clearTimeout(timer);
      reject(new Error(`Registry exited: ${code} ${output}`));
    });
    child.stderr.on('data', (data) => {
      output += data;
    });
    child.stdout.setEncoding('utf8').on('data', (data) => {
      output += data;
      const handshake = output.split('\n').find((line) => line.startsWith('{"registry":'));
      if (handshake) {
        clearTimeout(timer);
        resolve(JSON.parse(handshake).registry);
      }
    });
  });
}

// Emulate an enterprise registry. Credentials terminate here and are never sent upstream.
async function authenticatedRegistry(upstream) {
  const requests = new Set();
  const server = createServer(async (request, response) => {
    if (request.headers.authorization !== `Bearer ${token}`) {
      response.writeHead(401).end('Unauthorized');
      return;
    }
    requests.add(request.url);
    try {
      const remote = request.url.startsWith('/tarball/')
        ? decodeURIComponent(request.url.slice('/tarball/'.length))
        : upstream + request.url;
      assert.ok(
        [new URL(upstream).origin, 'https://registry.npmjs.org'].includes(new URL(remote).origin),
      );
      const result = await fetch(remote, {
        headers: { accept: request.headers.accept ?? '*/*' },
        signal: AbortSignal.timeout(120_000),
      });
      response.statusCode = result.status;
      const contentType = result.headers.get('content-type') ?? '';
      response.setHeader('content-type', contentType);
      if (contentType.includes('json')) {
        const text = JSON.stringify(await result.json(), (key, value) =>
          key === 'tarball' && typeof value === 'string'
            ? `${url}/tarball/${encodeURIComponent(value)}`
            : value,
        );
        response.end(text);
      } else {
        response.end(Buffer.from(await result.arrayBuffer()));
      }
    } catch {
      response.writeHead(502).end('Registry fixture failed');
    }
  });
  servers.push(server);
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const url = `http://127.0.0.1:${server.address().port}`;
  return { url, requests };
}

async function user(name, registryUrl, authenticated = false) {
  const home = path.join(root, name);
  await fs.mkdir(home);
  await fs.writeFile(
    path.join(home, '.npmrc'),
    `registry=${registryUrl}/\n${authenticated ? `//${new URL(registryUrl).host}/:_authToken=\${NPM_TOKEN}\n` : ''}`,
  );
  return {
    ...baseEnv,
    HOME: home,
    VP_HOME: path.join(home, '.vite-plus'),
    NPM_TOKEN: token,
    VP_NODE_MANAGER: 'yes',
    VP_PM_MANAGER: 'no',
    VP_PNPM_MANAGER: 'yes',
    VP_SELF_SETUP_NO_MODIFY_PATH: '1',
    CI: 'true',
    PATH: [
      path.join(home, '.vite-plus/bin'),
      path.join(prefix, 'bin'),
      '/usr/bin',
      '/bin',
      '/usr/sbin',
      '/sbin',
      path.join(home, '.vite-plus/fallback-bin'),
    ].join(path.delimiter),
  };
}

const packageDir = (env, value = version) =>
  path.join(env.VP_HOME, 'cli-packages', value, platform);
const shim = (env, tool) => path.join(env.VP_HOME, 'bin', tool);
const publicVp = path.join(prefix, 'bin/vp');
const formula = await fs.readFile(path.join(repository, 'HomebrewFormula/vp.rb'), 'utf8');

async function formulaFor(binary, value, registryUrl, revision = false) {
  const payload = path.join(root, `payload-${value}${revision ? '-revision' : ''}`);
  await fs.mkdir(payload);
  await fs.copyFile(path.resolve(binary), path.join(payload, 'vp'));
  await fs.chmod(path.join(payload, 'vp'), 0o755);
  const archive = `${payload}.tar.gz`;
  await run('tar', ['-czf', archive, '-C', payload, 'vp']);
  const sha256 = createHash('sha256')
    .update(await fs.readFile(archive))
    .digest('hex');
  const assets = targets.map(([os, arch]) => ({
    os,
    arch,
    url: pathToFileURL(archive).href,
    sha256,
  }));
  let result = updateFormula(formula, value, assets);
  // Local archive URLs have no GitHub release tag from which Homebrew can infer the version.
  result = result.replace('  license', `  version "${value}"\n  license`);
  if (revision) {
    result = result.replace('  license', '  revision 1\n  license');
  }
  // brew test uses a separate home. Point only this disposable formula at the checkout registry.
  result = result.replace(
    '  test do\n',
    `  test do\n    ENV["NPM_CONFIG_REGISTRY"] = "${registryUrl}"\n`,
  );
  return result;
}

try {
  const upstream = await registry(values['packages-dir'] && path.resolve(values['packages-dir']));
  const privateRegistry = await authenticatedRegistry(upstream);
  await run('git', ['clone', '--depth', '1', values['brew-source'], prefix]);
  const brewHome = path.join(root, 'brew-home');
  const brewTemp = path.join(root, 'brew-tmp');
  await fs.mkdir(brewHome);
  await fs.mkdir(brewTemp);
  const noNpm = path.join(root, 'no-npm');
  await fs.mkdir(noNpm);
  for (const tool of ['node', 'npm', 'pnpm']) {
    await fs.writeFile(
      path.join(noNpm, tool),
      '#!/bin/sh\necho "npm tools must not run during brew install" >&2\nexit 77\n',
      { mode: 0o755 },
    );
  }
  const brewEnv = {
    ...baseEnv,
    HOME: brewHome,
    HOMEBREW_TEMP: brewTemp,
    HOMEBREW_CACHE: path.join(root, 'brew-cache'),
    HOMEBREW_LOGS: path.join(root, 'brew-logs'),
    HOMEBREW_NO_AUTO_UPDATE: '1',
    HOMEBREW_NO_ANALYTICS: '1',
    HOMEBREW_NO_ENV_HINTS: '1',
    HOMEBREW_NO_INSTALL_CLEANUP: '1',
    PATH: [noNpm, '/usr/bin', '/bin', '/usr/sbin', '/sbin'].join(path.delimiter),
  };
  const tapSource = path.join(root, 'tap');
  await fs.mkdir(path.join(tapSource, 'HomebrewFormula'), { recursive: true });
  await fs.writeFile(
    path.join(tapSource, 'HomebrewFormula/vp.rb'),
    await formulaFor(values.binary, version, upstream),
  );
  await run('git', ['init', tapSource]);
  await run('git', ['-C', tapSource, 'add', '.']);
  await run('git', [
    '-C',
    tapSource,
    '-c',
    'user.name=Vite+ E2E',
    '-c',
    'user.email=e2e@example.test',
    '-c',
    'commit.gpgsign=false',
    'commit',
    '-m',
    'Test tap',
  ]);
  await run(brew, ['tap', 'viteplus/e2e', pathToFileURL(tapSource).href], brewEnv);
  await run(brew, ['install', '--build-from-source', formulaName], brewEnv);
  const oldKeg = path.dirname(path.dirname(await fs.realpath(publicVp)));
  const before = createHash('sha256')
    .update(await fs.readFile(publicVp))
    .digest('hex');
  assert.ok(!(await fs.readdir(brewHome)).includes('.vite-plus'));
  assert.ok(!(await fs.readdir(oldKeg)).includes('node_modules'));
  for (const alias of ['vpr', 'vpx']) {
    assert.equal(await fs.realpath(path.join(prefix, 'bin', alias)), await fs.realpath(publicVp));
  }
  console.log(
    'Homebrew installs the current native binary and aliases without npm tools or user setup.',
  );

  const first = await user('first-user', upstream);
  const second = await user('second-user', privateRegistry.url, true);
  const project = path.join(root, 'project');
  await fs.mkdir(project);
  await fs.writeFile(
    path.join(project, 'package.json'),
    JSON.stringify({
      name: 'homebrew-e2e',
      private: true,
      type: 'module',
      packageManager: 'pnpm@10.33.0',
    }),
  );
  await fs.writeFile(
    path.join(project, 'index.html'),
    '<script type="module" src="/src.js"></script>',
  );
  await fs.writeFile(path.join(project, 'src.js'), "document.body.textContent = 'Homebrew';\n");
  await fs.chmod(oldKeg, 0o555);
  await fs.chmod(path.join(oldKeg, 'bin'), 0o555);
  await Promise.all([
    run(publicVp, ['--help'], first, project),
    run(publicVp, ['--help'], first, project),
  ]);
  await run(publicVp, ['--help'], second, project);
  assert.ok([...privateRegistry.requests].some((url) => url.startsWith('/pnpm')));
  assert.ok([...privateRegistry.requests].some((url) => url.startsWith('/vite-plus')));
  assert.equal(
    createHash('sha256')
      .update(await fs.readFile(publicVp))
      .digest('hex'),
    before,
  );
  assert.ok(!(await fs.readdir(path.join(oldKeg, 'bin'))).includes('.vp-setup-complete'));
  assert.ok(!(await fs.readdir(first.VP_HOME)).includes('current'));
  const settings = await fs.readFile(path.join(first.VP_HOME, 'config.json'), 'utf8');
  for (const env of [first, second]) {
    for (const variable of ['VP_NODE_MANAGER', 'VP_PM_MANAGER', 'VP_PNPM_MANAGER']) {
      delete env[variable];
    }
  }
  const marker = path.join(packageDir(first), '.vp-deps-complete');
  const completed = (await fs.stat(marker)).mtimeMs;
  await run(shim(first, 'vp'), ['--help'], first, project);
  assert.equal((await fs.stat(marker)).mtimeMs, completed);
  for (const args of [
    ['fmt', 'src.js'],
    ['fmt', '--check', 'src.js'],
    ['lint', 'src.js'],
    ['build'],
  ]) {
    await run(publicVp, args, first, project);
  }
  assert.ok(!(await fs.readdir(project)).includes('node_modules'));
  await fs.access(path.join(project, 'dist/index.html'));
  console.log(
    'Public and authenticated setup install matching dependencies; fmt, lint, and build use the global package.',
  );

  await run(publicVp, ['install', '-g', 'typescript@5.8.3'], first, project);
  const previous = path.join(first.VP_HOME, 'script-install/bin/vp');
  await fs.mkdir(path.dirname(previous), { recursive: true });
  await fs.copyFile(path.resolve(values.binary), previous);
  await fs.symlink('script-install', path.join(first.VP_HOME, 'current'));
  for (const name of ['vp', 'tsc']) {
    await fs.unlink(shim(first, name));
    await fs.symlink(path.join(first.VP_HOME, 'current/bin/vp'), shim(first, name));
  }
  await run(publicVp, ['env', 'setup', '--refresh'], first, project);
  assert.equal(await fs.readlink(shim(first, 'tsc')), publicVp);
  await fs.rm(path.join(first.VP_HOME, 'script-install'), { recursive: true });
  await run(shim(first, 'tsc'), ['--version'], first, project);
  assert.equal(await fs.readFile(path.join(first.VP_HOME, 'config.json'), 'utf8'), settings);
  console.log(
    'Migration refreshes global-package shims without replacing preferences or installed packages.',
  );

  let nextRegistry = upstream;
  if (replacementVersion !== version) {
    assert.ok(values['replacement-binary'] && values['packages-dir']);
    const packages = path.join(root, 'next-packages');
    await fs.cp(path.resolve(values['packages-dir']), packages, { recursive: true });
    for (const archive of await fs.readdir(packages)) {
      if (!archive.endsWith('.tgz')) {
        continue;
      }
      const unpacked = await fs.mkdtemp(path.join(root, 'next-package-'));
      await run('tar', ['-xzf', path.join(packages, archive), '-C', unpacked]);
      const manifestPath = path.join(unpacked, 'package/package.json');
      const manifest = JSON.parse(await fs.readFile(manifestPath, 'utf8'));
      manifest.version = replacementVersion;
      if (manifest.name === 'vite-plus') {
        manifest.dependencies.vite = `npm:@voidzero-dev/vite-plus-core@${replacementVersion}`;
      }
      await fs.writeFile(manifestPath, JSON.stringify(manifest));
      await run('tar', ['-czf', path.join(packages, archive), '-C', unpacked, 'package']);
    }
    nextRegistry = await registry(packages);
  }
  await fs.chmod(oldKeg, 0o755);
  await fs.chmod(path.join(oldKeg, 'bin'), 0o755);
  const tap = (await run(brew, ['--repository', 'viteplus/e2e'], brewEnv)).trim();
  await fs.writeFile(
    path.join(tap, 'HomebrewFormula/vp.rb'),
    await formulaFor(
      values['replacement-binary'] ?? values.binary,
      replacementVersion,
      nextRegistry,
      replacementVersion === version,
    ),
  );
  await run(brew, ['upgrade', '--build-from-source', formulaName], brewEnv);
  await run(brew, ['cleanup', '--prune=all', formulaName], brewEnv);
  await assert.rejects(fs.access(oldKeg));
  await fs.writeFile(path.join(first.HOME, '.npmrc'), `registry=${nextRegistry}/\n`);
  await run(shim(first, 'vp'), ['--help'], first, project);
  for (const name of ['node', 'pn', 'tsc']) {
    await run(shim(first, name), ['--version'], first, project);
  }
  await run(shim(first, 'pnx'), ['--help'], first, project);
  await run(path.join(first.VP_HOME, 'fallback-bin/npm'), ['--version'], first, project);
  await fs.access(path.join(packageDir(first, replacementVersion), '.vp-deps-complete'));
  await run(publicVp, ['build'], first, project);
  assert.equal(await fs.readFile(path.join(first.VP_HOME, 'config.json'), 'utf8'), settings);
  assert.ok(
    (await run(publicVp, ['upgrade'], first, project, 1)).includes(`brew upgrade ${formulaName}`),
  );
  assert.ok(
    (await run(publicVp, ['upgrade', '--check'], first, project)).includes(
      `brew outdated ${formulaName}`,
    ),
  );
  await run(brew, ['test', formulaName], brewEnv);
  if (process.platform === 'linux' && process.arch === 'x64') {
    const file = path.join(tap, 'HomebrewFormula/vp.rb');
    const fixture = await fs.readFile(file, 'utf8');
    try {
      // Audit the production URLs and release gate, not the local archive override.
      await fs.writeFile(file, formula);
      await run(brew, ['style', formulaName], brewEnv);
      await run(brew, ['audit', '--strict', '--formula', formulaName], brewEnv);
    } finally {
      await fs.writeFile(file, fixture);
    }
  }
  console.log(
    'Homebrew upgrade and old-keg cleanup preserve both shim directories and saved preferences.',
  );

  await run(publicVp, ['implode', '--yes'], first, project);
  await assert.rejects(fs.access(first.VP_HOME));
  await fs.access(publicVp);
  await run(brew, ['uninstall', formulaName], brewEnv);
  await assert.rejects(fs.access(publicVp));
  await fs.access(path.join(packageDir(second), '.vp-deps-complete'));
  console.log(
    'Implode removes one user installation; brew uninstall leaves the other user data intact.',
  );
} finally {
  for (const server of servers) {
    server.close();
    server.closeAllConnections();
  }
  for (const child of children) {
    child.kill();
  }
  // Keep failures available for inspection; the prefix is always task-local.
  console.log(`Homebrew e2e files: ${root}`);
}
