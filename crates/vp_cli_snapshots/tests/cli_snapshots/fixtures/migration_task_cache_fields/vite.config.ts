import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: {
        command: 'vp build',
        // Rebuild when the deploy target changes.
        env: ['DEPLOY_TARGET'],
        untrackedEnv: ['CI'],
        input: [{ auto: true }, '!dist/**'],
        output: ['dist/**'], // restored on cache hits
      },
      typecheck: {
        command: 'tsc --noEmit',
        cache: true,
        env: ['TSC_MODE'],
      },
      lint: {
        command: 'vp lint',
        cache: {
          env: ['LINT_LEVEL'],
        },
        input: ['src/**'],
      },
    },
  },
});
