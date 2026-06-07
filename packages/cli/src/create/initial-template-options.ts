import type { WorkspacePackage } from '../types/workspace.ts';
import { BuiltinTemplate } from './templates/types.ts';

export interface InitialTemplateOption {
  label: string;
  value: string;
  hint: string;
}

export function getInitialTemplateOptions(
  isMonorepo: boolean,
  packages: WorkspacePackage[] = [],
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
    // Local generator packages (scaffolded by `vp create vite:generator`) are
    // only relevant inside the monorepo that owns them.
    ...(isMonorepo
      ? packages
          .filter((pkg) => pkg.isTemplatePackage)
          .map((pkg) => ({
            label: pkg.name,
            value: pkg.name,
            hint: pkg.description ?? pkg.path ?? '',
          }))
      : []),
  ];
}
