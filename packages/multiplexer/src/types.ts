export type Command = {
  name: string;
  cmd: string;
  args: string[];
  cwd: string;
  env?: NodeJS.ProcessEnv;
  mode?: 'watch';
};

export type Dimensions = {
  width: number;
  height: number;
  top: number;
  left: number;
};

export type Position = 'left' | 'right' | 'top' | 'bottom';

export type PanelState = 'idle' | 'running' | 'done' | 'error';
