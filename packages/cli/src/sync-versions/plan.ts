import path from 'node:path';

import { applyEdits, modify, parse as parseJsonc, type ParseError } from 'jsonc-parser';
import semver from 'semver';
import { isScalar, parseDocument, Scalar } from 'yaml';
import { z } from 'zod';

import { detectFormattingOptions } from '../utils/json.ts';
import { extractOverrideTargetName } from '../utils/package-overrides.ts';
import { isAlignableVitestEcosystemPackage } from '../utils/vitest-ecosystem.ts';

const MAX_MANIFESTS = 256;
const MAX_MANIFEST_BYTES = 1024 * 1024;

const manifestSnapshotSchema = z.discriminatedUnion('kind', [
  z
    .object({
      path: z.string().min(1),
      kind: z.literal('packageJson'),
      contents: z.string().max(MAX_MANIFEST_BYTES),
    })
    .strict(),
  z
    .object({
      path: z.string().min(1),
      kind: z.literal('pnpmWorkspace'),
      contents: z.string().max(MAX_MANIFEST_BYTES),
    })
    .strict(),
  z
    .object({
      path: z.string().min(1),
      kind: z.literal('yarnRc'),
      contents: z.string().max(MAX_MANIFEST_BYTES),
    })
    .strict(),
]);

const syncVersionsRequestSchema = z
  .object({
    schemaVersion: z.literal(1),
    workspace: z.literal('.'),
    manifests: z.array(manifestSnapshotSchema).max(MAX_MANIFESTS),
  })
  .strict()
  .superRefine((request, context) => {
    const paths = new Set<string>();
    for (const [index, manifest] of request.manifests.entries()) {
      const segments = manifest.path.split('/');
      const invalidPath =
        path.posix.isAbsolute(manifest.path) ||
        manifest.path.includes('\\') ||
        segments.some((segment) => segment === '' || segment === '.' || segment === '..');
      if (invalidPath) {
        context.addIssue({
          code: 'custom',
          path: ['manifests', index, 'path'],
          message: 'Manifest paths must be normalized workspace-relative POSIX paths',
        });
      }

      const basename = path.posix.basename(manifest.path);
      const kindMatchesPath =
        (manifest.kind === 'packageJson' && basename === 'package.json') ||
        (manifest.kind === 'pnpmWorkspace' && basename === 'pnpm-workspace.yaml') ||
        (manifest.kind === 'yarnRc' && basename === '.yarnrc.yml');
      if (!kindMatchesPath) {
        context.addIssue({
          code: 'custom',
          path: ['manifests', index, 'kind'],
          message: 'Manifest kind does not match its file name',
        });
      }

      if (paths.has(manifest.path)) {
        context.addIssue({
          code: 'custom',
          path: ['manifests', index, 'path'],
          message: 'Manifest paths must be unique',
        });
      }
      paths.add(manifest.path);
    }
  });

export type SyncVersionsRequestV1 = z.infer<typeof syncVersionsRequestSchema>;
export type SyncVersionsManifestSnapshot = SyncVersionsRequestV1['manifests'][number];

export interface SyncVersionsToolchain {
  vitePlus: string;
  vitest: string;
}

export interface SyncVersionsReplacementV1 {
  path: string;
  kind: SyncVersionsManifestSnapshot['kind'];
  before: string;
  after: string;
}

export interface SyncVersionsPlanV1 {
  schemaVersion: 1;
  tool: {
    name: 'vite-plus';
    version: string;
  };
  workspace: '.';
  replacements: SyncVersionsReplacementV1[];
}

interface TextEdit {
  start: number;
  end: number;
  value: string;
}

const INSTALL_DEPENDENCY_FIELDS = [
  'dependencies',
  'devDependencies',
  'optionalDependencies',
] as const;

export function parseSyncVersionsRequest(input: unknown): SyncVersionsRequestV1 {
  return syncVersionsRequestSchema.parse(input);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function parsePackageJson(contents: string): Record<string, unknown> {
  const errors: ParseError[] = [];
  const parsed: unknown = parseJsonc(contents, errors, {
    allowTrailingComma: true,
    disallowComments: false,
  });
  if (errors.length > 0 || !isRecord(parsed)) {
    throw new Error('Invalid package.json manifest');
  }
  return parsed;
}

function targetVersion(name: string, toolchain: SyncVersionsToolchain): string | undefined {
  if (name === 'vite-plus' || name === '@voidzero-dev/vite-plus-core') {
    return toolchain.vitePlus;
  }
  if (name === 'vitest' || isAlignableVitestEcosystemPackage(name)) {
    return toolchain.vitest;
  }
  return undefined;
}

function alignedNpmAlias(current: string, toolchain: SyncVersionsToolchain): string {
  const prefix = 'npm:';
  const versionAt = current.lastIndexOf('@');
  if (!current.startsWith(prefix) || versionAt <= prefix.length) {
    return current;
  }

  const packageName = current.slice(prefix.length, versionAt);
  const currentVersion = current.slice(versionAt + 1);
  const target = targetVersion(packageName, toolchain);
  if (
    target === undefined ||
    currentVersion === target ||
    semver.validRange(currentVersion) === null
  ) {
    return current;
  }
  return `${current.slice(0, versionAt + 1)}${target}`;
}

function alignedSpec(name: string, current: string, toolchain: SyncVersionsToolchain): string {
  if (current.startsWith('npm:')) {
    return alignedNpmAlias(current, toolchain);
  }
  const target = targetVersion(name, toolchain);
  if (
    target === undefined ||
    current === target ||
    current.startsWith('$') ||
    /^[a-z][a-z+.-]*:/i.test(current) ||
    semver.validRange(current) === null
  ) {
    return current;
  }
  return target;
}

function replaceJsonValue(contents: string, location: readonly string[], value: string): string {
  const edits = modify(contents, [...location], value, {
    formattingOptions: detectFormattingOptions(contents),
  });
  return applyEdits(contents, edits);
}

function alignJsonStringMap(
  contents: string,
  source: Record<string, unknown>,
  location: readonly string[],
  toolchain: SyncVersionsToolchain,
  packageNameForKey: (key: string) => string = (key) => key,
): string {
  let output = contents;
  for (const [name, value] of Object.entries(source)) {
    if (typeof value !== 'string') {
      continue;
    }
    const aligned = alignedSpec(packageNameForKey(name), value, toolchain);
    if (aligned !== value) {
      output = replaceJsonValue(output, [...location, name], aligned);
    }
  }
  return output;
}

function alignJsonOverrideMap(
  contents: string,
  source: Record<string, unknown>,
  location: readonly string[],
  toolchain: SyncVersionsToolchain,
  parentPackageName?: string,
): string {
  let output = contents;
  for (const [selector, value] of Object.entries(source)) {
    const packageName = selector === '.' ? parentPackageName : extractOverrideTargetName(selector);
    if (typeof value === 'string' && packageName) {
      const aligned = alignedSpec(packageName, value, toolchain);
      if (aligned !== value) {
        output = replaceJsonValue(output, [...location, selector], aligned);
      }
    } else if (isRecord(value)) {
      output = alignJsonOverrideMap(output, value, [...location, selector], toolchain, packageName);
    }
  }
  return output;
}

function alignJsonCatalogs(
  contents: string,
  source: Record<string, unknown>,
  location: readonly string[],
  toolchain: SyncVersionsToolchain,
): string {
  let output = contents;
  for (const [catalogName, entries] of Object.entries(source)) {
    if (isRecord(entries)) {
      output = alignJsonStringMap(output, entries, [...location, catalogName], toolchain);
    }
  }
  return output;
}

function planPackageJson(contents: string, toolchain: SyncVersionsToolchain): string {
  const pkg = parsePackageJson(contents);
  let output = contents;

  for (const field of INSTALL_DEPENDENCY_FIELDS) {
    const dependencies = pkg[field];
    if (isRecord(dependencies)) {
      output = alignJsonStringMap(output, dependencies, [field], toolchain);
    }
  }

  const overrides = pkg.overrides;
  if (isRecord(overrides)) {
    output = alignJsonOverrideMap(output, overrides, ['overrides'], toolchain);
  }

  const resolutions = pkg.resolutions;
  if (isRecord(resolutions)) {
    output = alignJsonStringMap(
      output,
      resolutions,
      ['resolutions'],
      toolchain,
      extractOverrideTargetName,
    );
  }

  const pnpm = pkg.pnpm;
  if (isRecord(pnpm) && isRecord(pnpm.overrides)) {
    output = alignJsonStringMap(
      output,
      pnpm.overrides,
      ['pnpm', 'overrides'],
      toolchain,
      extractOverrideTargetName,
    );
  }

  const catalog = pkg.catalog;
  if (isRecord(catalog)) {
    output = alignJsonStringMap(output, catalog, ['catalog'], toolchain);
  }

  const catalogs = pkg.catalogs;
  if (isRecord(catalogs)) {
    output = alignJsonCatalogs(output, catalogs, ['catalogs'], toolchain);
  }

  const workspaces = pkg.workspaces;
  if (isRecord(workspaces)) {
    if (isRecord(workspaces.catalog)) {
      output = alignJsonStringMap(output, workspaces.catalog, ['workspaces', 'catalog'], toolchain);
    }
    if (isRecord(workspaces.catalogs)) {
      output = alignJsonCatalogs(
        output,
        workspaces.catalogs,
        ['workspaces', 'catalogs'],
        toolchain,
      );
    }
  }

  return output;
}

function alignYamlStringMap(
  document: ReturnType<typeof parseDocument>,
  source: unknown,
  location: readonly string[],
  toolchain: SyncVersionsToolchain,
  edits: TextEdit[],
  packageNameForKey: (key: string) => string = (key) => key,
): void {
  if (!isRecord(source)) {
    return;
  }
  for (const [name, value] of Object.entries(source)) {
    if (typeof value !== 'string') {
      continue;
    }
    const aligned = alignedSpec(packageNameForKey(name), value, toolchain);
    if (aligned === value) {
      continue;
    }
    const node = document.getIn([...location, name], true);
    if (!isScalar(node) || typeof node.value !== 'string') {
      throw new Error(`Expected a string scalar at ${[...location, name].join('.')}`);
    }
    const range = node.range;
    if (range === null || range === undefined) {
      throw new Error(`Expected a source range at ${[...location, name].join('.')}`);
    }
    let replacement: string;
    switch (node.type) {
      case Scalar.PLAIN:
        replacement = aligned;
        break;
      case Scalar.QUOTE_SINGLE:
        replacement = `'${aligned.replaceAll("'", "''")}'`;
        break;
      case Scalar.QUOTE_DOUBLE:
        replacement = JSON.stringify(aligned);
        break;
      default:
        throw new Error(`Unsupported YAML scalar style at ${[...location, name].join('.')}`);
    }
    edits.push({ start: range[0], end: range[1], value: replacement });
  }
}

function applyTextEdits(contents: string, edits: TextEdit[]): string {
  let output = contents;
  let previousStart = contents.length;
  for (const edit of edits.toSorted((left, right) => right.start - left.start)) {
    if (edit.start < 0 || edit.end < edit.start || edit.end > previousStart) {
      throw new Error('Overlapping or invalid YAML manifest edits');
    }
    output = `${output.slice(0, edit.start)}${edit.value}${output.slice(edit.end)}`;
    previousStart = edit.start;
  }
  return output;
}

function parseYamlManifest(
  contents: string,
  fileName: 'pnpm-workspace.yaml' | '.yarnrc.yml',
): {
  document: ReturnType<typeof parseDocument>;
  manifest: Record<string, unknown>;
} {
  const document = parseDocument(contents);
  if (document.errors.length > 0) {
    throw new Error(`Invalid ${fileName} manifest`);
  }
  const manifest: unknown = document.toJS();
  if (!isRecord(manifest)) {
    throw new Error(`Invalid ${fileName} manifest`);
  }
  return { document, manifest };
}

function alignYamlCatalogs(
  document: ReturnType<typeof parseDocument>,
  manifest: Record<string, unknown>,
  toolchain: SyncVersionsToolchain,
  edits: TextEdit[],
): void {
  alignYamlStringMap(document, manifest.catalog, ['catalog'], toolchain, edits);
  if (isRecord(manifest.catalogs)) {
    for (const [catalogName, entries] of Object.entries(manifest.catalogs)) {
      alignYamlStringMap(document, entries, ['catalogs', catalogName], toolchain, edits);
    }
  }
}

function planPnpmWorkspace(contents: string, toolchain: SyncVersionsToolchain): string {
  const { document, manifest } = parseYamlManifest(contents, 'pnpm-workspace.yaml');
  const edits: TextEdit[] = [];
  alignYamlCatalogs(document, manifest, toolchain, edits);
  alignYamlStringMap(
    document,
    manifest.overrides,
    ['overrides'],
    toolchain,
    edits,
    extractOverrideTargetName,
  );

  return applyTextEdits(contents, edits);
}

function planYarnRc(contents: string, toolchain: SyncVersionsToolchain): string {
  const { document, manifest } = parseYamlManifest(contents, '.yarnrc.yml');
  const edits: TextEdit[] = [];
  alignYamlCatalogs(document, manifest, toolchain, edits);
  return applyTextEdits(contents, edits);
}

function planManifest(
  manifest: SyncVersionsManifestSnapshot,
  toolchain: SyncVersionsToolchain,
): string {
  switch (manifest.kind) {
    case 'packageJson':
      return planPackageJson(manifest.contents, toolchain);
    case 'pnpmWorkspace':
      return planPnpmWorkspace(manifest.contents, toolchain);
    case 'yarnRc':
      return planYarnRc(manifest.contents, toolchain);
    default: {
      const exhaustive: never = manifest;
      return exhaustive;
    }
  }
}

export function planSyncVersions(
  request: SyncVersionsRequestV1,
  toolchain: SyncVersionsToolchain,
): SyncVersionsPlanV1 {
  const replacements: SyncVersionsReplacementV1[] = [];
  for (const manifest of request.manifests) {
    const after = planManifest(manifest, toolchain);
    if (after !== manifest.contents) {
      replacements.push({
        path: manifest.path,
        kind: manifest.kind,
        before: manifest.contents,
        after,
      });
    }
  }

  return {
    schemaVersion: 1,
    tool: { name: 'vite-plus', version: toolchain.vitePlus },
    workspace: '.',
    replacements,
  };
}
