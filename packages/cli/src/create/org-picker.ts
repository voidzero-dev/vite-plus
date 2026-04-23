import * as prompts from '@voidzero-dev/vite-plus-prompts';

import {
  filterManifestForContext,
  type OrgManifest,
  type OrgTemplateEntry,
} from './org-manifest.ts';

/**
 * Sentinel `value` used by the picker's trailing "Vite+ built-in templates"
 * entry. Callers compare against this to detect the escape-hatch path.
 */
export const BUILTIN_ESCAPE_VALUE = '__vp_builtin_escape__';

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

function padRight(value: string, width: number): string {
  if (value.length >= width) {
    return value;
  }
  return value + ' '.repeat(width - value.length);
}

/**
 * Render the manifest as a plain-text table suitable for the
 * `--no-interactive` error output. Context-filtered the same way as the
 * picker.
 *
 * The output is deliberately machine-parseable (fixed column order,
 * whitespace-separated) so that AI agents and scripts can recover the
 * available template names without a `--json` flag.
 *
 * Returns `{ lines, filteredCount }` so the caller can decide whether to
 * add a "(omitted N monorepo-only entries)" footer line.
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
  lines.push(`  ${padRight('NAME', nameWidth)}  ${padRight('DESCRIPTION', descWidth)}  TEMPLATE`);
  for (const entry of visible) {
    lines.push(
      `  ${padRight(entry.name, nameWidth)}  ${padRight(entry.description, descWidth)}  ${entry.template}`,
    );
  }
  return { lines, filteredCount };
}
