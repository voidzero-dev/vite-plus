import { BuiltinTemplate } from './templates/types.ts';

export interface InitialTemplateOption {
  label: string;
  value: string;
  hint: string;
}

export function getInitialTemplateOptions(isMonorepo: boolean): InitialTemplateOption[] {
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
  ];
}
