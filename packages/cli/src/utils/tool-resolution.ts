import { DEFAULT_ENVS } from './constants.ts';

export interface ToolResolution {
  binPath: string;
  envs: Record<string, string>;
}

export function createToolResolution(
  binPath: string,
  envs: Record<string, string> = {},
): ToolResolution {
  return {
    binPath,
    envs: {
      ...DEFAULT_ENVS,
      ...envs,
    },
  };
}
