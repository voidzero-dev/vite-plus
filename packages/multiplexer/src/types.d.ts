export type Command = {
  name: string;
  cmd: string;
  args: string[];
  cwd: string;
  env?: NodeJS.ProcessEnv;
};
