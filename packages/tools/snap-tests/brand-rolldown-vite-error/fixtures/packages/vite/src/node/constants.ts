import { version } from '../../package.json';

// upstream changed: renamed VERSION to VITE_VERSION
export const VITE_VERSION = version as string;
