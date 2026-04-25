import { randomUUID } from 'node:crypto';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import {
  filterManifestForContext,
  type OrgManifest,
  type OrgTemplateEntry,
} from './org-manifest.ts';

export const ORG_PICKER_CANCEL = Symbol('org-picker-cancel');
export const ORG_PICKER_BUILTIN_ESCAPE = Symbol('org-picker-builtin-escape');

export type OrgPickerResult =
  | { kind: 'entry'; entry: OrgTemplateEntry }
  | typeof ORG_PICKER_CANCEL
  | typeof ORG_PICKER_BUILTIN_ESCAPE;

const ESCAPE_HATCH = Symbol('builtin-escape');

/**
 * Render the interactive picker for an org manifest. Always appends a
 * trailing "Vite+ built-in templates" escape-hatch entry.
 *
 * Context-filters entries with `monorepo: true` when running inside an
 * existing monorepo, mirroring `initial-template-options.ts:9-31`.
 *
 * Returns `ORG_PICKER_BUILTIN_ESCAPE` when the escape hatch is selected,
 * or `ORG_PICKER_CANCEL` when the user hits Ctrl-C.
 */
export async function pickOrgTemplate(
  manifest: OrgManifest,
  opts: { isMonorepo: boolean },
): Promise<OrgPickerResult> {
  const filtered = filterManifestForContext(manifest.templates, opts.isMonorepo);
  if (filtered.length === 0) {
    // Caller surfaces the context-specific reason before falling through.
    return ORG_PICKER_BUILTIN_ESCAPE;
  }

  // Per-invocation nonce — guarantees the escape hatch's `value` can't
  // collide with any user-provided manifest entry name no matter what
  // they chose.
  const escapeValue = `__vp_builtin_escape__::${randomUUID()}`;
  const lookup = new Map<string, OrgTemplateEntry | typeof ESCAPE_HATCH>();
  const options: { value: string; label: string; hint?: string }[] = filtered.map((entry) => {
    lookup.set(entry.name, entry);
    return { value: entry.name, label: entry.name, hint: entry.description };
  });
  lookup.set(escapeValue, ESCAPE_HATCH);
  // Mirror `getInitialTemplateOptions(isMonorepo)`: `monorepo` is hidden
  // inside an existing monorepo (and would be rejected at scaffold time
  // anyway); `generator` isn't part of the builtin picker at all.
  const builtinHint = opts.isMonorepo
    ? 'Use defaults (application / library)'
    : 'Use defaults (monorepo / application / library)';
  options.push({ value: escapeValue, label: 'Vite+ built-in templates', hint: builtinHint });

  const picked = await prompts.select({
    message: `Pick a template from ${manifest.scope}`,
    options,
  });

  if (prompts.isCancel(picked)) {
    return ORG_PICKER_CANCEL;
  }
  const found = lookup.get(picked);
  if (found === ESCAPE_HATCH) {
    return ORG_PICKER_BUILTIN_ESCAPE;
  }
  if (!found) {
    // Unreachable: every option's `value` was just registered in `lookup`
    // a few lines above. Throw rather than masquerade as a cancel — a
    // missing entry would mean a real internal bug.
    throw new Error(`org-picker: prompts.select returned an unregistered value: ${picked}`);
  }
  return { kind: 'entry', entry: found };
}

/**
 * Render the manifest as a plain-text table for the `--no-interactive`
 * error output. Fixed column order so AI agents and scripts can recover
 * available template names without a `--json` flag.
 */
export function formatManifestTable(
  manifest: OrgManifest,
  isMonorepo: boolean,
): { lines: string[]; filteredCount: number } {
  const visible = filterManifestForContext(manifest.templates, isMonorepo);
  const filteredCount = manifest.templates.length - visible.length;

  const nameWidth = Math.max('NAME'.length, ...visible.map((entry) => entry.name.length));
  const descWidth = Math.max(
    'DESCRIPTION'.length,
    ...visible.map((entry) => entry.description.length),
  );
  const lines: string[] = [];
  lines.push(`  ${'NAME'.padEnd(nameWidth)}  ${'DESCRIPTION'.padEnd(descWidth)}  TEMPLATE`);
  for (const entry of visible) {
    lines.push(
      `  ${entry.name.padEnd(nameWidth)}  ${entry.description.padEnd(descWidth)}  ${entry.template}`,
    );
  }
  return { lines, filteredCount };
}
