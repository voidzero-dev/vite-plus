import path from 'node:path';

import type * as t from '@oxc-project/types';

import {
  SourceEditor,
  importedName,
  isString,
  memberName,
  objectProperty,
  staticObject,
} from './ast.ts';
import { resolveConfigObject } from './config-object.ts';

const STORYBOOK_PLUGIN = new Set(['@storybook/addon-vitest/vitest-plugin']);
const NODE_PATH = new Set(['node:path', 'path']);
const NODE_URL = new Set(['node:url', 'url']);

export interface StorybookScope {
  root?: string;
  include?: string[];
  reason?: string;
}

function importMeta(node: t.Node): boolean {
  return (
    node.type === 'MetaProperty' && node.meta.name === 'import' && node.property.name === 'meta'
  );
}

function pathFunction(editor: SourceEditor, node: t.Node): string | undefined {
  const name = importedName(editor, node, NODE_PATH);
  if (name) {
    return name;
  }
  if (node.type !== 'MemberExpression' || node.object.type !== 'Identifier') {
    return undefined;
  }
  const declaration = editor.binding(node.object)?.declaration;
  const parent = declaration && editor.parent(declaration);
  if (
    declaration?.type === 'ImportDefaultSpecifier' &&
    parent?.type === 'ImportDeclaration' &&
    NODE_PATH.has(parent.source.value)
  ) {
    return memberName(node);
  }
  return undefined;
}

/** Recognize common configDir expressions, without evaluating config code. */
function configPath(
  editor: SourceEditor,
  node: t.Node,
  file: string,
  cwd: string,
  seen = new Set<t.Node>(),
): string | undefined {
  if (seen.has(node)) {
    return undefined;
  }
  seen.add(node);
  if (isString(node)) {
    return node.value;
  }
  if (node.type === 'Identifier') {
    const binding = editor.binding(node);
    if (!binding && node.name === '__dirname') {
      return path.dirname(file);
    }
    if (!binding && node.name === '__filename') {
      return file;
    }
    if (
      binding?.constant &&
      binding.declaration.type === 'VariableDeclarator' &&
      binding.declaration.init
    ) {
      return configPath(editor, binding.declaration.init, file, cwd, seen);
    }
  }
  if (node.type === 'MemberExpression' && importMeta(node.object)) {
    if (memberName(node) === 'dirname') {
      return path.dirname(file);
    }
    if (memberName(node) === 'filename') {
      return file;
    }
  }
  if (node.type !== 'CallExpression' || node.optional) {
    return undefined;
  }
  const first = node.arguments[0];
  if (
    importedName(editor, node.callee, NODE_URL) === 'fileURLToPath' &&
    node.arguments.length === 1 &&
    first?.type === 'MemberExpression' &&
    importMeta(first.object) &&
    memberName(first) === 'url'
  ) {
    return file;
  }
  const method = pathFunction(editor, node.callee);
  if (!['join', 'resolve', 'dirname'].includes(method ?? '')) {
    return undefined;
  }
  const args = node.arguments.map((arg) => configPath(editor, arg, file, cwd, new Set(seen)));
  if (args.some((arg) => arg === undefined)) {
    return undefined;
  }
  const values = args as string[];
  if (method === 'dirname') {
    return values.length === 1 ? path.dirname(values[0]) : undefined;
  }
  return method === 'resolve' ? path.resolve(cwd, ...values) : path.join(...values);
}

/** The addon replaces test.include with Storybook stories, not Vitest's default
 * test/spec glob. Keep this adapter static; never load addons or run presets.
 * https://storybook.js.org/docs/writing-tests/integrations/vitest-addon */
export function resolveStorybookScope(
  editor: SourceEditor,
  object: t.ObjectExpression,
  sources: ReadonlyMap<string, string>,
  file: string,
  cwd: string,
): StorybookScope | undefined {
  const plugins = objectProperty(object, 'plugins')?.value;
  if (!plugins) {
    return undefined;
  }
  const calls: t.CallExpression[] = [];
  editor.visit({
    CallExpression(node) {
      if (
        node.start >= plugins.start &&
        node.end <= plugins.end &&
        importedName(editor, node.callee, STORYBOOK_PLUGIN) === 'storybookTest'
      ) {
        calls.push(node);
      }
    },
  });
  if (!calls.length) {
    return undefined;
  }
  const unknown = (detail: string): StorybookScope => ({
    reason: `Storybook test ownership is unresolved (${path.basename(file)}): ${detail}.`,
  });
  if (calls.length !== 1) {
    return unknown('multiple storybookTest calls');
  }
  const call = calls[0];
  let container: t.Node = call;
  while (container !== plugins) {
    const parent = editor.parent(container);
    if (parent?.type !== 'ArrayExpression' && parent?.type !== 'AwaitExpression') {
      return unknown('conditional or wrapped storybookTest call');
    }
    container = parent;
  }
  const options = call.arguments[0];
  if (call.optional || call.arguments.length > 1 || (options && !staticObject(options))) {
    return unknown('dynamic storybookTest options');
  }
  const configDir = options && objectProperty(options, 'configDir')?.value;
  const directory = configDir ? configPath(editor, configDir, file, cwd) : '.storybook';
  if (directory === undefined) {
    return unknown('dynamic configDir');
  }
  const resolvedDir = path.resolve(cwd, directory);
  // Multiple main files can have loader-specific precedence. Do not guess.
  const mains = ['js', 'jsx', 'ts', 'tsx', 'mjs', 'mts', 'cjs', 'cts']
    .map((extension) => path.join(resolvedDir, `main.${extension}`))
    .filter((candidate) => sources.has(candidate));
  const label = path.relative(cwd, resolvedDir).replaceAll('\\', '/');
  if (mains.length !== 1) {
    return unknown(`expected one readable main config in ${label}`);
  }
  if (
    ['js', 'ts', 'mjs', 'cjs'].some((extension) =>
      sources.has(path.join(resolvedDir, `presets.${extension}`)),
    )
  ) {
    return unknown(`custom presets in ${label}`);
  }
  const mainFile = mains[0];
  try {
    const mainEditor = new SourceEditor(mainFile, sources.get(mainFile)!);
    const exported = mainEditor.ast.body.find((node) => node.type === 'ExportDefaultDeclaration');
    const main = resolveConfigObject(mainEditor, exported?.declaration);
    if (!main || main.properties.some((property) => property.type === 'SpreadElement')) {
      return unknown(`dynamic main config in ${label}`);
    }
    if (
      ['presets', 'viteFinal', 'experimental_indexers'].some((key) => objectProperty(main, key))
    ) {
      return unknown(`custom discovery hooks in ${label}/main`);
    }
    const stories = objectProperty(main, 'stories')?.value;
    if (stories?.type !== 'ArrayExpression') {
      return unknown(`dynamic stories in ${label}/main`);
    }
    const include: string[] = [];
    for (const story of stories.elements) {
      let pattern: string;
      if (isString(story)) {
        pattern = story.value;
      } else if (staticObject(story)) {
        const directory = objectProperty(story, 'directory')?.value;
        const files = objectProperty(story, 'files')?.value;
        // Leave Storybook's version-dependent default files pattern unresolved.
        if (!isString(directory) || !isString(files)) {
          return unknown(`dynamic story directory/files in ${label}/main`);
        }
        pattern = path.join(directory.value, files.value);
      } else {
        return unknown(`dynamic stories in ${label}/main`);
      }
      if (pattern.startsWith('!')) {
        return unknown(`negated story pattern in ${label}/main`);
      }
      const target = path.resolve(resolvedDir, pattern);
      if (!/[?*{[(]/.test(pattern) && !sources.has(target)) {
        return unknown(`unresolved story file or directory in ${label}/main`);
      }
      include.push(target.replaceAll('\\', '/'));
    }
    return { root: path.resolve(resolvedDir, '..'), include };
  } catch {
    return unknown(`cannot parse ${label}/main`);
  }
}
