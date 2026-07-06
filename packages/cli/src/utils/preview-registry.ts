import fs from 'node:fs';
import path from 'node:path';

import { Scalar } from 'yaml';

import { PackageManager } from '../types/index.ts';
import { VITE_PLUS_VERSION } from './constants.ts';
import { readJsonFile } from './json.ts';
import { editYamlFile } from './yaml.ts';

const DEFAULT_BRIDGE_REGISTRY = 'https://registry-bridge.viteplus.dev/';
const REGISTRY_MARKER = '# vite-plus preview build registry bridge (auto-added by vp)';
// Host used to recognize a bridge registry that we (or the harness) wrote, so a
// non-preview run can clean it up. Matches the default and any VP_REGISTRY_BRIDGE
// pointed at the same host.
const BRIDGE_HOST = 'registry-bridge.viteplus.dev';

/**
 * Registry bridge that serves PR preview builds as ordinary `0.0.0-commit.<sha>`
 * versions (and proxies everything else to npmjs). Only ever used for preview
 * builds (see {@link isPreviewVitePlusVersion}); real releases never resolve
 * through it. Overridable via `VP_REGISTRY_BRIDGE` for testing or an alternate
 * bridge host; read at call time so the override applies per process.
 */
function bridgeRegistry(): string {
  return process.env.VP_REGISTRY_BRIDGE || DEFAULT_BRIDGE_REGISTRY;
}

/**
 * A preview / test build is published as `0.0.0-commit.<sha>` (and, generally,
 * any `0.0.0-<prerelease>`). A real release is never `0.0.0`, so this reliably
 * flags a build under test with no false positives on real-user migrations.
 */
export function isPreviewVitePlusVersion(
  version: string = process.env.VP_VERSION || VITE_PLUS_VERSION,
): boolean {
  return version.startsWith('0.0.0-');
}

/**
 * Berry reads `.yarnrc.yml` and ignores `.npmrc`, so the registry must be
 * written to the right file. Detect Berry without an install: a `.yarnrc.yml`
 * is Berry-only, a Berry lockfile carries a `__metadata:` block, and a
 * `packageManager: "yarn@>=2"` pin selects Berry up front.
 */
function isYarnBerryProject(projectRoot: string): boolean {
  if (fs.existsSync(path.join(projectRoot, '.yarnrc.yml'))) {
    return true;
  }
  const yarnLock = path.join(projectRoot, 'yarn.lock');
  if (fs.existsSync(yarnLock)) {
    try {
      if (fs.readFileSync(yarnLock, 'utf8').includes('__metadata:')) {
        return true;
      }
    } catch {
      // unreadable lockfile: fall through to the manifest check
    }
  }
  try {
    const pkg = readJsonFile(path.join(projectRoot, 'package.json'));
    const pm = typeof pkg.packageManager === 'string' ? pkg.packageManager : '';
    const major = /^yarn@(\d+)/.exec(pm)?.[1];
    if (major && Number(major) >= 2) {
      return true;
    }
  } catch {
    // no/invalid package.json: treat as not-Berry
  }
  return false;
}

function ensureNpmrcRegistry(projectRoot: string): void {
  const npmrc = path.join(projectRoot, '.npmrc');
  const bridge = bridgeRegistry();
  let content = '';
  if (fs.existsSync(npmrc)) {
    content = fs.readFileSync(npmrc, 'utf8');
    if (content.includes(REGISTRY_MARKER) || content.includes(bridge)) {
      return; // already pointed at the bridge
    }
  }
  const prefix = content.length > 0 && !content.endsWith('\n') ? '\n' : '';
  fs.appendFileSync(npmrc, `${prefix}${REGISTRY_MARKER}\nregistry=${bridge}\n`);
}

// Comment attached to the bridge `npmRegistryServer` value so a later
// real-release run can restore the user's original registry instead of
// deleting it. Comments survive the YAML round-trip.
const YARN_ORIGINAL_REGISTRY_COMMENT_PREFIX =
  ' vite-plus preview bridge (auto-added by vp); original npmRegistryServer: ';

function ensureYarnBerryRegistry(projectRoot: string): void {
  const yarnrc = path.join(projectRoot, '.yarnrc.yml');
  if (!fs.existsSync(yarnrc)) {
    fs.writeFileSync(yarnrc, '');
  }
  editYamlFile(yarnrc, (doc) => {
    const current = doc.get('npmRegistryServer');
    if (current === bridgeRegistry()) {
      return; // already pointed at the bridge; keep any stashed original
    }
    doc.set('npmRegistryServer', bridgeRegistry());
    const node = doc.get('npmRegistryServer', true);
    if (node instanceof Scalar && typeof current === 'string' && !current.includes(BRIDGE_HOST)) {
      // Overwriting a custom registry (e.g. a corporate proxy) must not lose
      // it for good: stash it in a comment so clear/restore can put it back.
      node.comment = `${YARN_ORIGINAL_REGISTRY_COMMENT_PREFIX}${current}`;
    }
  });
}

function clearNpmrcRegistry(projectRoot: string): boolean {
  const npmrc = path.join(projectRoot, '.npmrc');
  if (!fs.existsSync(npmrc)) {
    return false;
  }
  const original = fs.readFileSync(npmrc, 'utf8');
  if (!original.includes(REGISTRY_MARKER)) {
    return false; // nothing we added
  }
  const lines = original.split('\n');
  const kept: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim() === REGISTRY_MARKER) {
      // Drop the marker and the `registry=` line we wrote right after it.
      if (lines[i + 1]?.startsWith('registry=')) {
        i++;
      }
      continue;
    }
    kept.push(lines[i]);
  }
  const result = kept.join('\n').replace(/\n{2,}$/, '\n');
  if (result.trim() === '') {
    fs.rmSync(npmrc);
  } else {
    fs.writeFileSync(npmrc, result.endsWith('\n') ? result : `${result}\n`);
  }
  return true;
}

function clearYarnBerryRegistry(projectRoot: string): boolean {
  const yarnrc = path.join(projectRoot, '.yarnrc.yml');
  if (!fs.existsSync(yarnrc)) {
    return false;
  }
  const content = fs.readFileSync(yarnrc, 'utf8');
  if (!content.includes(BRIDGE_HOST) && !content.includes(bridgeRegistry())) {
    return false; // npmRegistryServer is not pointed at the bridge
  }
  let cleared = false;
  editYamlFile(yarnrc, (doc) => {
    const current = doc.get('npmRegistryServer');
    if (
      typeof current === 'string' &&
      (current.includes(BRIDGE_HOST) || current === bridgeRegistry())
    ) {
      // A prior preview run may have replaced a custom registry and stashed it
      // in the value's comment; restore it instead of deleting the setting.
      const node = doc.get('npmRegistryServer', true);
      const comment = node instanceof Scalar ? node.comment : undefined;
      const original = comment?.split(YARN_ORIGINAL_REGISTRY_COMMENT_PREFIX.trimStart())[1]?.trim();
      if (original) {
        doc.set('npmRegistryServer', original);
        const restored = doc.get('npmRegistryServer', true);
        if (restored instanceof Scalar) {
          restored.comment = undefined;
        }
      } else {
        doc.delete('npmRegistryServer');
      }
      cleared = true;
    }
  });
  return cleared;
}

/**
 * Reconcile the project's registry config with the running build:
 *
 * - Preview / test build: point the project at the registry bridge (`.npmrc`, or
 *   `.yarnrc.yml` for Yarn Berry) so the `0.0.0-commit.<sha>` versions that
 *   migrate/create just pinned resolve during this install and in the project's
 *   own CI.
 * - Real release: remove any bridge registry a PRIOR preview run left behind, so
 *   real installs resolve from npmjs instead of the test bridge.
 *
 * No-op and idempotent in the common case. Returns true when it changed config.
 */
export function reconcilePreviewBridgeRegistry(
  projectRoot: string,
  version: string = process.env.VP_VERSION || VITE_PLUS_VERSION,
  packageManager?: PackageManager,
): boolean {
  if (isPreviewVitePlusVersion(version)) {
    // Write the file the ACTIVE package manager reads: Yarn Berry uses
    // `.yarnrc.yml`, everything else uses `.npmrc`. Fall back to file-based
    // detection only when the manager is unknown, so a stray leftover
    // `.yarnrc.yml` in a pnpm/npm/bun project doesn't leave `.npmrc` without the
    // bridge registry (the install would then fail to resolve `0.0.0-commit.<sha>`).
    const useYarnBerry =
      packageManager === PackageManager.yarn
        ? isYarnBerryProject(projectRoot)
        : packageManager === undefined && isYarnBerryProject(projectRoot);
    if (useYarnBerry) {
      ensureYarnBerryRegistry(projectRoot);
    } else {
      ensureNpmrcRegistry(projectRoot);
    }
    return true;
  }
  // Real release: undo a previous preview run's bridge registry (check both
  // files in case the project switched package managers since).
  const clearedNpmrc = clearNpmrcRegistry(projectRoot);
  const clearedYarnrc = clearYarnBerryRegistry(projectRoot);
  return clearedNpmrc || clearedYarnrc;
}
