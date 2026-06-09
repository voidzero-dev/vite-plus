import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { hasConfigKey, mergeJsonConfig } from '../../binding/index.js';
import { resolveViteConfig } from '../resolve-vite-config.ts';
import { VITE_PLUS_NAME } from '../utils/constants.ts';
import { displayRelative } from '../utils/path.ts';
import type { CreateTemplateEntry } from './org-manifest.ts';
import { validateCreateTemplates } from './org-manifest.ts';

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
 * This is a read-modify-write helper. The NAPI `mergeJsonConfig` can only
 * *prepend* a key (it duplicates a key that already exists rather than
 * deep-merging), so the existing `create` object is read in full first and
 * the complete, recomputed object is written back. That keeps
 * `defaultTemplate` and prior `templates` intact regardless of which write
 * path is taken.
 */
export async function registerLocalTemplate(
  workspaceRoot: string,
  entry: CreateTemplateEntry,
  silent = false,
): Promise<void> {
  const configFile = findViteConfig(workspaceRoot);
  const configPath = configFile ? path.join(workspaceRoot, configFile) : undefined;

  // Read the current create config so we can recompute the full object.
  // Resolve the given workspace root directly (do not walk up like
  // getConfiguredCreate does): the caller passes the exact monorepo root.
  const existing = await readCreateConfig(workspaceRoot, configPath);

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

  if (hasConfigKey(targetPath, 'create')) {
    // `mergeJsonConfig` would prepend a duplicate `create:` key here, so
    // replace the existing `create: { ... }` value in place instead.
    replaceCreateBlock(targetPath, nextCreate);
  } else {
    // No `create` key yet (fresh or pre-existing config without one):
    // `mergeJsonConfig` cleanly prepends the full object.
    mergeCreateBlock(targetPath, nextCreate);
  }
}

function findViteConfig(workspaceRoot: string): string | undefined {
  return VITE_CONFIG_FILES.find((name) => fs.existsSync(path.join(workspaceRoot, name)));
}

/**
 * Read `create.defaultTemplate` and `create.templates` from `workspaceRoot`'s
 * config. Best-effort: an unresolvable or absent config reads as empty. A
 * present-but-malformed `create.templates` still throws (via
 * `validateCreateTemplates`) so misconfiguration is not silently dropped.
 */
async function readCreateConfig(
  workspaceRoot: string,
  configPath: string | undefined,
): Promise<{ defaultTemplate?: string; templates: CreateTemplateEntry[] }> {
  if (!configPath) {
    return { templates: [] };
  }
  let create: { defaultTemplate?: unknown; templates?: unknown } | undefined;
  try {
    const config = (await resolveViteConfig(workspaceRoot)) as {
      create?: { defaultTemplate?: unknown; templates?: unknown };
    };
    create = config.create;
  } catch {
    return { templates: [] };
  }
  const defaultTemplate =
    typeof create?.defaultTemplate === 'string' && create.defaultTemplate.length > 0
      ? create.defaultTemplate
      : undefined;
  const templates = validateCreateTemplates(create?.templates);
  return { ...(defaultTemplate !== undefined ? { defaultTemplate } : {}), templates };
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
 * Prepend a complete `create: { ... }` block via the NAPI merger. Only
 * valid when the config has no existing `create` key (the merger would
 * otherwise emit a duplicate).
 */
function mergeCreateBlock(configPath: string, create: object): void {
  const tempPath = path.join(path.dirname(configPath), '.vite-plus-create-register.json');
  fs.writeFileSync(tempPath, JSON.stringify(create));
  let result;
  try {
    result = mergeJsonConfig(configPath, tempPath, 'create');
  } finally {
    fs.rmSync(tempPath, { force: true });
  }
  if (result.updated) {
    fs.writeFileSync(configPath, result.content);
  }
}

/**
 * Replace the value of an existing top-level `create:` property with a
 * freshly serialized object literal, preserving everything else in the
 * file.
 *
 * The NAPI surface cannot replace an existing key (it only prepends), so
 * this does a narrow, deterministic text edit: locate the `create:` key,
 * find the `{` that opens its object value, scan to the matching `}`
 * (tracking nested braces/brackets and skipping string/template/comment
 * spans), and swap that span. This is not general TS parsing; it relies
 * only on the value being an object literal, which is the only shape this
 * module ever writes.
 */
function replaceCreateBlock(
  configPath: string,
  create: { defaultTemplate?: string; templates: CreateTemplateEntry[] },
): void {
  const content = fs.readFileSync(configPath, 'utf8');
  const span = findCreateObjectSpan(content);
  if (!span) {
    // Shape we do not recognize (e.g. `create:` referencing a variable).
    // Fall back to the merge path rather than risk a bad edit; this may
    // duplicate the key, but only happens for configs this module never
    // produces.
    mergeCreateBlock(configPath, create);
    return;
  }
  const indent = leadingIndent(content, span.keyStart);
  const literal = serializeCreateValue(create, indent);
  const next = content.slice(0, span.valueStart) + literal + content.slice(span.valueEnd);
  fs.writeFileSync(configPath, next);
}

interface CreateSpan {
  /** Index of the `c` in the `create` key. */
  keyStart: number;
  /** Index of the opening `{` of the value. */
  valueStart: number;
  /** Index just past the closing `}` of the value. */
  valueEnd: number;
}

/**
 * Find the top-level `create:` property whose value is an object literal.
 * Returns `undefined` for any other shape.
 */
function findCreateObjectSpan(content: string): CreateSpan | undefined {
  const keyRe = /(^|[\s,{])create\s*:\s*\{/g;
  let match: RegExpExecArray | null;
  while ((match = keyRe.exec(content)) !== null) {
    // `keyStart` points at the `c` of `create`; `match[1]` is the boundary
    // char captured before it.
    const keyStart = match.index + match[1].length;
    // The matched `{` is the last char of the match.
    const valueStart = match.index + match[0].length - 1;
    const valueEnd = matchClosingBrace(content, valueStart);
    if (valueEnd !== undefined) {
      return { keyStart, valueStart, valueEnd };
    }
  }
  return undefined;
}

/**
 * Given the index of an opening `{`, return the index just past its
 * matching `}`, skipping nested braces/brackets and string, template, and
 * comment spans. Returns `undefined` if unbalanced.
 */
function matchClosingBrace(content: string, openIndex: number): number | undefined {
  let depth = 0;
  for (let i = openIndex; i < content.length; i += 1) {
    const ch = content[i];
    if (ch === '"' || ch === "'" || ch === '`') {
      i = skipString(content, i, ch);
      continue;
    }
    if (ch === '/' && content[i + 1] === '/') {
      i = content.indexOf('\n', i);
      if (i === -1) {
        return undefined;
      }
      continue;
    }
    if (ch === '/' && content[i + 1] === '*') {
      const end = content.indexOf('*/', i + 2);
      if (end === -1) {
        return undefined;
      }
      i = end + 1;
      continue;
    }
    if (ch === '{' || ch === '[') {
      depth += 1;
    } else if (ch === '}' || ch === ']') {
      depth -= 1;
      if (depth === 0) {
        return i + 1;
      }
    }
  }
  return undefined;
}

/**
 * Skip a string/template literal starting at the opening quote `quote` at
 * `start`. Returns the index of the closing quote. Template literals can
 * contain `${...}` interpolations; their braces are skipped recursively.
 */
function skipString(content: string, start: number, quote: string): number {
  for (let i = start + 1; i < content.length; i += 1) {
    const ch = content[i];
    if (ch === '\\') {
      i += 1;
      continue;
    }
    if (quote === '`' && ch === '$' && content[i + 1] === '{') {
      const end = matchClosingBrace(content, i + 1);
      if (end === undefined) {
        return content.length;
      }
      i = end - 1;
      continue;
    }
    if (ch === quote) {
      return i;
    }
  }
  return content.length;
}

/** Leading whitespace on the line containing `index`. */
function leadingIndent(content: string, index: number): string {
  const lineStart = content.lastIndexOf('\n', index - 1) + 1;
  const line = content.slice(lineStart, index);
  const ws = /^[ \t]*/.exec(line);
  return ws ? ws[0] : '';
}

/**
 * Serialize the `create` value as a TS object literal indented to sit
 * under a `create:` key at `baseIndent`. JSON is valid TS for this shape
 * (string/array/object literals only), so `JSON.stringify` output is
 * re-indented and returned verbatim.
 */
function serializeCreateValue(create: object, baseIndent: string): string {
  const json = JSON.stringify(create, null, 2);
  return json
    .split('\n')
    .map((line, i) => (i === 0 ? line : baseIndent + line))
    .join('\n');
}
