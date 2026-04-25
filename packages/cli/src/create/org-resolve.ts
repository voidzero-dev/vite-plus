import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { findWorkspaceRoot, hasViteConfig, resolveViteConfig } from '../resolve-vite-config.ts';
import {
  filterManifestForContext,
  isRelativePath,
  OrgManifestSchemaError,
  parseOrgScopedSpec,
  readOrgManifest,
  type OrgManifest,
  type OrgTemplateEntry,
} from './org-manifest.ts';
import {
  formatManifestTable,
  ORG_PICKER_BUILTIN_ESCAPE,
  ORG_PICKER_CANCEL,
  pickOrgTemplate,
} from './org-picker.ts';
import { ensureOrgPackageExtracted, resolveBundledPath } from './org-tarball.ts';
import { cancelAndExit } from './prompts.ts';

/**
 * Resolution outcome for an org template spec.
 *
 * - `passthrough`: no manifest applied; caller keeps the original spec.
 * - `replaced`: manifest entry resolves to a non-bundled specifier (npm,
 *   github, vite:*, local). Caller uses `templateName`.
 * - `bundled`: manifest entry uses a relative path; tarball has been
 *   extracted; caller passes `bundledLocalPath` into `discoverTemplate`.
 *   `scope` carries the resolved `@org` so a `monorepo: true` scaffold
 *   can wire `create.defaultTemplate` back to that same org. `monorepo`
 *   flows the caller into the monorepo scaffold path (parent-dir prompt
 *   + `rewriteMonorepo` integration).
 * - `escape-hatch`: user picked "Vite+ built-in templates" from the picker.
 */
export type OrgResolution =
  | { kind: 'passthrough' }
  | { kind: 'replaced'; templateName: string }
  | {
      kind: 'bundled';
      bundledLocalPath: string;
      entryName: string;
      scope: string;
      monorepo?: true;
    }
  | { kind: 'escape-hatch' };

function printNonInteractiveTable(
  manifest: OrgManifest,
  orgSpec: { scope: string },
  isMonorepo: boolean,
): void {
  const { lines, filteredCount } = formatManifestTable(manifest, isMonorepo);
  const [firstVisible] = filterManifestForContext(manifest.templates, isMonorepo);
  const body: string[] = [
    '',
    `A template name is required when running \`vp create ${orgSpec.scope}\` in non-interactive mode.`,
    '',
    `Available templates in ${manifest.packageName}:`,
    '',
    ...lines,
  ];
  if (filteredCount > 0) {
    body.push(
      '',
      `(omitted ${filteredCount} monorepo-only ${
        filteredCount === 1 ? 'entry' : 'entries'
      } because this workspace is already a monorepo)`,
    );
  }
  body.push('', 'Examples:');
  if (firstVisible) {
    body.push(
      '  # Scaffold a specific template from the org',
      `  vp create ${orgSpec.scope}:${firstVisible.name} --no-interactive`,
      '',
    );
  }
  body.push(
    '  # Or use a Vite+ built-in template',
    '  vp create vite:application --no-interactive',
    '',
  );
  process.stderr.write(body.join('\n'));
}

function rejectMonorepoEntryInsideMonorepo(entry: OrgTemplateEntry, isMonorepo: boolean): void {
  if (entry.monorepo && isMonorepo) {
    prompts.log.info(
      'You are already in a monorepo workspace.\nUse a different template or run this command outside the monorepo',
    );
    cancelAndExit('Cannot create a monorepo inside an existing monorepo', 1);
  }
}

async function resolveEntry(
  manifest: OrgManifest,
  entry: OrgTemplateEntry,
): Promise<OrgResolution> {
  if (isRelativePath(entry.template)) {
    const extracted = await ensureOrgPackageExtracted(manifest);
    const bundledLocalPath = resolveBundledPath(extracted, entry.template);
    return {
      kind: 'bundled',
      bundledLocalPath,
      entryName: entry.name,
      scope: manifest.scope,
      ...(entry.monorepo === true ? { monorepo: true as const } : {}),
    };
  }
  return { kind: 'replaced', templateName: entry.template };
}

/**
 * If `selectedTemplateName` points at an `@scope[/name]` org whose
 * `@scope/create` package publishes a `createConfig.templates` manifest, apply the
 * manifest rules (picker / direct lookup / escape hatch / bundled
 * extraction) and report the outcome.
 *
 * The caller — `packages/cli/src/create/bin.ts` — decides what to do next
 * based on the returned variant.
 */
export async function resolveOrgManifestForCreate(args: {
  templateName: string;
  isMonorepo: boolean;
  interactive: boolean;
}): Promise<OrgResolution> {
  const orgSpec = parseOrgScopedSpec(args.templateName);
  if (!orgSpec) {
    return { kind: 'passthrough' };
  }

  // Never silently skip the picker when the user explicitly typed `@org`.
  let manifest: OrgManifest | null;
  try {
    manifest = await readOrgManifest(orgSpec.scope, orgSpec.version);
  } catch (error) {
    const message =
      error instanceof OrgManifestSchemaError
        ? error.message
        : `Failed to read ${orgSpec.scope}/create manifest: ${(error as Error).message}`;
    cancelAndExit(message, 1);
  }

  if (!manifest) {
    if (orgSpec.name !== undefined) {
      // `@org:name` is an explicit manifest lookup; no manifest → hard error.
      cancelAndExit(
        `No \`createConfig.templates\` manifest in ${orgSpec.scope}/create — \`@org:name\` requires one.`,
        1,
      );
    }
    // Scope-only input (`vp create @org`) strongly implies the user
    // expected the picker. Be explicit about why it didn't engage, so a
    // later `ERR_NO_BIN` from the package manager doesn't look mysterious.
    prompts.log.info(
      `No \`createConfig.templates\` manifest in ${orgSpec.scope}/create — running it as a normal package.`,
    );
    return { kind: 'passthrough' };
  }

  if (orgSpec.name === undefined) {
    if (!args.interactive) {
      printNonInteractiveTable(manifest, orgSpec, args.isMonorepo);
      process.exit(1);
    }
    const picked = await pickOrgTemplate(manifest, { isMonorepo: args.isMonorepo });
    if (picked === ORG_PICKER_CANCEL) {
      cancelAndExit();
    }
    if (picked === ORG_PICKER_BUILTIN_ESCAPE) {
      // Only the in-monorepo filter can empty the list today; the message
      // stays in sync if more context-specific filters are added here.
      if (args.isMonorepo && manifest.templates.every((t) => t.monorepo)) {
        prompts.log.info(
          `No templates from ${manifest.packageName} are applicable inside a monorepo — showing Vite+ built-in templates instead.`,
        );
      }
      return { kind: 'escape-hatch' };
    }
    rejectMonorepoEntryInsideMonorepo(picked.entry, args.isMonorepo);
    return resolveEntry(manifest, picked.entry);
  }

  const entry = manifest.templates.find((candidate) => candidate.name === orgSpec.name);
  if (!entry) {
    // `@scope:name` is an explicit manifest lookup — no ambiguous fall-through.
    const available = filterManifestForContext(manifest.templates, args.isMonorepo)
      .map((t) => t.name)
      .join(', ');
    cancelAndExit(
      `No template named "${orgSpec.name}" in ${manifest.packageName}. Available: ${available || '(none applicable in this context)'}.`,
      1,
    );
  }
  rejectMonorepoEntryInsideMonorepo(entry, args.isMonorepo);
  return resolveEntry(manifest, entry);
}

/**
 * Read `create.defaultTemplate` from the workspace root's `vite.config.ts`.
 *
 * Walks up from `startDir` via `findWorkspaceRoot` (monorepo markers
 * only — `pnpm-workspace.yaml`, `workspaces` in `package.json`,
 * `lerna.json`) so monorepo invocations from any subdirectory still
 * pick up the root config. Standalone repos without a monorepo marker
 * only see a config that sits at `startDir` itself.
 *
 * Best-effort: if there's no config file or evaluation fails, return
 * `undefined` so the create flow behaves as if no default was set.
 */
export async function getConfiguredDefaultTemplate(startDir: string): Promise<string | undefined> {
  const projectRoot = findWorkspaceRoot(startDir) ?? startDir;
  if (!hasViteConfig(projectRoot)) {
    return undefined;
  }
  try {
    const config = (await resolveViteConfig(projectRoot)) as {
      create?: { defaultTemplate?: unknown };
    };
    const value = config.create?.defaultTemplate;
    if (typeof value === 'string' && value.length > 0) {
      return value;
    }
  } catch {
    // Unresolvable config → treat as no default.
  }
  return undefined;
}
