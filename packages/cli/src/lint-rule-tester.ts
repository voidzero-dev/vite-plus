// Oxlint's `RuleTester`, re-exported from the copy of Oxlint that ships with
// Vite+. Companion to `vite-plus/lint/plugins`: rule *tests* break the same way
// plugin *sources* do, just at a different specifier (`RuleTester` moved from
// `oxlint` to `oxlint/plugins-dev`).
//
// Kept out of `vite-plus/lint/plugins` on purpose: it is a test-only utility,
// and importing the authoring API should not pull it in.

export { RuleTester } from 'oxlint/plugins-dev';
export type * from 'oxlint/plugins-dev';
