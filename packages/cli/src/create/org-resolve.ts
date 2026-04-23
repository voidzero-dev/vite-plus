import * as prompts from '@voidzero-dev/vite-plus-prompts';

import {
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
 * Resolution outcome for an org template spec. The caller (bin.ts) acts on
 * each variant:
 *
 * - `passthrough`: no manifest applied; `selectedTemplateName` stays as-is.
 * - `replaced`: manifest entry was selected and resolves to a non-bundled
 *   specifier (npm, github, vite:*, local). Caller should use the
 *   returned `templateName`.
 * - `bundled`: manifest entry uses a relative path; tarball has been
 *   extracted; caller should pass `bundledLocalPath` into
 *   `discoverTemplate`.
 * - `escape-hatch`: user picked "Vite+ built-in templates" from the org
 *   picker. Caller should fall through to the normal builtin flow.
 */
export type OrgResolution =
  | { kind: 'passthrough' }
  | { kind: 'replaced'; templateName: string; entry: OrgTemplateEntry }
  | { kind: 'bundled'; bundledLocalPath: string; entry: OrgTemplateEntry; templateName: string }
  | { kind: 'escape-hatch' };

function printNonInteractiveTable(
  manifest: OrgManifest,
  orgSpec: { scope: string },
  isMonorepo: boolean,
): void {
  const { lines, filteredCount } = formatManifestTable(manifest, isMonorepo);
  const stderr = process.stderr;
  stderr.write(
    `error: vp create ${orgSpec.scope} requires a template selection in non-interactive mode.\n`,
  );
  stderr.write('\n');
  stderr.write(`available templates from ${manifest.packageName}:\n`);
  stderr.write('\n');
  for (const line of lines) {
    stderr.write(`${line}\n`);
  }
  if (filteredCount > 0) {
    stderr.write('\n');
    stderr.write(
      `(omitted ${filteredCount} monorepo-only ${
        filteredCount === 1 ? 'entry' : 'entries'
      } because this workspace is already a monorepo)\n`,
    );
  }
  stderr.write('\n');
  const firstVisible = manifest.templates.find((t) => !(t.monorepo && isMonorepo));
  if (firstVisible) {
    stderr.write(
      `hint: rerun with an explicit selection, e.g. \`vp create ${orgSpec.scope}/${firstVisible.name}\`,\n`,
    );
  }
  stderr.write('      or use a Vite+ built-in template like `vp create vite:application`.\n');
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
  orgSpec: { scope: string; name?: string },
): Promise<OrgResolution> {
  if (entry.template.startsWith('./') || entry.template.startsWith('../')) {
    const extracted = await ensureOrgPackageExtracted(manifest);
    const bundledLocalPath = resolveBundledPath(extracted, entry.template);
    // Keep the original @scope spec visible to downstream logging /
    // parent-dir inference. Bundled scaffolding is driven by
    // bundledLocalPath.
    const templateName = orgSpec.name ? `${orgSpec.scope}/${entry.name}` : orgSpec.scope;
    return { kind: 'bundled', bundledLocalPath, entry, templateName };
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
    // Hard error — never silently skip the picker when the user
    // explicitly typed `@org`.
    if (error instanceof OrgManifestSchemaError) {
      prompts.log.error(error.message);
    } else {
      prompts.log.error(
        `Failed to read ${orgSpec.scope}/create manifest: ${(error as Error).message}`,
      );
    }
    cancelAndExit('Failed to resolve org template manifest', 1);
    // cancelAndExit exits the process; this is unreachable.
    return { kind: 'passthrough' };
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
      return { kind: 'passthrough' };
    }
    if (picked === ORG_PICKER_BUILTIN_ESCAPE) {
      return { kind: 'escape-hatch' };
    }
    rejectMonorepoEntryInsideMonorepo(picked.entry, args.isMonorepo);
    return resolveEntry(manifest, picked.entry, orgSpec);
  }

  const entry = manifest.templates.find((candidate) => candidate.name === orgSpec.name);
  if (!entry) {
    // No matching manifest entry → fall through to the existing
    // `@scope/create-name` shorthand handled by discovery.ts.
    return { kind: 'passthrough' };
  }
  rejectMonorepoEntryInsideMonorepo(entry, args.isMonorepo);
  return resolveEntry(manifest, entry, orgSpec);
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
  // Cheap pre-check: if no vite config file exists, skip the heavyweight
  // resolveViteConfig import entirely.
  const { findViteConfigUp, resolveViteConfig } = await import('../resolve-vite-config.ts');
  if (!findViteConfigUp(workspaceRootDir, workspaceRootDir)) {
    return undefined;
  }
  try {
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
