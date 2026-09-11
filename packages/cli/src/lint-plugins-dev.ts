// Keep test utilities separate so plugin imports do not load them at runtime.
// The subpath mirrors oxlint/plugins-dev and uses the bundled Oxlint version.

export { RuleTester } from 'oxlint/plugins-dev';
export type * from 'oxlint/plugins-dev';
