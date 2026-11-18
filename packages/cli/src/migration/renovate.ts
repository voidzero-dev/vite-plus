import fs from 'node:fs/promises';
import path from 'node:path';

import {
  findNodeAtLocation,
  getNodeValue,
  parseTree,
  type Node,
  type ParseError,
} from 'jsonc-parser';

import { appendMissingJsoncArrayItems, detectFormattingOptions } from '../utils/json.ts';

const PRESET = 'group:vitePlus';

// Renovate uses the first repository config in this order.
// https://docs.renovatebot.com/configuration-options/#locations-for-configuration-filenames
export const STANDALONE_RENOVATE_CONFIG_FILES = [
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
];
const CONFIG_FILES = [...STANDALONE_RENOVATE_CONFIG_FILES, 'package.json'];

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
  const errors: ParseError[] = [];
  const tree = parseTree(source, errors, {
    allowTrailingComma: !embedded,
    disallowComments: embedded,
  });
  if (!tree || errors.length || hasDuplicateKeys(tree)) {
    return undefined;
  }
  const config = embedded ? findNodeAtLocation(tree, ['renovate']) : tree;
  if (config?.type !== 'object') {
    return undefined;
  }
  const extendsNode = findNodeAtLocation(config, ['extends']);
  const presets: unknown = extendsNode && getNodeValue(extendsNode);
  if (
    presets !== undefined &&
    (!Array.isArray(presets) ||
      !presets.every((preset) => typeof preset === 'string') ||
      presets.includes(PRESET))
  ) {
    return undefined;
  }
  // Keep a compact config or preset list on one line.
  const target = extendsNode ?? config;
  const multiline = source.slice(target.offset, target.offset + target.length).includes('\n');
  return appendMissingJsoncArrayItems(
    source,
    embedded ? ['renovate', 'extends'] : ['extends'],
    presets,
    [PRESET],
    multiline ? detectFormattingOptions(source) : undefined,
  );
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
