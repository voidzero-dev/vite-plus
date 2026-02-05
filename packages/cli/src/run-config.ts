export type Task = {
  /**
   * The command to run for the task.
   *
   * If omitted, the script from `package.json` with the same name will be used
   */
  command?: string;
  /**
   * The working directory for the task, relative to the package root (not workspace root).
   */
  cwd?: string;
  /**
   * Dependencies of this task. Use `package-name#task-name` to refer to tasks in other packages.
   */
  dependsOn?: Array<string>;
} & (
  | {
      /**
       * Whether to cache the task
       */
      cache?: true;
      /**
       * Environment variable names to be fingerprinted and passed to the task.
       */
      envs?: Array<string>;
      /**
       * Environment variable names to be passed to the task without fingerprinting.
       */
      passThroughEnvs?: Array<string>;
    }
  | {
      /**
       * Whether to cache the task
       */
      cache: false;
    }
);

export type RunConfig = {
  /**
   * Enable cache for all scripts from package.json.
   *
   * This option can only be set in the workspace root's config file.
   * Setting it in a package's config will result in an error.
   */
  cacheScripts?: boolean;
  /**
   * Task definitions
   */
  tasks?: { [key in string]?: Task };
};
