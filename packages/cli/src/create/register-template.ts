import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { hasConfigKey, mergeJsonConfig, replaceJsonConfig } from '../../binding/index.js';
import { VITE_PLUS_NAME } from '../utils/constants.ts';
import { displayRelative } from '../utils/path.ts';
import type { CreateTemplateEntry } from './org-manifest.ts';
import { getConfiguredCreate } from './org-resolve.ts';

/**
 * Vite config filenames we know how to read and write `create.templates`
 * into, in resolution priority order. Mirrors the list in
 * `resolve-vite-config.ts` but trimmed to the extensions the create flow
 * emits and the migrator's `mergeJsonConfig` path understands.
 */
const VITE_CONFIG_FILES = ['vite.config.ts', 'vite.config.js', 'vite.config.mjs'] as const;

/**
 * Register a local template into `create.templates` in a monorepo's root
 * `vite.config.ts`. Used after `vp create vite:generator` scaffolds a
 * generator so the generated template shows up in this workspace's
 * `vp create` picker.
 *
 * Behavior:
 * - Reads the existing `create` config from `<workspaceRoot>/vite.config.{ts,js,mjs}`.
 * - If an entry with the same `name` already exists → no-op (idempotent).
 * - Otherwise appends `entry` to `create.templates`, preserving any sibling
 *   `create.defaultTemplate` and any existing entries, and writes back.
 * - If there is no `vite.config.*` yet, or no `create` block, it is created.
 *
 * Read-modify-write: the existing `create` object is read in full first and
 * the complete, recomputed object is written back via the shared config merger
 * (`replaceJsonConfig` when a `create` block exists, `mergeJsonConfig` to
 * prepend one otherwise), so `defaultTemplate` and prior `templates` are kept.
 */
export async function registerLocalTemplate(
  workspaceRoot: string,
  entry: CreateTemplateEntry,
  silent = false,
): Promise<void> {
  const configFile = findViteConfig(workspaceRoot);
  const configPath = configFile ? path.join(workspaceRoot, configFile) : undefined;

  // Read the current create config so we can recompute the full object.
  // `walkUp: false`: the caller passes the exact monorepo root, so read it
  // directly rather than searching for an enclosing workspace.
  const existing = await getConfiguredCreate(workspaceRoot, { walkUp: false });

  // Idempotent: an entry with the same name is left untouched.
  if (existing.templates.some((t) => t.name === entry.name)) {
    return;
  }

  const nextCreate: { defaultTemplate?: string; templates: CreateTemplateEntry[] } = {
    ...(existing.defaultTemplate !== undefined
      ? { defaultTemplate: existing.defaultTemplate }
      : {}),
    templates: [...existing.templates, entry],
  };

  const targetPath = configPath ?? ensureViteConfig(workspaceRoot, silent);
  writeCreateBlock(targetPath, nextCreate);
}

function findViteConfig(workspaceRoot: string): string | undefined {
  return VITE_CONFIG_FILES.find((name) => fs.existsSync(path.join(workspaceRoot, name)));
}

/**
 * Create a minimal `vite.config.ts` (matching the migrator's
 * `ensureViteConfig` shape) and return its absolute path.
 */
function ensureViteConfig(workspaceRoot: string, silent: boolean): string {
  const configPath = path.join(workspaceRoot, 'vite.config.ts');
  fs.writeFileSync(
    configPath,
    `import { defineConfig } from '${VITE_PLUS_NAME}';\n\nexport default defineConfig({});\n`,
  );
  if (!silent) {
    prompts.log.success(`✔ Created vite.config.ts in ${displayRelative(configPath)}`);
  }
  return configPath;
}

/**
 * Write the full `create` object into vite.config.ts via the shared config
 * merger: replace the existing `create:` value in place when the key is
 * present, otherwise prepend the key. The caller reformats the file afterward,
 * so the JSON-style block written here is normalized to the surrounding style.
 */
function writeCreateBlock(configPath: string, create: object): void {
  const tempPath = path.join(path.dirname(configPath), '.vite-plus-create-register.json');
  fs.writeFileSync(tempPath, JSON.stringify(create));
  try {
    const result = hasConfigKey(configPath, 'create')
      ? replaceJsonConfig(configPath, tempPath, 'create')
      : mergeJsonConfig(configPath, tempPath, 'create');
    if (result.updated) {
      fs.writeFileSync(configPath, result.content);
    }
  } finally {
    fs.rmSync(tempPath, { force: true });
  }
}
