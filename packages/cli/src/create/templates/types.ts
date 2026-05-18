export const LibraryTemplateRepo = 'github:sxzz/tsdown-templates/vite-plus';

export const BuiltinTemplate = {
  generator: 'vite:generator',
  monorepo: 'vite:monorepo',
  application: 'vite:application',
  library: 'vite:library',
} as const;
export type BuiltinTemplate = (typeof BuiltinTemplate)[keyof typeof BuiltinTemplate];

export const TemplateType = {
  builtin: 'builtin',
  bingo: 'bingo',
  remote: 'remote',
  bundled: 'bundled',
} as const;
export type TemplateType = (typeof TemplateType)[keyof typeof TemplateType];

export interface TemplateInfo {
  command: string;
  args: string[];
  envs: NodeJS.ProcessEnv;
  type: TemplateType;
  // The parent directory of the generated package, only for monorepo
  // For example, "packages"
  parentDir?: string;
  interactive?: boolean;
  // Absolute path to an extracted template directory. Only set for
  // `TemplateType.bundled` entries sourced from a manifest's relative path.
  localPath?: string;
}

export interface BuiltinTemplateInfo extends Omit<TemplateInfo, 'parentDir'> {
  packageName: string;
  targetDir: string;
}
