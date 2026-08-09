// Oxlint's dev-time plugin utilities (`RuleTester`), re-exported from the copy
// of Oxlint that ships with Vite+. Companion to `vite-plus/lint/plugins`: rule
// *tests* break the same way plugin *sources* do, just at a different specifier
// (these utilities live in `oxlint/plugins-dev`, not `@oxlint/plugins`).
//
// The subpath mirrors upstream's rather than naming today's single export, so
// the mapping stays a mechanical `oxlint/plugins-dev` ->
// `vite-plus/lint/plugins-dev`, and whatever upstream adds to that entry later
// still arrives under a name that fits.
//
// Kept out of `vite-plus/lint/plugins` on purpose: these are test-only, and
// importing the authoring API should not pull them in.

export { RuleTester } from 'oxlint/plugins-dev';
export type * from 'oxlint/plugins-dev';
