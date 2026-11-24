import fs from 'node:fs/promises';
import path from 'node:path';

import type { Node, ParseError } from 'jsonc-parser';
import { findNodeAtLocation, parseTree } from 'jsonc-parser';

import { appendMissingJsoncArrayItems, detectFormattingOptions } from '../utils/json.ts';
import { withJson5Preset } from './renovate-json5.ts';

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
    const keys = children.map((property) => {
      const key: unknown = property.children?.[0]?.value;
      return typeof key === 'string' ? key : undefined;
    });
    if (keys.length !== new Set(keys).size) {
      return true;
    }
  }
  return children.some(hasDuplicateKeys);
}

async function withVitePlusPreset(file: string, source: string): Promise<string | undefined> {
  if (file.endsWith('.json5')) {
    return withJson5Preset(source, PRESET);
  }
  const embedded = path.basename(file) === 'package.json';
  const errors: Array<ParseError> = [];
  const tree = parseTree(source, errors, {
    allowTrailingComma: !embedded,
    disallowComments: embedded,
  });
  if (!tree || errors.length || hasDuplicateKeys(tree)) {
    return undefined;
  }
  const configPath = embedded ? ['renovate'] : [];
  const config = findNodeAtLocation(tree, configPath);
  const extendsPath = [...configPath, 'extends'];
  // Renovate ignores an empty `renovate` field in package.json.
  const manifestShorthand = embedded && config?.type === 'string' && config.value !== '';
  const extendsNode = manifestShorthand ? config : findNodeAtLocation(tree, extendsPath);
  if (extendsNode?.type === 'string') {
    if (extendsNode.value === PRESET) {
      return undefined;
    }
    // Keep the original string literal, including its escape sequences.
    const { offset, length } = extendsNode;
    const end = offset + length;
    const presets = `[${source.slice(offset, end)}, "${PRESET}"]`;
    const replacement = manifestShorthand ? `{"extends": ${presets}}` : presets;
    return source.slice(0, offset) + replacement + source.slice(end);
  }
  if (config?.type !== 'object' || (extendsNode && extendsNode.type !== 'array')) {
    return undefined;
  }
  const presets = extendsNode?.children?.map((item) => {
    const value: unknown = item.value;
    return typeof value === 'string' ? value : undefined;
  });
  if (presets?.some((preset) => preset === undefined || preset === PRESET)) {
    return undefined;
  }
  // Keep a compact config or preset list on one line.
  const target = extendsNode ?? config;
  const multiline = source.slice(target.offset, target.offset + target.length).includes('\n');
  return appendMissingJsoncArrayItems(
    source,
    extendsPath,
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
      const updated = await withVitePlusPreset(file, source);
      return updated === undefined ? undefined : file;
    } catch (error) {
      if (
        !(typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT')
      ) {
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
    if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') {
      return false;
    }
    throw error;
  }
  const updated = await withVitePlusPreset(file, source);
  if (updated === undefined) {
    return false;
  }
  await fs.writeFile(file, updated);
  return true;
}
