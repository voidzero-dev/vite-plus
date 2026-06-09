import type { CreateTemplateEntry } from './org-manifest.ts';
import { BuiltinTemplate } from './templates/types.ts';

export interface InitialTemplateOption {
  label: string;
  value: string;
  hint: string;
}

export function getInitialTemplateOptions(
  isMonorepo: boolean,
  templates: CreateTemplateEntry[] = [],
): InitialTemplateOption[] {
  return [
    ...(!isMonorepo
      ? [
          {
            label: 'Vite+ Monorepo',
            value: BuiltinTemplate.monorepo,
            hint: 'Create a new Vite+ monorepo project',
          },
        ]
      : []),
    {
      label: 'Vite+ Application',
      value: BuiltinTemplate.application,
      hint: 'Create vite applications',
    },
    {
      label: 'Vite+ Library',
      value: BuiltinTemplate.library,
      hint: 'Create vite libraries',
    },
    // Local templates declared in `create.templates` (vite.config.ts) are only
    // relevant inside the monorepo that owns them. They are selected by `name`
    // and resolved to their `template` specifier in the create flow.
    ...(isMonorepo
      ? templates.map((entry) => ({
          label: entry.name,
          value: entry.name,
          hint: entry.description,
        }))
      : []),
  ];
}
