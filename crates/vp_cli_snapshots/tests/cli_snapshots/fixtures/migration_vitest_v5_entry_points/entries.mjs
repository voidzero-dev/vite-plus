import { BaseCoverageProvider } from 'vite-plus/test/coverage';
export { DefaultReporter } from 'vite-plus/test/reporters';
export * as environments from 'vite-plus/test/environments';
export { VitestSnapshotEnvironment } from 'vite-plus/test/snapshot';
export * as mocker from 'vite-plus/test/mocker';

export { BaseCoverageProvider };
export const loadEnvironment = () => import('vite-plus/test/environments');
