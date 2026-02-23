import { ROLLUP_HOOKS, VERSION } from './constants';

export function buildBanner() {
  return `vite v${VERSION} building for production...`;
}

export function buildError() {
  return `[vite]: Rolldown failed to resolve`;
}
