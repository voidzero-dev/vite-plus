import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { findViteConfigUp } from '../resolve-vite-config.ts';
import { errorMsg } from '../utils/terminal.ts';
import {
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
 * - `escape-hatch`: user picked "Vite+ built-in templates" from the picker.
 */
export type OrgResolution =
  | { kind: 'passthrough' }
  | { kind: 'replaced'; templateName: string; entry: OrgTemplateEntry }
  | { kind: 'bundled'; bundledLocalPath: string; entry: OrgTemplateEntry }
  | { kind: 'escape-hatch' };

function printNonInteractiveTable(
  manifest: OrgManifest,
  orgSpec: { scope: string },
  isMonorepo: boolean,
): void {
  const { lines, filteredCount } = formatManifestTable(manifest, isMonorepo);
  errorMsg(`vp create ${orgSpec.scope} requires a template selection in non-interactive mode.`);
  const stderrBody: string[] = [
    '',
    `available templates from ${manifest.packageName}:`,
    '',
    ...lines,
  ];
  if (filteredCount > 0) {
    stderrBody.push(
      '',
      `(omitted ${filteredCount} monorepo-only ${
        filteredCount === 1 ? 'entry' : 'entries'
      } because this workspace is already a monorepo)`,
    );
  }
  stderrBody.push('');
  const firstVisible = manifest.templates.find((t) => !(t.monorepo && isMonorepo));
  if (firstVisible) {
    stderrBody.push(
      `hint: rerun with an explicit selection, e.g. \`vp create ${orgSpec.scope}/${firstVisible.name}\`,`,
    );
  }
  stderrBody.push('      or use a Vite+ built-in template like `vp create vite:application`.');
  process.stderr.write(`${stderrBody.join('\n')}\n`);
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
  // Breadcrumb so a later downstream failure (e.g. the referenced
  // `@org/template-web` package is missing) still tells the user what
  // manifest entry produced that chain.
  prompts.log.info(`selected '${entry.name}' from ${manifest.packageName}`);
  if (isRelativePath(entry.template)) {
    const extracted = await ensureOrgPackageExtracted(manifest);
    const bundledLocalPath = resolveBundledPath(extracted, entry.template);
    return { kind: 'bundled', bundledLocalPath, entry };
  }
  return { kind: 'replaced', templateName: entry.template, entry };
}

/**
 * If `selectedTemplateName` points at an `@scope[/name]` org whose
 * `@scope/create` package publishes a `vp.templates` manifest, apply the
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

  let manifest: OrgManifest | null;
  try {
    manifest = await readOrgManifest(orgSpec.scope);
  } catch (error) {
    if (error instanceof OrgManifestSchemaError) {
      prompts.log.error(error.message);
    } else {
      prompts.log.error(
        `Failed to read ${orgSpec.scope}/create manifest: ${(error as Error).message}`,
      );
    }
    // Never silently skip the picker when the user explicitly typed `@org`.
    cancelAndExit('Failed to resolve org template manifest', 1);
  }

  if (!manifest) {
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
      return { kind: 'escape-hatch' };
    }
    rejectMonorepoEntryInsideMonorepo(picked.entry, args.isMonorepo);
    return resolveEntry(manifest, picked.entry);
  }

  const entry = manifest.templates.find((candidate) => candidate.name === orgSpec.name);
  if (!entry) {
    // Fall through to the existing `@scope/create-name` shorthand in discovery.ts.
    return { kind: 'passthrough' };
  }
  rejectMonorepoEntryInsideMonorepo(entry, args.isMonorepo);
  return resolveEntry(manifest, entry);
}

/**
 * Read `create.defaultTemplate` from the project's `vite.config.ts`.
 *
 * Best-effort: if there's no config file or evaluation fails, return
 * `undefined` so the create flow behaves as if no default was set.
 */
export async function getConfiguredDefaultTemplate(
  workspaceRootDir: string,
): Promise<string | undefined> {
  if (!findViteConfigUp(workspaceRootDir, workspaceRootDir)) {
    return undefined;
  }
  try {
    // Dynamic-import the heavy resolver only when a config file is present;
    // bare `vp create` in a fresh dir should not pay this cost.
    const { resolveViteConfig } = await import('../resolve-vite-config.ts');
    const config = (await resolveViteConfig(workspaceRootDir)) as {
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
