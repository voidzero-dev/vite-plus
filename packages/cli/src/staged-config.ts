// Copied from lint-staged@16.4.0 (node_modules/lint-staged/lib/index.d.ts)
// TODO: Re-export directly from lint-staged once we can bundle .d.ts files (#744).

type SyncGenerateTask = (stagedFileNames: readonly string[]) => string | string[];

type AsyncGenerateTask = (stagedFileNames: readonly string[]) => Promise<string | string[]>;

type GenerateTask = SyncGenerateTask | AsyncGenerateTask;

type TaskFunction = {
  title: string;
  task: (stagedFileNames: readonly string[]) => void | Promise<void>;
};

export type StagedConfig =
  | Record<string, string | TaskFunction | GenerateTask | (string | GenerateTask)[]>
  | GenerateTask;
