import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { upsertJsonConfig } from '../../binding/index.js';
import { findViteConfig } from '../resolve-vite-config.ts';
import { VITE_PLUS_NAME } from '../utils/constants.ts';
import { displayRelative } from '../utils/path.ts';
import type { CreateTemplateEntry } from './org-manifest.ts';
import { getConfiguredCreate } from './org-resolve.ts';

/**
 * Register a local template into `create.templates` in a monorepo's root
 * `vite.config.ts`. Used after `vp create vite:generator` scaffolds a
 * generator so the generated template shows up in this workspace's
 * `vp create` picker.
 *
 * Behavior:
 * - Reads the existing `create` config from the workspace root's `vite.config.*`.
 * - If an entry with the same `name` already exists → no-op (idempotent),
 *   warning when it points at a different `template` so a stale entry does
 *   not silently shadow the new generator.
 * - Otherwise appends `entry` to `create.templates`, preserving any sibling
 *   `create.defaultTemplate` and any existing entries, and writes back.
 * - If there is no `vite.config.*` yet, or no `create` block, it is created.
 *
 * Read-modify-write: the existing `create` object is read in full first and
 * the complete, recomputed object is written back via `upsertJsonConfig`
 * (replace the existing `create` value, or insert the key), so
 * `defaultTemplate` and prior `templates` are kept. Throws when the config
 * shape is not supported by the upsert, rather than writing nothing or a key
 * that is dead at runtime.
 *
 * Returns the absolute path of the config file written, so the caller can fold
 * it into its own formatting pass (the upsert writes a JSON-style block that
 * needs reformatting). Returns `undefined` for the idempotent no-op.
 */
export async function registerLocalTemplate(
  workspaceRoot: string,
  entry: CreateTemplateEntry,
  silent = false,
): Promise<string | undefined> {
  const configPath = findViteConfig(workspaceRoot);

  // Read the current create config so we can recompute the full object.
  // `walkUp: false`: the caller passes the exact monorepo root, so read it
  // directly rather than searching for an enclosing workspace.
  // `throwOnReadError`: if the config exists but cannot be evaluated, abort
  // rather than overwrite its `create` block with only the new entry.
  const existing = await getConfiguredCreate(workspaceRoot, {
    walkUp: false,
    throwOnReadError: true,
  });

  // Idempotent: an entry with the same name is left untouched.
  const existingEntry = existing.templates.find((t) => t.name === entry.name);
  if (existingEntry) {
    if (existingEntry.template !== entry.template) {
      prompts.log.warn(
        `create.templates already has an entry named '${entry.name}' pointing at '${existingEntry.template}'; left unchanged.\n` +
          `Update it by hand if it should run '${entry.template}' instead.`,
      );
    }
    return undefined;
  }

  const nextCreate: { defaultTemplate?: string; templates: CreateTemplateEntry[] } = {
    ...(existing.defaultTemplate !== undefined
      ? { defaultTemplate: existing.defaultTemplate }
      : {}),
    templates: [...existing.templates, entry],
  };

  const targetPath = configPath ?? ensureViteConfig(workspaceRoot, silent);
  writeCreateBlock(targetPath, nextCreate);
  return targetPath;
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
 * upsert: replace the existing `create:` value in place, or insert the key
 * when absent. The caller reformats the file afterward, so the JSON-style
 * block written here is normalized to the surrounding style.
 *
 * Throws when the config shape is not supported (`updated: false`, e.g.
 * `module.exports` or `export default someVar`), so the caller can warn and
 * point at a manual edit instead of reporting a registration that never
 * happened.
 */
function writeCreateBlock(configPath: string, create: object): void {
  // A unique OS-temp path: a fixed name in the workspace could collide with a
  // user's own file and be overwritten/deleted by the merge.
  const tempPath = path.join(os.tmpdir(), `vite-plus-create-register-${randomUUID()}.json`);
  fs.writeFileSync(tempPath, JSON.stringify(create));
  try {
    const result = upsertJsonConfig(configPath, tempPath, 'create');
    if (!result.updated) {
      throw new Error(`could not find a supported config object in ${displayRelative(configPath)}`);
    }
    fs.writeFileSync(configPath, result.content);
  } finally {
    fs.rmSync(tempPath, { force: true });
  }
}
