import fs from 'node:fs/promises';
import path from 'node:path';

import jju from 'jju';
import { getNodeValue, parseTree, type Node, type ParseError } from 'jsonc-parser';

import { isPlainRecord } from './migrator/shared.ts';

const PRESET = 'group:vitePlus';
// Treat names like "constructor" as ordinary JSON properties.
const JSON5_OPTIONS = { null_prototype: true, reserved_keys: 'replace' } as const;

// Renovate uses the first repository config in this order.
// https://docs.renovatebot.com/configuration-options/#locations-for-configuration-filenames
const CONFIG_FILES = [
  'renovate.json',
  'renovate.jsonc',
  'renovate.json5',
  '.github/renovate.json',
  '.github/renovate.jsonc',
  '.github/renovate.json5',
  '.gitlab/renovate.json',
  '.gitlab/renovate.jsonc',
  '.gitlab/renovate.json5',
  '.renovaterc',
  '.renovaterc.json',
  '.renovaterc.jsonc',
  '.renovaterc.json5',
  'package.json',
];

function hasDuplicateKeys(node: Node): boolean {
  const children = node.children ?? [];
  if (node.type === 'object') {
    const keys = children.map((property) => property.children![0].value);
    if (keys.length !== new Set(keys).size) {
      return true;
    }
  }
  return children.some(hasDuplicateKeys);
}

function withVitePlusPreset(file: string, source: string): string | undefined {
  const embedded = path.basename(file) === 'package.json';
  try {
    let document: unknown;
    if (file.endsWith('.json5')) {
      document = jju.parse(source, { ...JSON5_OPTIONS });
    } else {
      const errors: ParseError[] = [];
      const tree = parseTree(source, errors, {
        allowTrailingComma: !embedded,
        disallowComments: embedded,
      });
      if (!tree || errors.length || hasDuplicateKeys(tree)) {
        return undefined;
      }
      document = getNodeValue(tree);
    }
    const config = embedded && isPlainRecord(document) ? document.renovate : document;
    if (!isPlainRecord(config)) {
      return undefined;
    }
    const presets = config.extends === undefined ? [] : config.extends;
    if (
      !Array.isArray(presets) ||
      !presets.every((preset) => typeof preset === 'string') ||
      presets.includes(PRESET)
    ) {
      return undefined;
    }
    config.extends = [...presets, PRESET];
    // Edit only the changed property, preserving comments, indentation and JSON5 syntax.
    return jju.update(source, document, { ...JSON5_OPTIONS, mode: embedded ? 'json' : 'json5' });
  } catch {
    return undefined;
  }
}

export async function findRenovateConfigToUpdate(rootDir: string): Promise<string | undefined> {
  for (const name of CONFIG_FILES) {
    const file = path.join(rootDir, name);
    try {
      const source = await fs.readFile(file, 'utf8');
      return withVitePlusPreset(file, source) === undefined ? undefined : file;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
        return undefined;
      }
    }
  }
  return undefined;
}

export async function addVitePlusRenovatePreset(file: string): Promise<boolean> {
  // package.json may also have been edited by the migration since the prompt.
  let source: string;
  try {
    source = await fs.readFile(file, 'utf8');
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return false;
    }
    throw error;
  }
  const updated = withVitePlusPreset(file, source);
  if (updated === undefined) {
    return false;
  }
  await fs.writeFile(file, updated);
  return true;
}
