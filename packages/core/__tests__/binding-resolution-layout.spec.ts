import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterAll, describe, expect, it } from 'vitest';

import { rewriteRolldownBindingRequires } from '../build-support/rewrite-rolldown-binding.ts';

// Reproduces issue #2054 at the module resolution layer. pnpm's
// enable-global-virtual-store installs packages at a realpath outside the
// project, where only declared dependencies are resolvable siblings; Node
// resolves from the realpath, so the project's own node_modules is never on
// the walk-up path. These tests rebuild that layout with stub packages and
// prove the collapsed `vite-plus/binding` rewrite fails there while the
// per-platform transform output resolves. Full-stack coverage with real
// release artifacts lives in the preview pipeline (see
// rfcs/core-binding-resolution.md).

const PLATFORM_SUFFIX = 'linux-x64-gnu';
const PLATFORM_PACKAGE = `@voidzero-dev/vite-plus-${PLATFORM_SUFFIX}`;
const CORE_VERSION = '0.0.0-layout-test';

// The napi-rs loader shape for one platform branch, before any release
// rewrite. Resolution does not depend on the host platform, so the branch is
// unconditional here.
const upstreamLoader = `
const binding = require("@rolldown/binding-${PLATFORM_SUFFIX}");
const bindingPackageVersion = require("@rolldown/binding-${PLATFORM_SUFFIX}/package.json").version;
if (bindingPackageVersion !== "1.2.1" && process.env.NAPI_RS_ENFORCE_VERSION_CHECK && process.env.NAPI_RS_ENFORCE_VERSION_CHECK !== "0") throw new Error(\`Native binding package version mismatch, expected 1.2.1 but got \${bindingPackageVersion}. You can reinstall dependencies to fix this issue.\`);
module.exports = binding;
`;

// The collapsed rewrite shipped before this fix (packages/core/build.ts).
const collapsedLoader = upstreamLoader.replace(
  /@rolldown\/binding-([a-z0-9-]+)/g,
  'vite-plus/binding',
);

const rewrittenLoader = rewriteRolldownBindingRequires(upstreamLoader, {
  packageName: '@voidzero-dev/vite-plus',
  platformSuffixes: new Set([PLATFORM_SUFFIX]),
  version: CORE_VERSION,
}).source;

const tmpRoots: string[] = [];
afterAll(() => {
  for (const root of tmpRoots) {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

function write(file: string, content: string) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, content);
}

function writeJson(file: string, data: unknown) {
  write(file, JSON.stringify(data, null, 2) + '\n');
}

/**
 * Builds the pnpm enable-global-virtual-store layout:
 *
 *   <root>/global-store/.pnpm/<pkg>/node_modules/<name>   package realpaths
 *   <root>/project/node_modules/<name>                    symlinks into the store
 *
 * Core's realpath directory contains only its declared dependencies; the
 * project's node_modules (including the undeclared `vite-plus`) is not on
 * core's resolution path, exactly like pnpm's global virtual store.
 */
function buildGlobalVirtualStoreLayout(options: {
  loader: string;
  platformPackageVersion?: string;
}): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-binding-layout-'));
  tmpRoots.push(root);

  const storeCoreDir = path.join(
    root,
    'global-store/.pnpm/core@0.0.0/node_modules/@voidzero-dev/vite-plus-core',
  );
  writeJson(path.join(storeCoreDir, 'package.json'), {
    name: '@voidzero-dev/vite-plus-core',
    version: CORE_VERSION,
    main: './dist/rolldown-loader.cjs',
    // Layout fidelity: a real published core declares the platform package it
    // loads (injected at publish time, pinned to core's own version).
    ...(options.platformPackageVersion
      ? { optionalDependencies: { [PLATFORM_PACKAGE]: CORE_VERSION } }
      : {}),
  });
  write(path.join(storeCoreDir, 'dist/rolldown-loader.cjs'), options.loader);

  if (options.platformPackageVersion) {
    const storePlatformDir = path.join(
      root,
      `global-store/.pnpm/platform@0.0.0/node_modules/${PLATFORM_PACKAGE}`,
    );
    writeJson(path.join(storePlatformDir, 'package.json'), {
      name: PLATFORM_PACKAGE,
      version: options.platformPackageVersion,
      main: './binding.cjs',
    });
    write(path.join(storePlatformDir, 'binding.cjs'), `module.exports = { marker: 'native' };\n`);
    // Declared dependency: a resolvable sibling of core's realpath, i.e.
    // under the store entry's node_modules directory (two levels above the
    // scoped core package directory).
    const storeModulesDir = path.dirname(path.dirname(storeCoreDir));
    const link = path.join(storeModulesDir, PLATFORM_PACKAGE);
    fs.mkdirSync(path.dirname(link), { recursive: true });
    fs.symlinkSync(storePlatformDir, link, 'junction');
  }

  // The project depends on vite-plus and core; vite-plus (with its ./binding
  // export) sits in the project's node_modules like a real install, but core
  // resolves from its store realpath and cannot see it.
  const projectModules = path.join(root, 'project/node_modules');
  const vitePlusDir = path.join(projectModules, 'vite-plus');
  writeJson(path.join(vitePlusDir, 'package.json'), {
    name: 'vite-plus',
    version: CORE_VERSION,
    exports: {
      './binding': './binding/index.cjs',
      './package.json': './package.json',
    },
  });
  write(path.join(vitePlusDir, 'binding/index.cjs'), `module.exports = { marker: 'native' };\n`);
  const coreLink = path.join(projectModules, '@voidzero-dev/vite-plus-core');
  fs.mkdirSync(path.dirname(coreLink), { recursive: true });
  fs.symlinkSync(storeCoreDir, coreLink, 'junction');

  write(
    path.join(root, 'project/main.cjs'),
    `console.log(require('@voidzero-dev/vite-plus-core').marker);\n`,
  );
  return root;
}

function runProject(root: string, env: Record<string, string> = {}) {
  const childEnv: Record<string, string | undefined> = {
    ...process.env,
    NAPI_RS_ENFORCE_VERSION_CHECK: '',
    ...env,
  };
  // The layout under test is a hermetic pnpm global virtual store. An ambient
  // NODE_PATH (pnpm sets it to the outer repo's `.pnpm/node_modules` when tests
  // run via `pnpm test`) would put real hoisted packages on the child's
  // resolution path and defeat the isolation this test depends on — the real
  // issue #2054 layout has no such NODE_PATH.
  delete childEnv.NODE_PATH;
  try {
    const stdout = execFileSync(process.execPath, [path.join(root, 'project/main.cjs')], {
      env: childEnv,
      encoding: 'utf-8',
      timeout: 30_000,
    });
    return { ok: true as const, stdout };
  } catch (error) {
    return { ok: false as const, stderr: String((error as { stderr?: string }).stderr ?? error) };
  }
}

describe('binding resolution under the pnpm global virtual store layout', () => {
  // Shared by the resolution and enforcement-pass tests below.
  const matchingRoot = buildGlobalVirtualStoreLayout({
    loader: rewrittenLoader,
    platformPackageVersion: CORE_VERSION,
  });

  it('reproduces #2054: the collapsed vite-plus/binding rewrite cannot resolve', () => {
    const root = buildGlobalVirtualStoreLayout({ loader: collapsedLoader });
    expect(runProject(root)).toMatchObject({
      ok: false,
      stderr: expect.stringContaining("Cannot find module 'vite-plus/binding'"),
    });
  });

  it('resolves through the declared platform package after the rewrite', () => {
    expect(runProject(matchingRoot)).toEqual({ ok: true, stdout: 'native\n' });
  });

  it('re-arms the version guard: enforcement rejects a mismatched platform package', () => {
    const root = buildGlobalVirtualStoreLayout({
      loader: rewrittenLoader,
      platformPackageVersion: '0.0.0-stale',
    });
    expect(runProject(root, { NAPI_RS_ENFORCE_VERSION_CHECK: '1' })).toMatchObject({
      ok: false,
      stderr: expect.stringContaining(
        `Native binding package version mismatch, expected ${CORE_VERSION} but got 0.0.0-stale`,
      ),
    });
    // Matching versions pass with enforcement on.
    expect(runProject(matchingRoot, { NAPI_RS_ENFORCE_VERSION_CHECK: '1' })).toEqual({
      ok: true,
      stdout: 'native\n',
    });
  });
});
