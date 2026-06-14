import * as prompts from '@voidzero-dev/vite-plus-prompts';

// Spinner shims shared by prompts.ts, approve-builds.ts, and the migrator.
// They live here (rather than in prompts.ts) so approve-builds.ts can reuse
// them without an import cycle: prompts.ts already imports from approve-builds.ts.

export function getSpinner(interactive?: boolean) {
  if (interactive) {
    return prompts.spinner();
  }
  return {
    start: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
    stop: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
    message: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
  };
}

export function getSilentSpinner() {
  return {
    start: () => {},
    stop: () => {},
    message: () => {},
  };
}
