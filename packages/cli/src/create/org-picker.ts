import * as prompts from '@voidzero-dev/vite-plus-prompts';

import {
  filterManifestForContext,
  type OrgManifest,
  type OrgTemplateEntry,
} from './org-manifest.ts';

// Sentinel `value` for the picker's trailing "Vite+ built-in templates"
// entry. Internal to this module — callers react to
// `ORG_PICKER_BUILTIN_ESCAPE` instead.
const BUILTIN_ESCAPE_VALUE = '__vp_builtin_escape__';

export const ORG_PICKER_CANCEL = Symbol('org-picker-cancel');
export const ORG_PICKER_BUILTIN_ESCAPE = Symbol('org-picker-builtin-escape');

export type OrgPickerResult =
  | { kind: 'entry'; entry: OrgTemplateEntry }
  | typeof ORG_PICKER_CANCEL
  | typeof ORG_PICKER_BUILTIN_ESCAPE;

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
    // Rather than show a picker containing only the escape hatch, send
    // callers to the built-in flow directly — the most useful thing for a
    // user who ended up with nothing applicable to their workspace.
    return ORG_PICKER_BUILTIN_ESCAPE;
  }

  const options: { value: string; label: string; hint?: string }[] = filtered.map((entry) => ({
    value: entry.name,
    label: entry.name,
    hint: entry.description,
  }));
  options.push({
    value: BUILTIN_ESCAPE_VALUE,
    label: 'Vite+ built-in templates',
    hint: 'Use defaults (monorepo / application / library / generator)',
  });

  const picked = await prompts.select({
    message: `Pick a template from ${manifest.scope}`,
    options,
  });

  if (prompts.isCancel(picked)) {
    return ORG_PICKER_CANCEL;
  }
  if (picked === BUILTIN_ESCAPE_VALUE) {
    return ORG_PICKER_BUILTIN_ESCAPE;
  }
  const entry = filtered.find((candidate) => candidate.name === picked);
  if (!entry) {
    // Should never happen — the select only surfaces values we produced —
    // but fall back cleanly.
    return ORG_PICKER_CANCEL;
  }
  return { kind: 'entry', entry };
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
