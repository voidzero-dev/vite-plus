import { documentationUrl } from '../../utils/documentation.ts';

const guide = documentationUrl('/guide/vitest-v5');
const upstream = 'https://vitest.dev/guide/migration/';

const sections: Record<string, string> = {
  'api-conflict': 'browser-api-is-replaced-by-the-top-level-api',
  'assertion-types': 'assertion-types-expose-return-and-received-types',
  'async-render': 'render-is-async-in-vitest-browser-vue-and-vitest-browser-svelte',
  'benchmark-api': 'benchmarking-api-rewrite',
  'browser-automock': 'automocked-modules-stay-automocked-in-the-browser',
  'browser-session': 'browser-orchestrator-url-requires-a-session',
  'class-mock': 'class-mocks-keep-prototype-methods',
  'coverage-patterns': 'coverage-include-and-exclude-match-more-precisely',
  'coverage-thresholds': 'glob-coverage-thresholds-no-longer-inherit-perfile',
  'dom-global': 'dom-environment-global-assignments-now-update-the-underlying-window',
  'global-descriptors': 'populateglobal-returns-descriptors-in-originals',
  'locator-commands': 'locators-in-commands-are-serialized-as-objects',
  'nested-hoisted-mock': 'hoisted-mocking-calls-must-be-at-the-top-level',
  'nested-project-merge': 'referenced-config-files-can-define-their-own-projects',
  'poll-timeout': 'expect-poll-fails-when-it-times-out',
  'project-inheritance': 'inline-projects-inherit-the-root-config-by-default',
  'project-server-lifecycle': 'inline-projects-share-the-vite-server-by-default',
  'removed-api': 'removed-deprecated-entrypoints',
  'resolve-config': 'resolveconfig-returns-the-resolved-vite-config',
  'sequential-api': 'removed-test-sequential-describe-sequential-and-sequential-options',
  'temporal-system-time': 'fake-timers-and-setsystemtime-now-mock-temporal',
  'test-name-pattern': 'testnamepattern-matches-the-joined-full-name',
  'text-content': 'tohavetextcontent-now-performs-strict-equality',
  'text-content-project': 'tohavetextcontent-now-performs-strict-equality',
  'ui-token': 'vitest-ui-requires-an-authenticated-url',
  'unawaited-assertion': 'unawaited-asynchronous-assertions-fail-the-test',
  'worker-id': 'worker-and-concurrency-ids-are-1-based',
};

const localSections: Record<string, string> = {
  'browser-provider': 'community-webdriverio-provider',
  'dynamic-config': 'resolve-migration-findings',
  'dynamic-project': 'resolve-migration-findings',
  'global-api-ownership': 'resolve-migration-findings',
  'jest-dom-types': 'review-checklist',
  'merged-config-defaults': 'preserve-existing-behavior',
  'node-runtime': 'node-runtime',
  'overlapping-edits': 'resolve-migration-findings',
  'source-parse': 'resolve-migration-findings',
  'source-version': 'before-you-migrate',
  'static-collect': 'source-changes',
  'static-list': 'source-changes',
  'unsafe-syntax': 'resolve-migration-findings',
};

/** Give future/unknown findings a useful fallback rather than omitting help. */
export function vitestV5Documentation(code: string): string {
  if (sections[code]) {
    return `${upstream}#${sections[code]}`;
  }
  return `${guide}#${localSections[code] ?? 'resolve-migration-findings'}`;
}
