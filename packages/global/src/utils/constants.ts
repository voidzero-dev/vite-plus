export const VITE_PLUS_NAME = 'vite-plus';
export const VITE_PLUS_VERSION = process.env.VITE_PLUS_VERSION || 'latest';

export const VITE_PLUS_OVERRIDE_PACKAGES: Record<string, string> = process.env
  .VITE_PLUS_OVERRIDE_PACKAGES
  ? JSON.parse(process.env.VITE_PLUS_OVERRIDE_PACKAGES)
  : {
      vite: 'npm:@voidzero-dev/vite-plus-core@latest',
      vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
    };
