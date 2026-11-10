export interface MigrateConfig {
  /** Paths excluded from migration, using .gitignore syntax relative to this config. */
  ignorePatterns?: string[];
}

export function migrationIgnorePatterns(config: unknown): string[] {
  if (config === undefined) {
    return [];
  }
  if (config === null || typeof config !== 'object' || Array.isArray(config)) {
    throw new Error('migrate must be an object.');
  }
  const patterns = (config as MigrateConfig).ignorePatterns;
  if (patterns === undefined) {
    return [];
  }
  if (!Array.isArray(patterns) || patterns.some((pattern) => typeof pattern !== 'string')) {
    throw new Error('migrate.ignorePatterns must be an array of strings.');
  }
  return patterns;
}
