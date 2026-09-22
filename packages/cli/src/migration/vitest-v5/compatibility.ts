import { documentationUrl } from '../../utils/documentation.ts';
import type { AddedProperty } from './ast.ts';

const settings = {
  clearMocks: {
    value: 'false',
    reason: 'preserve mock call history.',
    adoption: 'Remove after tests no longer rely on calls from setup or earlier tests.',
    section: 'clearmocks-is-enabled-by-default',
  },
  sharedViteServer: {
    value: 'false',
    reason: 'keep separate Vite servers for inline projects.',
    adoption: 'Remove when plugins and config hooks can run once for shared projects.',
    section: 'inline-projects-share-the-vite-server-by-default',
  },
  extends: {
    value: 'false',
    reason: 'keep this inline project independent of the root config.',
    adoption: 'Remove to inherit root options, including plugins and setup files.',
    section: 'inline-projects-inherit-the-root-config-by-default',
  },
  exact: {
    value: 'false',
    reason: 'keep partial, case-insensitive locator matching.',
    adoption: 'Remove after updating locators for full, case-sensitive matches.',
    section: 'locators-are-strict-by-default',
  },
  perFile: {
    value: 'true',
    reason: 'enforce this coverage threshold per file.',
    adoption: 'Keep for per-file enforcement; remove to check matching files as a group.',
    section: 'glob-coverage-thresholds-no-longer-inherit-perfile',
  },
  toNotFake: {
    value: "['Temporal']",
    reason: 'exclude the global Temporal polyfill from fake timers.',
    adoption: 'Remove only the Temporal entry when it should follow mocked time.',
    section: 'fake-timers-and-setsystemtime-now-mock-temporal',
  },
};

/** Only attach this guidance to newly inserted v4 compatibility properties. */
export function compatibilityProperty(key: keyof typeof settings): AddedProperty {
  const { value, reason, adoption, section } = settings[key];
  return {
    key,
    value,
    comment: `// Vitest v4 compatibility: ${reason}\n// ${adoption}\n// ${documentationUrl('/guide/vitest-v5#remove-unneeded-compatibility-settings')}\n// https://vitest.dev/guide/migration/#${section}`,
  };
}
