import { defineConfig } from 'vite';

// Models a project plugin that installs a process-level error backstop while
// its config is loaded. Re-throwing from this handler makes Node exit with code
// 7, which used to terminate `vp migrate` during its best-effort compatibility
// check instead of allowing migration to continue.
process.on('uncaughtException', (error) => {
  throw error;
});
queueMicrotask(() => {
  throw new Error('simulated project config crash');
});

export default defineConfig({});
