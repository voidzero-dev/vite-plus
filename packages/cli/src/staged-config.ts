type StagedGenerateTask = (
  stagedFileNames: readonly string[],
) => string | string[] | Promise<string | string[]>;

type StagedTaskFunction = {
  title: string;
  task: (stagedFileNames: readonly string[]) => void | Promise<void>;
};

export type StagedConfig = Record<
  string,
  string | string[] | StagedGenerateTask | StagedGenerateTask[] | StagedTaskFunction
>;
