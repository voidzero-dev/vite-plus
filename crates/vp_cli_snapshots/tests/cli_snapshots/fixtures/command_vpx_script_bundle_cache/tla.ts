import { counter } from './shared.ts';

// A dependency that awaits an import() at its top level, whose target imports shared.ts
// from the same startup set.
const { lazy } = await import('./lazy.ts');
export const early = `${lazy()}:${counter.value}`;
