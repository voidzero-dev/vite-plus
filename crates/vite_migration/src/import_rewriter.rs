use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use ast_grep_config::RuleConfig;
use ast_grep_language::SupportLang;
use rayon::prelude::*;
use regex::Regex;
use vite_error::Error;

use crate::{ast_grep, file_walker};

/// ast-grep rules for rewriting vite imports and declare module statements
///
/// This rewrites:
/// - `import { ... } from 'vite'` → `import { ... } from 'vite-plus'`
/// - `import { ... } from 'vite/{name}'` → `import { ... } from 'vite-plus/{name}'`
/// - `require('vite')` → `require('vite-plus')`
/// - `require('vite/{name}')` → `require('vite-plus/{name}')`
/// - `declare module 'vite' { ... }` → `declare module 'vite-plus' { ... }`
/// - `declare module 'vite/{name}' { ... }` → `declare module 'vite-plus/{name}' { ... }`
///
/// The `require(...)` sibling rules constrain themselves to a literal `require`
/// callee so unrelated calls like `my_require('vite')` or `require.cache['vite']`
/// are NOT touched. The `*-dynamic-import` sibling rules match dynamic
/// `import('vite')` calls (and TS type-position `typeof import('vite')`) by
/// pinning the call_expression's `function` field to the `import` token kind,
/// which excludes ordinary identifier calls. Together with the upstream
/// `import_statement` rule (covering ES `import { ... } from 'vite'`), this
/// gives full coverage of static imports/exports, dynamic imports, and
/// CommonJS requires.
const REWRITE_VITE_RULES: &str = r#"---
id: rewrite-vite-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vite-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vite-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vite-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vite-subpath-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite/.+['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite/
      by: "vite-plus/"
fix: $NEW_IMPORT
---
id: rewrite-vite-subpath-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite/.+['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite/
      by: "vite-plus/"
fix: $NEW_IMPORT
---
id: rewrite-vite-subpath-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite/.+['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite/
      by: "vite-plus/"
fix: $NEW_IMPORT
---
id: rewrite-vite-subpath-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite/.+['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite/
      by: "vite-plus/"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-vite
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['\"]vite['\"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-vite-subpath
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['\"]vite/.+['\"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite/
      by: "vite-plus/"
fix: $NEW_IMPORT
"#;

/// ast-grep rules for rewriting vitest imports.
///
/// This rewrites (the canonical mapping shared with the `oxlint-plugin.ts`
/// `rewriteVitePlusImportSpecifier` autofix — both implementations MUST stay
/// in sync and only produce targets that exist in the `vite-plus` package
/// `exports` map, otherwise Node fails with `ERR_PACKAGE_PATH_NOT_EXPORTED`):
/// - `import { ... } from 'vitest'` → `import { ... } from 'vite-plus/test'`
/// - `import { ... } from 'vitest/config'` → `import { ... } from 'vite-plus'`
/// - `import { ... } from 'vitest/{name}'` → `import { ... } from 'vite-plus/test/{name}'`
/// - `import { ... } from '@vitest/browser'` → `import { ... } from 'vite-plus/test/browser'`
/// - `import { ... } from '@vitest/browser/context'` → `import { ... } from 'vite-plus/test/browser/context'`
/// - `import { ... } from '@vitest/browser/client'` → `import { ... } from 'vite-plus/test/client'`
/// - `import { ... } from '@vitest/browser/locators'` → `import { ... } from 'vite-plus/test/locators'`
/// - `import { ... } from '@vitest/browser/matchers'` → `import { ... } from 'vite-plus/test/matchers'`
/// - `import { ... } from '@vitest/browser/utils'` → `import { ... } from 'vite-plus/test/utils'`
///
///   Note: `vite-plus` only exports `./test/browser/context` under the nested
///   `browser/` path; `client`, `locators`, `matchers` and `utils` are exposed
///   at the bare `./test/{name}` surface, so the `/browser/` segment is stripped.
/// - `import { ... } from '@vitest/browser-playwright'` → `import { ... } from 'vite-plus/test/browser-playwright'`
/// - `import { ... } from '@vitest/browser-playwright/context'` → `import { ... } from 'vite-plus/test/browser/context'`
/// - `import { ... } from '@vitest/browser-playwright/provider'` → `import { ... } from 'vite-plus/test/browser/providers/playwright'`
/// - `import { ... } from '@vitest/browser-preview'` → `import { ... } from 'vite-plus/test/browser-preview'`
/// - `import { ... } from '@vitest/browser-preview/context'` → `import { ... } from 'vite-plus/test/browser/context'`
/// - `import { ... } from '@vitest/browser-preview/provider'` → `import { ... } from 'vite-plus/test/browser/providers/preview'`
/// - `import { ... } from '@vitest/browser-webdriverio'` → `import { ... } from 'vite-plus/test/browser-webdriverio'`
/// - `import { ... } from '@vitest/browser-webdriverio/context'` → `import { ... } from 'vite-plus/test/browser/context'`
/// - `import { ... } from '@vitest/browser-webdriverio/provider'` → `import { ... } from 'vite-plus/test/browser/providers/webdriverio'`
///
/// `declare module 'vitest' { ... }` (and the subpath/`@vitest/*` variants) are
/// intentionally NOT rewritten — the `vite-plus/test*` subpaths are thin shims
/// that `export * from 'vitest'` (and the browser provider packages), so the
/// underlying type identity is upstream. Augmenting `'vite-plus/test'` would
/// only augment the shim module and would not merge into the upstream types
/// the user actually sees through their `import { expect } from 'vite-plus/test'`
/// statements. Leaving the `declare module 'vitest' { ... }` alone keeps
/// augmentations targeting the real upstream module identity.
///
/// `vitest/package.json` is also intentionally NOT rewritten. It is a
/// metadata-access pattern (typically used to read the vitest version) and
/// `vite-plus`'s generated exports map deliberately does not expose
/// `./test/package.json` (see `syncTestPackageExports()` in
/// `packages/cli/build.ts`, which skips the upstream `./package.json`
/// export). Rewriting it would yield `vite-plus/test/package.json`, which
/// fails at runtime with `ERR_PACKAGE_PATH_NOT_EXPORTED`. The original
/// specifier still resolves through the transitively-installed `vitest`
/// dependency.
const REWRITE_VITEST_RULES: &str = r#"---
id: rewrite-vitest-config-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/config['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/config
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vitest-config-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/config['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/config
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vitest-config-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/config['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/config
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vitest-config-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/config['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/config
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vitest-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest
      by: "vite-plus/test"
fix: $NEW_IMPORT
---
id: rewrite-vitest-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest
      by: "vite-plus/test"
fix: $NEW_IMPORT
---
id: rewrite-vitest-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest
      by: "vite-plus/test"
fix: $NEW_IMPORT
---
id: rewrite-vitest-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest
      by: "vite-plus/test"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser"
      by: "vite-plus/test/browser"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser"
      by: "vite-plus/test/browser"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser"
      by: "vite-plus/test/browser"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser"
      by: "vite-plus/test/browser"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-context-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser/context['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-context-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser/context['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-context-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser/context['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-context-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser/context['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-flat-subpath-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser/(client|locators|matchers|utils)['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser/"
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-flat-subpath-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser/(client|locators|matchers|utils)['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser/"
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-flat-subpath-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser/(client|locators|matchers|utils)['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser/"
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-flat-subpath-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser/(client|locators|matchers|utils)['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser/"
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright"
      by: "vite-plus/test/browser-playwright"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright"
      by: "vite-plus/test/browser-playwright"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright"
      by: "vite-plus/test/browser-playwright"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright"
      by: "vite-plus/test/browser-playwright"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-context-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright/context['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-context-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright/context['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-context-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright/context['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-context-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright/context['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-provider-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright/provider['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright/provider"
      by: "vite-plus/test/browser/providers/playwright"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-provider-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright/provider['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright/provider"
      by: "vite-plus/test/browser/providers/playwright"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-provider-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright/provider['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright/provider"
      by: "vite-plus/test/browser/providers/playwright"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-playwright-provider-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-playwright/provider['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-playwright/provider"
      by: "vite-plus/test/browser/providers/playwright"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview"
      by: "vite-plus/test/browser-preview"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview"
      by: "vite-plus/test/browser-preview"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview"
      by: "vite-plus/test/browser-preview"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview"
      by: "vite-plus/test/browser-preview"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-context-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview/context['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-context-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview/context['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-context-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview/context['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-context-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview/context['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-provider-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview/provider['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview/provider"
      by: "vite-plus/test/browser/providers/preview"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-provider-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview/provider['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview/provider"
      by: "vite-plus/test/browser/providers/preview"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-provider-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview/provider['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview/provider"
      by: "vite-plus/test/browser/providers/preview"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-preview-provider-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-preview/provider['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-preview/provider"
      by: "vite-plus/test/browser/providers/preview"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio"
      by: "vite-plus/test/browser-webdriverio"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio"
      by: "vite-plus/test/browser-webdriverio"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio"
      by: "vite-plus/test/browser-webdriverio"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio"
      by: "vite-plus/test/browser-webdriverio"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-context-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio/context['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-context-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio/context['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-context-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio/context['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-context-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio/context['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio/context"
      by: "vite-plus/test/browser/context"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-provider-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio/provider['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio/provider"
      by: "vite-plus/test/browser/providers/webdriverio"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-provider-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio/provider['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio/provider"
      by: "vite-plus/test/browser/providers/webdriverio"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-provider-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio/provider['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio/provider"
      by: "vite-plus/test/browser/providers/webdriverio"
fix: $NEW_IMPORT
---
id: rewrite-vitest-browser-webdriverio-provider-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/browser-webdriverio/provider['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/browser-webdriverio/provider"
      by: "vite-plus/test/browser/providers/webdriverio"
fix: $NEW_IMPORT
---
id: rewrite-vitest-subpath-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/.+['"]$
  not:
    regex: ^['"]vitest/(package\.json|config)['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-vitest-subpath-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/.+['"]$
  not:
    regex: ^['"]vitest/(package\.json|config)['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-vitest-subpath-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/.+['"]$
  not:
    regex: ^['"]vitest/(package\.json|config)['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-vitest-subpath-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/.+['"]$
  not:
    regex: ^['"]vitest/(package\.json|config)['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/
      by: "vite-plus/test/"
fix: $NEW_IMPORT
"#;

/// ast-grep rules for rewriting tsdown imports and declare module statements
///
/// This rewrites:
/// - `import { ... } from 'tsdown'` → `import { ... } from 'vite-plus/pack'`
/// - `declare module 'tsdown' { ... }` → `declare module 'vite-plus/pack' { ... }`
const REWRITE_TSDOWN_RULES: &str = r#"---
id: rewrite-tsdown-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown
      by: "vite-plus/pack"
fix: $NEW_IMPORT
---
id: rewrite-tsdown-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown
      by: "vite-plus/pack"
fix: $NEW_IMPORT
---
id: rewrite-tsdown-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown
      by: "vite-plus/pack"
fix: $NEW_IMPORT
---
id: rewrite-tsdown-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown
      by: "vite-plus/pack"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-tsdown
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown['"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown
      by: "vite-plus/pack"
fix: $NEW_IMPORT
---
id: rewrite-tsdown-client-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown/client['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown/client
      by: "vite-plus/pack/client"
fix: $NEW_IMPORT
---
id: rewrite-tsdown-client-export
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown/client['"]$
  inside:
    kind: export_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown/client
      by: "vite-plus/pack/client"
fix: $NEW_IMPORT
---
id: rewrite-tsdown-client-require
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown/client['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        regex: ^require$
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown/client
      by: "vite-plus/pack/client"
fix: $NEW_IMPORT
---
id: rewrite-tsdown-client-dynamic-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown/client['"]$
  inside:
    kind: arguments
    inside:
      kind: call_expression
      has:
        field: function
        kind: import
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown/client
      by: "vite-plus/pack/client"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-tsdown-client
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown/client['"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown/client
      by: "vite-plus/pack/client"
fix: $NEW_IMPORT
"#;

static PARSED_VITE_RULES: LazyLock<Vec<RuleConfig<SupportLang>>> = LazyLock::new(|| {
    ast_grep::load_rules(REWRITE_VITE_RULES).expect("failed to parse vite rewrite rules")
});

static PARSED_VITEST_RULES: LazyLock<Vec<RuleConfig<SupportLang>>> = LazyLock::new(|| {
    ast_grep::load_rules(REWRITE_VITEST_RULES).expect("failed to parse vitest rewrite rules")
});

const BARE_VITEST_RULE_IDS: [&str; 4] = [
    "rewrite-vitest-import",
    "rewrite-vitest-export",
    "rewrite-vitest-require",
    "rewrite-vitest-dynamic-import",
];

fn is_bare_vitest_rule(rule: &RuleConfig<SupportLang>) -> bool {
    BARE_VITEST_RULE_IDS.contains(&rule.id.as_str())
}

fn is_unscoped_vitest_rule(rule: &RuleConfig<SupportLang>) -> bool {
    is_bare_vitest_rule(rule)
        || rule.id.starts_with("rewrite-vitest-config-")
        || rule.id.starts_with("rewrite-vitest-subpath-")
}

static PARSED_UNSCOPED_VITEST_RULES: LazyLock<Vec<RuleConfig<SupportLang>>> = LazyLock::new(|| {
    ast_grep::load_rules(REWRITE_VITEST_RULES)
        .expect("failed to parse vitest rewrite rules")
        .into_iter()
        .filter(is_unscoped_vitest_rule)
        .collect()
});

static PARSED_VITEST_RULES_WITHOUT_UNSCOPED: LazyLock<Vec<RuleConfig<SupportLang>>> =
    LazyLock::new(|| {
        ast_grep::load_rules(REWRITE_VITEST_RULES)
            .expect("failed to parse vitest rewrite rules")
            .into_iter()
            .filter(|rule| !is_unscoped_vitest_rule(rule))
            .collect()
    });

static PARSED_TSDOWN_RULES: LazyLock<Vec<RuleConfig<SupportLang>>> = LazyLock::new(|| {
    ast_grep::load_rules(REWRITE_TSDOWN_RULES).expect("failed to parse tsdown rewrite rules")
});

// Regex patterns for rewriting `/// <reference types="..." />` directives.
// These cannot be handled by ast-grep because triple-slash references are parsed as comments.

/// `vitest/config` → `vite-plus` (special case, must be applied before generic vitest subpath)
static RE_REF_VITEST_CONFIG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vitest/config(["']\s*/>)"#).unwrap()
});

/// bare `vitest` → `vite-plus/test`
static RE_REF_VITEST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vitest(["']\s*/>)"#).unwrap()
});

/// `vitest/{subpath}` → `vite-plus/test/{subpath}`
static RE_REF_VITEST_SUBPATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vitest/(.+?)(["']\s*/>)"#).unwrap()
});

/// `@vitest/browser[/{subpath}]` references that map onto the *nested*
/// `vite-plus/test/browser[/{subpath}]` surface (a plain `@vitest/` → `vite-plus/test/`
/// swap is correct here):
///   - `@vitest/browser` → `vite-plus/test/browser`
///   - `@vitest/browser/context` → `vite-plus/test/browser/context`
///   - `@vitest/browser/providers/{name}` → `vite-plus/test/browser/providers/{name}`
static RE_REF_VITEST_SCOPED_BROWSER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^(\s*///\s*<reference\s+types\s*=\s*["'])@vitest/(browser(?:/(?:context|providers/.+?))?)(["']\s*/>)"#,
    )
    .unwrap()
});

/// `@vitest/browser/{client,locators,matchers,utils}` references. `vite-plus` only
/// exposes these four at the *bare* `./test/{name}` surface (NOT under `./test/browser/`),
/// so the `/browser/` segment is stripped: `@vitest/browser/{name}` → `vite-plus/test/{name}`.
static RE_REF_VITEST_SCOPED_BROWSER_FLAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^(\s*///\s*<reference\s+types\s*=\s*["'])@vitest/browser/(client|locators|matchers|utils)(["']\s*/>)"#,
    )
    .unwrap()
});

/// `@vitest/browser-{provider}` (exact, no subpath) →
/// `vite-plus/test/browser-{provider}` — a plain `@vitest/` → `vite-plus/test/` swap.
static RE_REF_VITEST_SCOPED_PROVIDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^(\s*///\s*<reference\s+types\s*=\s*["'])@vitest/(browser-playwright|browser-preview|browser-webdriverio)(["']\s*/>)"#,
    )
    .unwrap()
});

/// `@vitest/browser-{provider}/context` references. `vite-plus` projects every
/// provider's `context` onto the shared `./test/browser/context` export, so the
/// provider segment is dropped: → `vite-plus/test/browser/context`.
static RE_REF_VITEST_SCOPED_PROVIDER_CONTEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^(\s*///\s*<reference\s+types\s*=\s*["'])@vitest/browser-(playwright|preview|webdriverio)/context(["']\s*/>)"#,
    )
    .unwrap()
});

/// `@vitest/browser-{provider}/provider` references. `vite-plus` exposes provider
/// entry points at `./test/browser/providers/{provider}`, so the subpath is
/// rewritten accordingly: → `vite-plus/test/browser/providers/{provider}`.
static RE_REF_VITEST_SCOPED_PROVIDER_ENTRY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^(\s*///\s*<reference\s+types\s*=\s*["'])@vitest/browser-(playwright|preview|webdriverio)/provider(["']\s*/>)"#,
    )
    .unwrap()
});

/// bare `vite` → `vite-plus`
static RE_REF_VITE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vite(["']\s*/>)"#).unwrap()
});

/// `vite/{subpath}` → `vite-plus/{subpath}`
static RE_REF_VITE_SUBPATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vite/(.+?)(["']\s*/>)"#).unwrap()
});

/// bare `tsdown` → `vite-plus/pack`
static RE_REF_TSDOWN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])tsdown(["']\s*/>)"#).unwrap()
});

/// `tsdown/client` → `vite-plus/pack/client`
static RE_REF_TSDOWN_CLIENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])tsdown/client(["']\s*/>)"#).unwrap()
});

/// Apply a single regex replacement, updating `content` in place if matched.
/// Uses `Cow::Owned` variant check to avoid O(n) string comparison on no-match.
/// Uses `replace` (not `replace_all`) since each line contains at most one reference directive.
fn apply_regex_replace(content: &mut String, re: &Regex, replacement: &str) -> bool {
    use std::borrow::Cow;
    match re.replace(content, replacement) {
        Cow::Owned(new) => {
            *content = new;
            true
        }
        Cow::Borrowed(_) => false,
    }
}

/// Rewrite `/// <reference types="..." />` directives in place.
///
/// Only processes the file preamble (blank lines and comments before the first statement)
/// to match TypeScript semantics and avoid false positives inside string/template literals.
/// Allocates only for preamble lines, leaving the file body untouched.
/// Returns whether any changes were made.
fn rewrite_reference_types(
    content: &mut String,
    skip_packages: &SkipPackages,
    preserve_unscoped_vitest: bool,
) -> bool {
    // Fast path: skip files with no triple-slash reference directives.
    // Check for "///" which covers all spacing variants (///<ref, /// <ref, ///\t<ref).
    if !content.contains("///") {
        return false;
    }

    // Find the byte offset where the preamble ends.
    // TypeScript allows triple-slash directives after blank lines, single-line comments (//),
    // block comments (/* ... */), a UTF-8 BOM, and a shebang line.
    // The preamble ends at the first non-comment statement.
    let bytes = content.as_bytes();
    let mut preamble_end = 0;
    let mut in_block_comment = false;

    // Advance preamble_end past a line and its terminator (\n or \r\n).
    let advance_past_line = |offset: usize, line_len: usize| -> usize {
        let mut pos = offset + line_len;
        if pos < bytes.len() && bytes[pos] == b'\r' {
            pos += 1;
        }
        if pos < bytes.len() && bytes[pos] == b'\n' {
            pos += 1;
        }
        pos
    };

    // Check what follows after a `*/` close, scanning past any additional `/* ... */` pairs.
    // Returns `None` if code follows (caller should break).
    // Returns `Some(true)` if an unclosed `/*` follows (enter block comment).
    // Returns `Some(false)` if the rest is empty, a `//` comment, or only closed block comments.
    let check_after_close = |text: &str| -> Option<bool> {
        let mut remaining = text.trim();
        loop {
            if remaining.is_empty() || remaining.starts_with("//") {
                return Some(false);
            }
            if !remaining.starts_with("/*") {
                return None; // Code follows — end of preamble.
            }
            // Another block comment starts — check if it closes on this line.
            match remaining[2..].find("*/") {
                Some(pos) => remaining = remaining[2 + pos + 2..].trim(),
                None => return Some(true), // Unclosed — enter block comment.
            }
        }
    };

    for line in content.lines() {
        // Strip UTF-8 BOM (U+FEFF) before trimming — Rust's trim() does not remove BOM.
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if in_block_comment {
            if let Some(pos) = trimmed.find("*/") {
                match check_after_close(&trimmed[pos + 2..]) {
                    None => break, // code after */ — end of preamble
                    Some(new_block) => in_block_comment = new_block,
                }
            }
            preamble_end = advance_past_line(preamble_end, line.len());
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#!") {
            preamble_end = advance_past_line(preamble_end, line.len());
            continue;
        }
        if trimmed.starts_with("/*") {
            if let Some(pos) = trimmed.find("*/") {
                match check_after_close(&trimmed[pos + 2..]) {
                    None => break,
                    Some(new_block) => in_block_comment = new_block,
                }
            } else {
                in_block_comment = true;
            }
            preamble_end = advance_past_line(preamble_end, line.len());
            continue;
        }
        break;
    }

    // Guard: unclosed block comment means the file has a syntax error; skip rewriting.
    if in_block_comment {
        return false;
    }

    let preamble = &content[..preamble_end];
    // Check for "///" which covers all spacing variants (///<ref, /// <ref, etc.)
    if !preamble.contains("///") {
        return false;
    }

    // Detect the line ending style used in the preamble for faithful reconstruction.
    let line_ending = if preamble.contains("\r\n") { "\r\n" } else { "\n" };

    let mut changed = false;
    let mut preamble_lines: Vec<String> =
        preamble.lines().map(std::string::ToString::to_string).collect();
    // Strip UTF-8 BOM from the first preamble line so the regex `^(\s*///` can match.
    if let Some(first) = preamble_lines.first_mut()
        && first.starts_with('\u{feff}')
    {
        *first = first.trim_start_matches('\u{feff}').to_string();
    }

    for line in &mut preamble_lines {
        // The regexes handle flexible spacing (///\s*<reference), so just check for "///"
        // to avoid filtering out valid variants like ///<reference or ///\t<reference.
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with("///") {
            continue;
        }
        // Each line matches at most one pattern; use early exit to skip remaining regexes.
        if !skip_packages.skip_vitest {
            if !preserve_unscoped_vitest
                && apply_regex_replace(line, &RE_REF_VITEST_CONFIG, "${1}vite-plus${2}")
            {
                changed = true;
                continue;
            }
            // `@vitest/browser-{provider}/{context,provider}` must be matched before the
            // bare provider regex so the more specific subpath rewrite wins.
            if apply_regex_replace(
                line,
                &RE_REF_VITEST_SCOPED_PROVIDER_CONTEXT,
                "${1}vite-plus/test/browser/context${3}",
            ) {
                changed = true;
                continue;
            }
            if apply_regex_replace(
                line,
                &RE_REF_VITEST_SCOPED_PROVIDER_ENTRY,
                "${1}vite-plus/test/browser/providers/${2}${3}",
            ) {
                changed = true;
                continue;
            }
            if apply_regex_replace(
                line,
                &RE_REF_VITEST_SCOPED_PROVIDER,
                "${1}vite-plus/test/${2}${3}",
            ) {
                changed = true;
                continue;
            }
            // `@vitest/browser/{client,locators,matchers,utils}` strips the `/browser/`
            // segment; the generic `@vitest/browser[/...]` rule keeps it.
            if apply_regex_replace(
                line,
                &RE_REF_VITEST_SCOPED_BROWSER_FLAT,
                "${1}vite-plus/test/${2}${3}",
            ) {
                changed = true;
                continue;
            }
            if apply_regex_replace(
                line,
                &RE_REF_VITEST_SCOPED_BROWSER,
                "${1}vite-plus/test/${2}${3}",
            ) {
                changed = true;
                continue;
            }
            if !preserve_unscoped_vitest
                && apply_regex_replace(line, &RE_REF_VITEST_SUBPATH, "${1}vite-plus/test/${2}${3}")
            {
                changed = true;
                continue;
            }
            if !preserve_unscoped_vitest
                && apply_regex_replace(line, &RE_REF_VITEST, "${1}vite-plus/test${2}")
            {
                changed = true;
                continue;
            }
        }
        if !skip_packages.skip_vite {
            if apply_regex_replace(line, &RE_REF_VITE_SUBPATH, "${1}vite-plus/${2}${3}") {
                changed = true;
                continue;
            }
            if apply_regex_replace(line, &RE_REF_VITE, "${1}vite-plus${2}") {
                changed = true;
                continue;
            }
        }
        if !skip_packages.skip_tsdown {
            if apply_regex_replace(line, &RE_REF_TSDOWN_CLIENT, "${1}vite-plus/pack/client${2}") {
                changed = true;
                continue;
            }
            if apply_regex_replace(line, &RE_REF_TSDOWN, "${1}vite-plus/pack${2}") {
                changed = true;
            }
        }
    }

    if changed {
        let suffix = &content[preamble_end..];
        let mut result = preamble_lines.join(line_ending);
        // Re-add the line ending between preamble and suffix if the original had one
        if preamble_end < content.len() || preamble.ends_with('\n') {
            result.push_str(line_ending);
        }
        result.push_str(suffix);
        *content = result;
    }

    changed
}

/// Packages to skip rewriting based on peerDependencies or dependencies
#[derive(Debug, Clone, Copy, Default)]
struct SkipPackages {
    /// Skip rewriting vite imports (vite is in peerDependencies or dependencies)
    skip_vite: bool,
    /// Skip rewriting vitest imports (vitest is in peerDependencies or dependencies)
    skip_vitest: bool,
    /// Skip rewriting tsdown imports (tsdown is in peerDependencies or dependencies)
    skip_tsdown: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct PackageRewriteContext {
    skip_packages: SkipPackages,
    uses_nuxt_test_utils: bool,
}

/// Options controlling directory-wide import rewriting.
#[derive(Debug, Clone, Copy, Default)]
pub struct RewriteImportsOptions {
    /// Preserve `vitest` and `vitest/*` module specifiers throughout packages
    /// whose nearest package.json declares `@nuxt/test-utils`.
    pub preserve_vitest_in_nuxt_packages: bool,
}

impl SkipPackages {
    /// Check if all packages should be skipped (file can be skipped entirely)
    const fn all_skipped(&self) -> bool {
        self.skip_vite && self.skip_vitest && self.skip_tsdown
    }
}

/// Find the nearest package.json by walking up from the file's directory.
/// Stops at the root directory.
fn find_nearest_package_json(file_path: &Path, root: &Path) -> Option<PathBuf> {
    let mut current = file_path.parent()?;

    loop {
        let package_json = current.join("package.json");
        if package_json.exists() {
            return Some(package_json);
        }

        // Stop if we've reached the root
        if current == root {
            break;
        }

        // Move to parent directory
        current = current.parent()?;
    }

    None
}

/// Returns whether the package follows a published-plugin naming convention
/// whose code is consumed by Vite projects: its unscoped name starts with
/// `vite-plugin-` (<https://vite.dev/guide/api-plugin>) or `unplugin-`
/// (<https://unplugin.unjs.io/guide/plugin-conventions.html>; cross-bundler
/// plugins that ship a Vite entry and are used by Vite projects).
///
/// For a scoped name like `@scope/vite-plugin-foo`, the part after the first `/`
/// is the unscoped name; for an unscoped name the whole name is used. The
/// trailing hyphen is required, so `vite-plugin`, `unplugin`, and their plurals
/// do not match.
fn package_name_is_plugin(pkg: &serde_json::Value) -> bool {
    let Some(name) = pkg.get("name").and_then(|v| v.as_str()) else {
        return false;
    };

    // Strip a leading `@scope/` for scoped packages, otherwise use the name as-is.
    let unscoped = if name.starts_with('@') {
        match name.split_once('/') {
            Some((_, rest)) => rest,
            None => return false,
        }
    } else {
        name
    };

    unscoped.starts_with("vite-plugin-") || unscoped.starts_with("unplugin-")
}

/// Parse package.json and check which packages are in peerDependencies or dependencies.
/// Returns default (no skipping) if package.json doesn't exist or can't be parsed.
fn get_package_rewrite_context(package_json_path: &Path) -> PackageRewriteContext {
    let content = match std::fs::read_to_string(package_json_path) {
        Ok(c) => c,
        Err(_) => return PackageRewriteContext::default(),
    };

    let pkg: serde_json::Value = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(_) => return PackageRewriteContext::default(),
    };

    // Helper to check if a package exists in a dependencies object
    let has_package = |deps_key: &str, package_name: &str| -> bool {
        pkg.get(deps_key)
            .and_then(|v| v.as_object())
            .is_some_and(|deps| deps.contains_key(package_name))
    };

    // Peer and runtime dependencies preserve the existing whole-package skip
    // behavior. Nuxt compatibility is narrower and accepts the three install
    // groups where @nuxt/test-utils is normally declared.
    //
    // `vite` is additionally skipped when the package follows a published-plugin
    // naming convention: an unscoped name starting with `vite-plugin-`
    // (<https://vite.dev/guide/api-plugin>) or `unplugin-`
    // (<https://unplugin.unjs.io/guide/plugin-conventions.html>). Such libraries
    // are consumed by both vite and vite-plus projects, so rewriting their
    // `vite` imports to `vite-plus` would break plain-vite consumers; many
    // declare `vite` only in devDependencies and would otherwise be missed by
    // the dependency checks above. Only the name is used (not `keywords`, which
    // is too broad), and this convention skip is scoped to `vite` (never
    // vitest/tsdown).
    PackageRewriteContext {
        skip_packages: SkipPackages {
            skip_vite: has_package("peerDependencies", "vite")
                || has_package("dependencies", "vite")
                || package_name_is_plugin(&pkg),
            skip_vitest: has_package("peerDependencies", "vitest")
                || has_package("dependencies", "vitest"),
            skip_tsdown: has_package("peerDependencies", "tsdown")
                || has_package("dependencies", "tsdown"),
        },
        uses_nuxt_test_utils: ["dependencies", "devDependencies", "optionalDependencies"]
            .into_iter()
            .any(|key| has_package(key, "@nuxt/test-utils")),
    }
}

#[cfg(test)]
fn get_skip_packages_from_package_json(package_json_path: &Path) -> SkipPackages {
    get_package_rewrite_context(package_json_path).skip_packages
}

/// Result of rewriting imports in a file
#[derive(Debug)]
struct RewriteResult {
    /// The updated file content
    pub content: String,
    /// Whether any changes were made
    pub updated: bool,
    /// Whether an upstream `vitest` specifier was intentionally preserved.
    pub preserved_vitest: bool,
}

/// Result of rewriting imports in multiple files
#[derive(Debug)]
pub struct BatchRewriteResult {
    /// Files that were modified
    pub modified_files: Vec<PathBuf>,
    /// Files that had no changes
    pub unchanged_files: Vec<PathBuf>,
    /// Files in Nuxt test-utils packages where upstream `vitest` imports were preserved.
    pub preserved_vitest_files: Vec<PathBuf>,
    /// Files that had errors (path, error message)
    pub errors: Vec<(PathBuf, String)>,
}

enum FileResult {
    Modified,
    Unchanged,
    Error(String),
}

/// Rewrite imports in all TypeScript/JavaScript files under a directory
///
/// This function finds all TypeScript and JavaScript files in the specified directory
/// (respecting `.gitignore` rules), applies the import rewrite rules to each file,
/// and writes the modified content back to disk.
///
/// # Arguments
///
/// * `root` - The root directory to search for files
///
/// # Returns
///
/// Returns a `BatchRewriteResult` containing:
/// - `modified_files`: Files that were changed
/// - `unchanged_files`: Files that required no changes
/// - `errors`: Files that had errors during processing
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use vite_migration::rewrite_imports_in_directory;
///
/// let result = rewrite_imports_in_directory(Path::new("./src"))?;
/// println!("Modified {} files", result.modified_files.len());
/// for file in &result.modified_files {
///     println!("  {}", file.display());
/// }
/// ```
pub fn rewrite_imports_in_directory(root: &Path) -> Result<BatchRewriteResult, Error> {
    rewrite_imports_in_directory_with_options(root, RewriteImportsOptions::default())
}

/// Rewrite imports with package-scoped compatibility options.
pub fn rewrite_imports_in_directory_with_options(
    root: &Path,
    options: RewriteImportsOptions,
) -> Result<BatchRewriteResult, Error> {
    let walk_result = file_walker::find_ts_files(root)?;

    // Pre-compute package context for each file (requires mutable cache, done sequentially).
    let mut package_context_cache: HashMap<PathBuf, PackageRewriteContext> = HashMap::new();
    let files_with_context: Vec<(PathBuf, PackageRewriteContext)> = walk_result
        .files
        .into_iter()
        .map(|file_path| {
            let package_context =
                if let Some(package_json_path) = find_nearest_package_json(&file_path, root) {
                    *package_context_cache
                        .entry(package_json_path.clone())
                        .or_insert_with(|| get_package_rewrite_context(&package_json_path))
                } else {
                    PackageRewriteContext::default()
                };
            (file_path, package_context)
        })
        .collect();

    // Process files in parallel using rayon
    let results: Vec<(PathBuf, FileResult, bool)> = files_with_context
        .into_par_iter()
        .map(|(file_path, package_context)| {
            let skip_packages = package_context.skip_packages;
            if skip_packages.all_skipped() {
                return (file_path, FileResult::Unchanged, false);
            }

            match rewrite_import(
                &file_path,
                &skip_packages,
                options.preserve_vitest_in_nuxt_packages && package_context.uses_nuxt_test_utils,
            ) {
                Ok(rewrite_result) => {
                    if rewrite_result.updated {
                        if let Err(e) = std::fs::write(&file_path, &rewrite_result.content) {
                            (file_path, FileResult::Error(e.to_string()), false)
                        } else {
                            (file_path, FileResult::Modified, rewrite_result.preserved_vitest)
                        }
                    } else {
                        (file_path, FileResult::Unchanged, rewrite_result.preserved_vitest)
                    }
                }
                Err(e) => (file_path, FileResult::Error(e.to_string()), false),
            }
        })
        .collect();

    // Collect results
    let mut batch_result = BatchRewriteResult {
        modified_files: Vec::new(),
        unchanged_files: Vec::new(),
        preserved_vitest_files: Vec::new(),
        errors: Vec::new(),
    };

    for (file_path, file_result, preserved_vitest) in results {
        if preserved_vitest {
            batch_result.preserved_vitest_files.push(file_path.clone());
        }
        match file_result {
            FileResult::Modified => batch_result.modified_files.push(file_path),
            FileResult::Unchanged => batch_result.unchanged_files.push(file_path),
            FileResult::Error(msg) => batch_result.errors.push((file_path, msg)),
        }
    }

    Ok(batch_result)
}

/// Rewrite imports in a TypeScript/JavaScript file from vite/vitest to vite-plus
///
/// This function reads a file and rewrites the import statements
/// to use 'vite-plus' instead of 'vite', 'vitest', or '@vitest/*'.
/// Packages that are in peerDependencies or dependencies will be skipped.
///
/// # Arguments
///
/// * `file_path` - Path to the TypeScript/JavaScript file
/// * `skip_packages` - Which packages to skip based on peerDependencies or dependencies
///
/// # Returns
///
/// Returns a `RewriteResult` containing:
/// - `content`: The updated file content
/// - `updated`: Whether any changes were made
fn rewrite_import(
    file_path: &Path,
    skip_packages: &SkipPackages,
    preserve_vitest_in_nuxt_package: bool,
) -> Result<RewriteResult, Error> {
    // Read the file
    let content = std::fs::read_to_string(file_path)?;

    // Rewrite the imports
    rewrite_import_content_with_options(&content, skip_packages, preserve_vitest_in_nuxt_package)
}

/// Fast pre-filter to skip expensive AST parsing for files with no relevant imports.
fn content_may_need_rewriting(content: &str, skip_packages: &SkipPackages) -> bool {
    // "vite" also matches "vitest" as a substring, covering both packages
    if (!skip_packages.skip_vite || !skip_packages.skip_vitest) && content.contains("vite") {
        return true;
    }
    // When only skip_vite is set, we still need to catch @vitest/ scoped packages
    if !skip_packages.skip_vitest && content.contains("@vitest/") {
        return true;
    }
    if !skip_packages.skip_tsdown && content.contains("tsdown") {
        return true;
    }
    false
}

/// Rewrite imports in content from vite/vitest to vite-plus
///
/// This is the internal function that performs the actual rewrite using ast-grep.
/// Packages that are in peerDependencies or dependencies will be skipped.
#[cfg(test)]
fn rewrite_import_content(
    content: &str,
    skip_packages: &SkipPackages,
) -> Result<RewriteResult, Error> {
    rewrite_import_content_with_options(content, skip_packages, false)
}

fn rewrite_import_content_with_options(
    content: &str,
    skip_packages: &SkipPackages,
    preserve_unscoped_vitest: bool,
) -> Result<RewriteResult, Error> {
    // Fast path: skip AST parsing if the file doesn't contain any target strings
    if !content_may_need_rewriting(content, skip_packages) {
        return Ok(RewriteResult {
            content: content.to_string(),
            updated: false,
            preserved_vitest: false,
        });
    }

    let mut new_content = content.to_string();
    let mut updated = false;
    let mut preserved_vitest = false;

    // Apply vite rules if not skipped (using pre-parsed rules)
    if !skip_packages.skip_vite {
        let vite_content = ast_grep::apply_loaded_rules(&new_content, &PARSED_VITE_RULES);
        if vite_content != new_content {
            new_content = vite_content;
            updated = true;
        }
    }

    // Apply vitest rules if not skipped (using pre-parsed rules)
    if !skip_packages.skip_vitest {
        let vitest_rules = if preserve_unscoped_vitest {
            let upstream_rewrite =
                ast_grep::apply_loaded_rules(&new_content, &PARSED_UNSCOPED_VITEST_RULES);
            preserved_vitest = upstream_rewrite != new_content;
            &*PARSED_VITEST_RULES_WITHOUT_UNSCOPED
        } else {
            &*PARSED_VITEST_RULES
        };
        let vitest_content = ast_grep::apply_loaded_rules(&new_content, vitest_rules);
        if vitest_content != new_content {
            new_content = vitest_content;
            updated = true;
        }
    }

    // Apply tsdown rules if not skipped (using pre-parsed rules)
    if !skip_packages.skip_tsdown {
        let tsdown_content = ast_grep::apply_loaded_rules(&new_content, &PARSED_TSDOWN_RULES);
        if tsdown_content != new_content {
            new_content = tsdown_content;
            updated = true;
        }
    }

    // Apply reference type rewriting (/// <reference types="..." />)
    // These cannot be handled by ast-grep because they are parsed as comments.
    updated |= rewrite_reference_types(&mut new_content, skip_packages, preserve_unscoped_vitest);

    Ok(RewriteResult { content: new_content, updated, preserved_vitest })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_rewrite_import_content_vite() {
        let vite_config = r#"import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus'

export default defineConfig({
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vite_double_quotes() {
        let vite_config = r#"import { defineConfig } from "vite";

export default defineConfig({
  plugins: [],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from "vite-plus";

export default defineConfig({
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_config() {
        let vite_config = r#"import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
  },
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    globals: true,
  },
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_multiple_imports() {
        let vite_config = r#"import { defineConfig, loadEnv, type UserWorkspaceConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig, loadEnv, type UserWorkspaceConfig } from 'vite-plus';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_already_vite_plus() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_rewrite_import_with_file() {
        // Create temporary directory (automatically cleaned up when dropped)
        let temp_dir = tempdir().unwrap();

        let vite_config_path = temp_dir.path().join("vite.config.ts");

        // Write test vite config
        let mut vite_file = std::fs::File::create(&vite_config_path).unwrap();
        write!(
            vite_file,
            r#"import {{ defineConfig }} from 'vite';

export default defineConfig({{
  plugins: [],
}});"#
        )
        .unwrap();

        // Run the rewrite
        let result = rewrite_import(&vite_config_path, &SkipPackages::default(), false).unwrap();

        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest() {
        let vite_config = r#"import { describe, it, expect } from 'vitest';

describe('test', () => {
  it('should work', () => {
    expect(true).toBe(true);
  });
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { describe, it, expect } from 'vite-plus/test';

describe('test', () => {
  it('should work', () => {
    expect(true).toBe(true);
  });
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_double_quotes() {
        let vite_config = r#"import { describe, it, expect } from "vitest";

describe('test', () => {});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { describe, it, expect } from "vite-plus/test";

describe('test', () => {});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser() {
        let vite_config = r#"import { page } from '@vitest/browser';

export default page;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { page } from 'vite-plus/test/browser';

export default page;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_double_quotes() {
        let vite_config = r#"import { page } from "@vitest/browser";

export default page;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { page } from "vite-plus/test/browser";

export default page;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_playwright() {
        let vite_config = r#"import { playwright } from '@vitest/browser-playwright';

export default playwright;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { playwright } from 'vite-plus/test/browser-playwright';

export default playwright;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_playwright_double_quotes() {
        let vite_config = r#"import { playwright } from "@vitest/browser-playwright";

export default playwright;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { playwright } from "vite-plus/test/browser-playwright";

export default playwright;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_subpath() {
        let vite_config = r#"import { context } from '@vitest/browser/context';

export default context;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { context } from 'vite-plus/test/browser/context';

export default context;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_playwright_subpath() {
        // `@vitest/browser-{provider}/context` maps onto the shared
        // `vite-plus/test/browser/context` export (the provider segment is dropped).
        let vite_config = r#"import { something } from "@vitest/browser-playwright/context";

export default something;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { something } from "vite-plus/test/browser/context";

export default something;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_playwright_provider() {
        // `@vitest/browser-{provider}/provider` maps to the provider entry point
        // under `vite-plus/test/browser/providers/{provider}`.
        let vite_config = r#"import { playwright } from '@vitest/browser-playwright/provider';

export default playwright;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { playwright } from 'vite-plus/test/browser/providers/playwright';

export default playwright;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_preview() {
        let vite_config = r#"import { preview } from '@vitest/browser-preview';

export default preview;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { preview } from 'vite-plus/test/browser-preview';

export default preview;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_preview_subpath() {
        let vite_config = r#"import { something } from "@vitest/browser-preview/context";

export default something;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { something } from "vite-plus/test/browser/context";

export default something;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_preview_provider() {
        let vite_config = r#"import { preview } from '@vitest/browser-preview/provider';

export default preview;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { preview } from 'vite-plus/test/browser/providers/preview';

export default preview;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_webdriverio() {
        let vite_config = r#"import { webdriverio } from '@vitest/browser-webdriverio';

export default webdriverio;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { webdriverio } from 'vite-plus/test/browser-webdriverio';

export default webdriverio;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_webdriverio_subpath() {
        let vite_config = r#"import { something } from "@vitest/browser-webdriverio/context";

export default something;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { something } from "vite-plus/test/browser/context";

export default something;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_webdriverio_provider() {
        let vite_config = r#"import { webdriverio } from '@vitest/browser-webdriverio/provider';

export default webdriverio;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { webdriverio } from 'vite-plus/test/browser/providers/webdriverio';

export default webdriverio;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_flat_subpaths() {
        // `@vitest/browser/{client,locators,matchers,utils}` are exposed by
        // vite-plus at the *bare* `./test/{name}` surface, so the `/browser/`
        // segment must be stripped (the nested `./test/browser/{name}` keys
        // do NOT exist in the exports map → ERR_PACKAGE_PATH_NOT_EXPORTED).
        for (sub, expected) in [
            ("client", "vite-plus/test/client"),
            ("locators", "vite-plus/test/locators"),
            ("matchers", "vite-plus/test/matchers"),
            ("utils", "vite-plus/test/utils"),
        ] {
            let single = format!("import x from '@vitest/browser/{sub}';");
            let result = rewrite_import_content(&single, &SkipPackages::default()).unwrap();
            assert!(result.updated, "single-quoted @vitest/browser/{sub} should be rewritten");
            assert_eq!(result.content, format!("import x from '{expected}';"));

            let double = format!("import x from \"@vitest/browser/{sub}\";");
            let result = rewrite_import_content(&double, &SkipPackages::default()).unwrap();
            assert!(result.updated, "double-quoted @vitest/browser/{sub} should be rewritten");
            assert_eq!(result.content, format!("import x from \"{expected}\";"));
        }
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_context_kept_nested() {
        // `@vitest/browser/context` keeps the nested path — `./test/browser/context`
        // IS exported.
        let vite_config = r#"import { context } from '@vitest/browser/context';"#;
        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"import { context } from 'vite-plus/test/browser/context';"#);
    }

    #[test]
    fn test_rewrite_import_content_vite_subpath() {
        let vite_config = r#"import { ModuleRunner } from 'vite/module-runner';

export default ModuleRunner;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { ModuleRunner } from 'vite-plus/module-runner';

export default ModuleRunner;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vite_subpath_double_quotes() {
        let vite_config = r#"import { ModuleRunner } from "vite/module-runner";

export default ModuleRunner;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { ModuleRunner } from "vite-plus/module-runner";

export default ModuleRunner;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_subpath() {
        // Test vitest/node subpath
        let vite_config = r#"import { startVitest } from 'vitest/node';

export default startVitest;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { startVitest } from 'vite-plus/test/node';

export default startVitest;"#
        );

        // Test vitest/plugins/runner subpath
        let vite_config = r#"import { somePlugin } from 'vitest/plugins/runner';

export default somePlugin;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { somePlugin } from 'vite-plus/test/plugins/runner';

export default somePlugin;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_subpath_double_quotes() {
        let vite_config = r#"import { startVitest } from "vitest/node";

export default startVitest;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { startVitest } from "vite-plus/test/node";

export default startVitest;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_mixed_imports() {
        // Test multiple different imports in the same file
        let vite_config = r#"import { defineConfig } from 'vite';
import { ModuleRunner } from 'vite/module-runner';
import { describe, it, expect } from 'vitest';
import { startVitest } from 'vitest/node';
import { page } from '@vitest/browser';
import { playwright } from '@vitest/browser-playwright';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';
import { ModuleRunner } from 'vite-plus/module-runner';
import { describe, it, expect } from 'vite-plus/test';
import { startVitest } from 'vite-plus/test/node';
import { page } from 'vite-plus/test/browser';
import { playwright } from 'vite-plus/test/browser-playwright';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#
        );
    }

    #[test]
    fn test_rewrite_imports_in_directory() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create src directory
        fs::create_dir(temp.path().join("src")).unwrap();

        // Create test files with vite/vitest imports
        fs::write(
            temp.path().join("src/config.ts"),
            r#"import { defineConfig } from 'vite';
export default defineConfig({});"#,
        )
        .unwrap();

        fs::write(
            temp.path().join("src/test.ts"),
            r#"import { describe, it } from 'vitest';
describe('test', () => {});"#,
        )
        .unwrap();

        // Create a file without vite imports (should be unchanged)
        fs::write(
            temp.path().join("src/utils.ts"),
            r#"export function add(a: number, b: number) {
  return a + b;
}"#,
        )
        .unwrap();

        // Create node_modules (should be ignored)
        fs::create_dir(temp.path().join("node_modules")).unwrap();
        fs::write(
            temp.path().join("node_modules/pkg.ts"),
            r#"import { defineConfig } from 'vite';"#,
        )
        .unwrap();

        // Create .gitignore
        fs::write(temp.path().join(".gitignore"), "node_modules/").unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // Should have 2 modified files (config.ts and test.ts)
        assert_eq!(result.modified_files.len(), 2);
        // Should have 1 unchanged file (utils.ts)
        assert_eq!(result.unchanged_files.len(), 1);
        // Should have no errors
        assert!(result.errors.is_empty());

        // Verify the files were actually modified
        let config_content = fs::read_to_string(temp.path().join("src/config.ts")).unwrap();
        assert!(config_content.contains("vite-plus"));

        let test_content = fs::read_to_string(temp.path().join("src/test.ts")).unwrap();
        assert!(test_content.contains("vite-plus/test"));

        // Verify utils.ts was not modified
        let utils_content = fs::read_to_string(temp.path().join("src/utils.ts")).unwrap();
        assert!(!utils_content.contains("vite-plus"));
    }

    #[test]
    fn test_preserves_unscoped_vitest_in_nuxt_test_utils_packages() {
        use std::fs;

        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{
  "devDependencies": {
    "@nuxt/test-utils": "4.0.3",
    "vitest": "4.1.9"
  }
}"#,
        )
        .unwrap();
        fs::write(
            temp.path().join("nuxt.spec.ts"),
            r#"import { vi } from 'vitest';
export { expect } from 'vitest';
const runtime = require('vitest');
const dynamic = import('vitest');
import { defineConfig } from 'vitest/config';
import { startVitest } from 'vitest/node';
import { page } from '@vitest/browser/context';
import { mockNuxtImport } from '@nuxt/test-utils/runtime';"#,
        )
        .unwrap();
        fs::write(
            temp.path().join("ordinary.spec.ts"),
            "/// <reference types=\"vitest/globals\" />\nimport { expect } from 'vitest';\n",
        )
        .unwrap();

        let result = rewrite_imports_in_directory_with_options(
            temp.path(),
            RewriteImportsOptions { preserve_vitest_in_nuxt_packages: true },
        )
        .unwrap();

        assert_eq!(result.preserved_vitest_files.len(), 2);
        assert!(result.preserved_vitest_files.contains(&temp.path().join("nuxt.spec.ts")));
        assert!(result.preserved_vitest_files.contains(&temp.path().join("ordinary.spec.ts")));
        let nuxt = fs::read_to_string(temp.path().join("nuxt.spec.ts")).unwrap();
        assert!(nuxt.contains("from 'vitest'"));
        assert!(nuxt.contains("require('vitest')"));
        assert!(nuxt.contains("import('vitest')"));
        assert!(nuxt.contains("from 'vitest/config'"));
        assert!(nuxt.contains("from 'vitest/node'"));
        assert!(nuxt.contains("from 'vite-plus/test/browser/context'"));

        let ordinary = fs::read_to_string(temp.path().join("ordinary.spec.ts")).unwrap();
        assert!(ordinary.contains("from 'vitest'"));
        assert!(ordinary.contains("types=\"vitest/globals\""));
    }

    #[test]
    fn test_nuxt_preservation_requires_declared_test_utils_dependency() {
        use std::fs;

        let temp = tempdir().unwrap();
        fs::write(temp.path().join("package.json"), r#"{"devDependencies":{"vitest":"4"}}"#)
            .unwrap();
        fs::write(
            temp.path().join("nuxt.spec.ts"),
            "import { vi } from 'vitest';\nimport { mockNuxtImport } from '@nuxt/test-utils/runtime';\n",
        )
        .unwrap();

        let result = rewrite_imports_in_directory_with_options(
            temp.path(),
            RewriteImportsOptions { preserve_vitest_in_nuxt_packages: true },
        )
        .unwrap();

        assert!(result.preserved_vitest_files.is_empty());
        let content = fs::read_to_string(temp.path().join("nuxt.spec.ts")).unwrap();
        assert!(content.contains("from 'vite-plus/test'"));
    }

    #[test]
    fn test_rewrite_imports_in_directory_empty() {
        let temp = tempdir().unwrap();

        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        assert!(result.modified_files.is_empty());
        assert!(result.unchanged_files.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_rewrite_imports_in_directory_nested() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create nested directory structure
        fs::create_dir_all(temp.path().join("src/components/Button")).unwrap();
        fs::create_dir_all(temp.path().join("tests/unit")).unwrap();

        // Create files at various depths
        fs::write(
            temp.path().join("vite.config.ts"),
            r#"import { defineConfig } from 'vite';
export default defineConfig({});"#,
        )
        .unwrap();

        fs::write(
            temp.path().join("src/index.ts"),
            r#"import { createServer } from 'vite';
export { createServer };"#,
        )
        .unwrap();

        fs::write(
            temp.path().join("src/components/Button/Button.tsx"),
            r#"import React from 'react';
export const Button = () => <button>Click</button>;"#,
        )
        .unwrap();

        fs::write(
            temp.path().join("tests/unit/app.test.ts"),
            r#"import { describe, it, expect } from 'vitest';
import { page } from '@vitest/browser';

describe('app', () => {
  it('works', () => {
    expect(true).toBe(true);
  });
});"#,
        )
        .unwrap();

        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // vite.config.ts, src/index.ts, tests/unit/app.test.ts should be modified
        assert_eq!(result.modified_files.len(), 3);
        // Button.tsx has no vite imports
        assert_eq!(result.unchanged_files.len(), 1);
        assert!(result.errors.is_empty());

        // Verify nested file was modified
        let test_content = fs::read_to_string(temp.path().join("tests/unit/app.test.ts")).unwrap();
        assert!(test_content.contains("vite-plus/test"));
        assert!(test_content.contains("vite-plus/test/browser"));
    }

    #[test]
    fn test_rewrite_declare_module_vite() {
        let content = r#"declare module 'vite' {
  interface UserConfig {
    runtimeEnv?: RuntimeEnvConfig;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus' {
  interface UserConfig {
    runtimeEnv?: RuntimeEnvConfig;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vite_double_quotes() {
        let content = r#"declare module "vite" {
  interface UserConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module "vite-plus" {
  interface UserConfig {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest() {
        // `declare module 'vitest'` is intentionally NOT rewritten — the
        // `vite-plus/test` subpath is a thin shim that re-exports vitest's
        // types, so augmentations must target the upstream module identity.
        let content = r#"declare module 'vitest' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_config() {
        // `declare module 'vitest/config'` is intentionally NOT rewritten —
        // `vite-plus` re-exports `vitest/config`, so augmentations must target
        // the upstream module identity to merge correctly.
        let content = r#"declare module 'vitest/config' {
  interface UserConfig {
    test?: TestConfig;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vite_subpath() {
        let content = r#"declare module 'vite/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_subpath() {
        // `declare module 'vitest/node'` stays — the `vite-plus/test/node`
        // shim re-exports from upstream, so augmentations must target the
        // upstream module identity.
        let content = r#"declare module 'vitest/node' {
  export interface VitestOptions {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser() {
        // `declare module '@vitest/browser'` stays — the
        // `vite-plus/test/browser` shim re-exports from upstream.
        let content = r#"declare module '@vitest/browser' {
  interface BrowserContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_subpath() {
        // `declare module '@vitest/browser/context'` stays — shim re-exports.
        let content = r#"declare module '@vitest/browser/context' {
  export interface Context {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_playwright() {
        // `declare module '@vitest/browser-playwright'` stays — shim re-exports.
        let content = r#"declare module '@vitest/browser-playwright' {
  interface PlaywrightContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_preview() {
        // `declare module '@vitest/browser-preview'` stays — shim re-exports.
        let content = r#"declare module '@vitest/browser-preview' {
  interface PreviewContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_webdriverio() {
        // `declare module '@vitest/browser-webdriverio'` stays — shim re-exports.
        let content = r#"declare module '@vitest/browser-webdriverio' {
  interface WebDriverContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_mixed_imports_and_declare_modules() {
        // Imports are rewritten; `declare module 'vite' { ... }` is rewritten
        // (vite-plus bundles vite, owning the type identity), but
        // `declare module 'vitest' { ... }` stays put — the shim re-exports
        // upstream and augmentations must target the upstream identity.
        let content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

declare module 'vite' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module 'vitest' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

export default defineConfig({});"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';
import { describe } from 'vite-plus/test';

declare module 'vite-plus' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module 'vitest' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_already_vite_plus() {
        let content = r#"declare module 'vite-plus' {
  interface UserConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_multiple_declare_modules() {
        // `declare module 'vite'` / `'vite/<sub>'` get rewritten (vite-plus
        // bundles vite, owning the type identity), but `declare module 'vitest'`
        // and `declare module '@vitest/browser'` are preserved — vite-plus
        // shims re-export upstream and augmentations must target upstream.
        let content = r#"declare module 'vite' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module 'vite/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}

declare module 'vitest' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

declare module '@vitest/browser' {
  interface BrowserContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module 'vite-plus/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}

declare module 'vitest' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

declare module '@vitest/browser' {
  interface BrowserContext {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_double_quotes() {
        // Double-quoted `declare module "vitest"` is preserved for the same
        // reason as the single-quoted variant — shim re-exports.
        let content = r#"declare module "vitest" {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_playwright_subpath() {
        let content = r#"declare module '@vitest/browser-playwright/context' {
  export interface Context {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_preview_subpath() {
        let content = r#"declare module '@vitest/browser-preview/context' {
  export interface Context {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_webdriverio_subpath() {
        let content = r#"declare module '@vitest/browser-webdriverio/context' {
  export interface Context {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_declare_module_complex_interface() {
        let content = r#"declare module 'vite' {
  interface UserConfig {
    /**
     * Options for vite-plugin-runtime-env
     */
    runtimeEnv?: RuntimeEnvConfig;
    /**
     * Options for vite-plugin-runtime-html
     */
    runtimeHtml?: RuntimeHtmlConfig;
  }

  interface Plugin {
    name: string;
    configResolved?: (config: ResolvedConfig) => void;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus' {
  interface UserConfig {
    /**
     * Options for vite-plugin-runtime-env
     */
    runtimeEnv?: RuntimeEnvConfig;
    /**
     * Options for vite-plugin-runtime-html
     */
    runtimeHtml?: RuntimeHtmlConfig;
  }

  interface Plugin {
    name: string;
    configResolved?: (config: ResolvedConfig) => void;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_import_content_tsdown() {
        let tsdown_config = r#"import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: 'src/index.ts',
});"#;

        let result = rewrite_import_content(tsdown_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: 'src/index.ts',
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_tsdown_double_quotes() {
        let tsdown_config = r#"import { defineConfig } from "tsdown";

export default defineConfig({
  entry: "src/index.ts",
});"#;

        let result = rewrite_import_content(tsdown_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from "vite-plus/pack";

export default defineConfig({
  entry: "src/index.ts",
});"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_tsdown() {
        let content = r#"declare module 'tsdown' {
  interface BuildConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/pack' {
  interface BuildConfig {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_tsdown_double_quotes() {
        let content = r#"declare module "tsdown" {
  interface BuildConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module "vite-plus/pack" {
  interface BuildConfig {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_import_content_tsdown_client() {
        let content = r#"import 'tsdown/client';"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"import 'vite-plus/pack/client';"#);
    }

    #[test]
    fn test_rewrite_import_content_tsdown_client_double_quotes() {
        let content = r#"import "tsdown/client";"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"import "vite-plus/pack/client";"#);
    }

    #[test]
    fn test_rewrite_declare_module_tsdown_client() {
        let content = r#"declare module 'tsdown/client' {
  interface ClientConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/pack/client' {
  interface ClientConfig {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_tsdown_client_double_quotes() {
        let content = r#"declare module "tsdown/client" {
  interface ClientConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module "vite-plus/pack/client" {
  interface ClientConfig {
    custom?: boolean;
  }
}"#
        );
    }

    // ========================
    // PeerDependencies Tests
    // ========================

    #[test]
    fn test_skip_vite_when_peer_dependency() {
        // When vite is a peerDependency, vite imports should NOT be rewritten
        let content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

export default defineConfig({});"#;

        let skip_packages =
            SkipPackages { skip_vite: true, skip_vitest: false, skip_tsdown: false };

        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        // vite import should NOT be rewritten, vitest import SHOULD be rewritten
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite';
import { describe } from 'vite-plus/test';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_skip_vitest_when_peer_dependency() {
        // When vitest is a peerDependency, vitest imports should NOT be rewritten
        let content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

export default defineConfig({});"#;

        let skip_packages =
            SkipPackages { skip_vite: false, skip_vitest: true, skip_tsdown: false };

        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        // vite import SHOULD be rewritten, vitest import should NOT be rewritten
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';
import { describe } from 'vitest';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_skip_all_when_all_peer_dependencies() {
        // When all packages are peerDependencies, nothing should be rewritten
        let content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';
import { build } from 'tsdown';

export default defineConfig({});"#;

        let skip_packages = SkipPackages { skip_vite: true, skip_vitest: true, skip_tsdown: true };

        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_skip_packages_all_skipped() {
        let skip_all = SkipPackages { skip_vite: true, skip_vitest: true, skip_tsdown: true };
        assert!(skip_all.all_skipped());

        let skip_some = SkipPackages { skip_vite: true, skip_vitest: false, skip_tsdown: true };
        assert!(!skip_some.all_skipped());

        let skip_none = SkipPackages::default();
        assert!(!skip_none.all_skipped());
    }

    #[test]
    fn test_get_skip_packages_from_package_json_with_vite_peer_dep() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create package.json with vite as peerDependency
        let pkg_json = r#"{
  "name": "my-vite-plugin",
  "peerDependencies": {
    "vite": "^5.0.0"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_get_skip_packages_from_package_json_with_all_peer_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        let pkg_json = r#"{
  "name": "my-plugin",
  "peerDependencies": {
    "vite": "^5.0.0",
    "vitest": "^1.0.0",
    "tsdown": "^1.0.0"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite);
        assert!(skip.skip_vitest);
        assert!(skip.skip_tsdown);
        assert!(skip.all_skipped());
    }

    #[test]
    fn test_get_skip_packages_from_package_json_with_vite_dependency() {
        use std::fs;

        let temp = tempdir().unwrap();

        // vite in dependencies should also skip rewriting
        let pkg_json = r#"{
  "name": "my-app",
  "dependencies": {
    "vite": "^5.0.0"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite); // NOW skips because vite is in dependencies
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_get_skip_packages_from_package_json_no_file() {
        let temp = tempdir().unwrap();

        // No package.json created - should return default (no skipping)
        let package_json_path = temp.path().join("package.json");
        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_get_skip_packages_from_package_json_no_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Package with no dependencies at all
        let pkg_json = r#"{
  "name": "my-app"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_get_skip_packages_mixed_peer_and_regular_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // vite in dependencies, vitest in peerDependencies
        let pkg_json = r#"{
  "name": "my-package",
  "dependencies": {
    "vite": "^5.0.0"
  },
  "peerDependencies": {
    "vitest": "^1.0.0"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite); // in dependencies
        assert!(skip.skip_vitest); // in peerDependencies
        assert!(!skip.skip_tsdown);
    }

    // ===================================
    // Vite plugin name-convention tests
    // ===================================

    #[test]
    fn test_skip_vite_when_scoped_vite_plugin_name_with_vite_devdep() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Real-world case: @nkzw/vite-plugin-remdx declares vite only in
        // devDependencies, yet is a Vite plugin consumed by plain-vite projects.
        let pkg_json = r#"{
  "name": "@nkzw/vite-plugin-remdx",
  "devDependencies": {
    "vite": "catalog:"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite); // skipped by the vite-plugin- name convention
        // Scope check: name convention only affects skip_vite.
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_skip_vite_when_unscoped_vite_plugin_name() {
        use std::fs;

        let temp = tempdir().unwrap();

        let pkg_json = r#"{
  "name": "vite-plugin-foo"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite);
        // Scope check: name convention only affects skip_vite.
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_no_skip_vite_for_app_with_vite_devdep() {
        use std::fs;

        let temp = tempdir().unwrap();

        // A regular app declaring vite as a devDependency should still be
        // rewritten; devDependencies alone never trigger the skip.
        let pkg_json = r#"{
  "name": "my-app",
  "private": true,
  "devDependencies": {
    "vite": "^7"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_no_skip_vite_for_scoped_non_plugin_name() {
        use std::fs;

        let temp = tempdir().unwrap();

        let pkg_json = r#"{
  "name": "@scope/not-a-plugin"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_no_skip_vite_for_vite_plugin_without_trailing_hyphen() {
        use std::fs;

        let temp = tempdir().unwrap();

        // "vite-plugin" without the trailing hyphen is not the plugin convention.
        let pkg_json = r#"{
  "name": "vite-plugin"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_no_skip_vite_for_vite_plugins_name() {
        use std::fs;

        let temp = tempdir().unwrap();

        // "vite-plugins" (plural, no trailing hyphen) is not the plugin convention.
        let pkg_json = r#"{
  "name": "vite-plugins"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_skip_vite_when_unscoped_unplugin_name() {
        use std::fs;

        let temp = tempdir().unwrap();

        // unplugin libraries (https://unplugin.unjs.io) ship a Vite entry and are
        // consumed by vite projects, so their `vite` imports must be preserved
        // even when `vite` is only a devDependency.
        let pkg_json = r#"{
  "name": "unplugin-icons",
  "devDependencies": {
    "vite": "^7"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite);
        // Scope check: name convention only affects skip_vite.
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_skip_vite_when_scoped_unplugin_name() {
        use std::fs;

        let temp = tempdir().unwrap();

        let pkg_json = r#"{
  "name": "@scope/unplugin-foo"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_no_skip_vite_for_unplugin_without_trailing_hyphen() {
        use std::fs;

        let temp = tempdir().unwrap();

        // "unplugin" without the trailing hyphen is not the plugin convention.
        let pkg_json = r#"{
  "name": "unplugin"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_no_skip_vite_for_unplugins_name() {
        use std::fs;

        let temp = tempdir().unwrap();

        // "unplugins" (plural, no trailing hyphen) is not the plugin convention.
        let pkg_json = r#"{
  "name": "unplugins"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_skip_vite_preserves_plugin_import_by_name_convention() {
        // A Vite plugin library keeps importing from 'vite' so plain-vite
        // consumers are not broken; without the skip it is rewritten.
        let content = "import { Plugin } from 'vite';";

        let preserved = SkipPackages { skip_vite: true, skip_vitest: false, skip_tsdown: false };
        let result = rewrite_import_content(content, &preserved).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, "import { Plugin } from 'vite';");

        let rewritten =
            SkipPackages { skip_vite: false, skip_vitest: false, skip_tsdown: false };
        let result = rewrite_import_content(content, &rewritten).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, "import { Plugin } from 'vite-plus';");
    }

    #[test]
    fn test_rewrite_imports_in_directory_with_vite_dependency() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create package.json with vite as dependency (not peerDependency)
        let pkg_json = r#"{
  "name": "my-app",
  "dependencies": {
    "vite": "^5.0.0"
  }
}"#;
        fs::write(temp.path().join("package.json"), pkg_json).unwrap();

        // Create src directory
        fs::create_dir(temp.path().join("src")).unwrap();

        // Create source file with vite and vitest imports
        let original_content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

export default defineConfig({});"#;
        fs::write(temp.path().join("src/index.ts"), original_content).unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // File should be modified (vitest was rewritten)
        assert_eq!(result.modified_files.len(), 1);
        assert!(result.errors.is_empty());

        // Verify vite import NOT rewritten (in dependencies), vitest IS rewritten
        let content = fs::read_to_string(temp.path().join("src/index.ts")).unwrap();
        assert_eq!(
            content,
            r#"import { defineConfig } from 'vite';
import { describe } from 'vite-plus/test';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_imports_in_directory_with_peer_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create package.json with vite as peerDependency
        let pkg_json = r#"{
  "name": "my-vite-plugin",
  "peerDependencies": {
    "vite": "^5.0.0"
  }
}"#;
        fs::write(temp.path().join("package.json"), pkg_json).unwrap();

        // Create src directory
        fs::create_dir(temp.path().join("src")).unwrap();

        // Create source file with vite and vitest imports
        let original_content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

export default defineConfig({});"#;
        fs::write(temp.path().join("src/index.ts"), original_content).unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // File should be modified (vitest was rewritten)
        assert_eq!(result.modified_files.len(), 1);
        assert!(result.errors.is_empty());

        // Verify vite import NOT rewritten, vitest IS rewritten
        let content = fs::read_to_string(temp.path().join("src/index.ts")).unwrap();
        assert_eq!(
            content,
            r#"import { defineConfig } from 'vite';
import { describe } from 'vite-plus/test';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_imports_skips_file_when_all_peer_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create package.json with all packages as peerDependencies
        let pkg_json = r#"{
  "name": "my-plugin",
  "peerDependencies": {
    "vite": "^5.0.0",
    "vitest": "^1.0.0",
    "tsdown": "^1.0.0"
  }
}"#;
        fs::write(temp.path().join("package.json"), pkg_json).unwrap();

        // Create source file
        let original_content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';
import { build } from 'tsdown';"#;
        fs::write(temp.path().join("index.ts"), original_content).unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // File should be unchanged (all skipped)
        assert!(result.modified_files.is_empty());
        assert_eq!(result.unchanged_files.len(), 1);

        // Verify content unchanged
        let content = fs::read_to_string(temp.path().join("index.ts")).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    fn test_find_nearest_package_json() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create monorepo structure
        fs::create_dir_all(temp.path().join("packages/vite-plugin/src")).unwrap();
        fs::create_dir_all(temp.path().join("packages/app/src")).unwrap();

        // Root package.json (no peerDeps)
        fs::write(temp.path().join("package.json"), r#"{"name": "monorepo"}"#).unwrap();

        // vite-plugin package.json (has vite in peerDeps)
        fs::write(
            temp.path().join("packages/vite-plugin/package.json"),
            r#"{"name": "vite-plugin", "peerDependencies": {"vite": "^5.0.0"}}"#,
        )
        .unwrap();

        // app package.json (no peerDeps)
        fs::write(temp.path().join("packages/app/package.json"), r#"{"name": "app"}"#).unwrap();

        // Test finding package.json from vite-plugin/src/index.ts
        let file_path = temp.path().join("packages/vite-plugin/src/index.ts");
        let result = find_nearest_package_json(&file_path, temp.path());
        assert_eq!(result, Some(temp.path().join("packages/vite-plugin/package.json")));

        // Test finding package.json from app/src/index.ts
        let file_path = temp.path().join("packages/app/src/index.ts");
        let result = find_nearest_package_json(&file_path, temp.path());
        assert_eq!(result, Some(temp.path().join("packages/app/package.json")));

        // Test finding package.json from root level file
        let file_path = temp.path().join("vite.config.ts");
        let result = find_nearest_package_json(&file_path, temp.path());
        assert_eq!(result, Some(temp.path().join("package.json")));
    }

    #[test]
    fn test_rewrite_imports_monorepo_different_peer_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create monorepo structure
        fs::create_dir_all(temp.path().join("packages/vite-plugin/src")).unwrap();
        fs::create_dir_all(temp.path().join("packages/app/src")).unwrap();

        // Root package.json (no peerDeps)
        fs::write(temp.path().join("package.json"), r#"{"name": "monorepo"}"#).unwrap();

        // vite-plugin package.json (has vite in peerDeps)
        fs::write(
            temp.path().join("packages/vite-plugin/package.json"),
            r#"{"name": "vite-plugin", "peerDependencies": {"vite": "^5.0.0"}}"#,
        )
        .unwrap();

        // app package.json (no peerDeps)
        fs::write(temp.path().join("packages/app/package.json"), r#"{"name": "app"}"#).unwrap();

        // vite-plugin source file with vite and vitest imports
        fs::write(
            temp.path().join("packages/vite-plugin/src/index.ts"),
            r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';
export default defineConfig({});"#,
        )
        .unwrap();

        // app source file with vite and vitest imports
        fs::write(
            temp.path().join("packages/app/src/index.ts"),
            r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';
export default defineConfig({});"#,
        )
        .unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // Both files should be modified
        assert_eq!(result.modified_files.len(), 2);

        // vite-plugin: vite NOT rewritten (has peerDep), vitest IS rewritten
        let vite_plugin_content =
            fs::read_to_string(temp.path().join("packages/vite-plugin/src/index.ts")).unwrap();
        assert_eq!(
            vite_plugin_content,
            r#"import { defineConfig } from 'vite';
import { describe } from 'vite-plus/test';
export default defineConfig({});"#
        );

        // app: vite IS rewritten (no peerDep), vitest IS rewritten
        let app_content =
            fs::read_to_string(temp.path().join("packages/app/src/index.ts")).unwrap();
        assert_eq!(
            app_content,
            r#"import { defineConfig } from 'vite-plus';
import { describe } from 'vite-plus/test';
export default defineConfig({});"#
        );
    }

    // ====================================
    // CommonJS `require(...)` Rewriting Tests
    // ====================================
    //
    // These tests cover the require-shape sibling rules that mirror the
    // `import_statement` rewrites for `.cjs` / `.cts` and other CJS sources.
    // The rules MUST only match a literal `require(...)` callee — not
    // `my_require(...)`, not `require.cache[...]`, and not dynamic
    // `import('...')` (the import_statement rules already cover ESM).

    #[test]
    fn test_rewrite_require_vite() {
        let content = r#"const { defineConfig } = require('vite');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const { defineConfig } = require('vite-plus');"#);
    }

    #[test]
    fn test_rewrite_require_vite_double_quotes() {
        let content = r#"const { defineConfig } = require("vite");"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const { defineConfig } = require("vite-plus");"#);
    }

    #[test]
    fn test_rewrite_require_vite_subpath() {
        let content = r#"const { ModuleRunner } = require('vite/module-runner');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const { ModuleRunner } = require('vite-plus/module-runner');"#
        );
    }

    #[test]
    fn test_rewrite_require_vitest() {
        let content = r#"const vi = require('vitest');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const vi = require('vite-plus/test');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_config() {
        let content = r#"const { defineConfig } = require('vitest/config');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const { defineConfig } = require('vite-plus');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_subpath() {
        let content = r#"const { startVitest } = require('vitest/node');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const { startVitest } = require('vite-plus/test/node');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_browser() {
        let content = r#"const x = require('@vitest/browser');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const x = require('vite-plus/test/browser');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_browser_context() {
        let content = r#"const { context } = require('@vitest/browser/context');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const { context } = require('vite-plus/test/browser/context');"#
        );
    }

    #[test]
    fn test_rewrite_require_vitest_browser_flat_subpaths() {
        // `@vitest/browser/{client,locators,matchers,utils}` get the `/browser/`
        // segment stripped, mirroring the import_statement rule.
        for (sub, expected) in [
            ("client", "vite-plus/test/client"),
            ("locators", "vite-plus/test/locators"),
            ("matchers", "vite-plus/test/matchers"),
            ("utils", "vite-plus/test/utils"),
        ] {
            let single = format!("const x = require('@vitest/browser/{sub}');");
            let result = rewrite_import_content(&single, &SkipPackages::default()).unwrap();
            assert!(result.updated, "single-quoted require @vitest/browser/{sub} should rewrite");
            assert_eq!(result.content, format!("const x = require('{expected}');"));

            let double = format!("const x = require(\"@vitest/browser/{sub}\");");
            let result = rewrite_import_content(&double, &SkipPackages::default()).unwrap();
            assert!(result.updated, "double-quoted require @vitest/browser/{sub} should rewrite");
            assert_eq!(result.content, format!("const x = require(\"{expected}\");"));
        }
    }

    #[test]
    fn test_rewrite_require_vitest_browser_playwright() {
        let content = r#"const x = require('@vitest/browser-playwright');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const x = require('vite-plus/test/browser-playwright');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_browser_playwright_context() {
        let content = r#"const x = require('@vitest/browser-playwright/context');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const x = require('vite-plus/test/browser/context');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_browser_playwright_provider() {
        let content = r#"const x = require('@vitest/browser-playwright/provider');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const x = require('vite-plus/test/browser/providers/playwright');"#
        );
    }

    #[test]
    fn test_rewrite_require_vitest_browser_preview() {
        let content = r#"const x = require('@vitest/browser-preview');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const x = require('vite-plus/test/browser-preview');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_browser_preview_context() {
        let content = r#"const x = require('@vitest/browser-preview/context');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const x = require('vite-plus/test/browser/context');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_browser_preview_provider() {
        let content = r#"const x = require('@vitest/browser-preview/provider');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const x = require('vite-plus/test/browser/providers/preview');"#
        );
    }

    #[test]
    fn test_rewrite_require_vitest_browser_webdriverio() {
        let content = r#"const x = require('@vitest/browser-webdriverio');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const x = require('vite-plus/test/browser-webdriverio');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_browser_webdriverio_context() {
        let content = r#"const x = require('@vitest/browser-webdriverio/context');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const x = require('vite-plus/test/browser/context');"#);
    }

    #[test]
    fn test_rewrite_require_vitest_browser_webdriverio_provider() {
        let content = r#"const x = require('@vitest/browser-webdriverio/provider');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const x = require('vite-plus/test/browser/providers/webdriverio');"#
        );
    }

    #[test]
    fn test_rewrite_require_tsdown() {
        let content = r#"const { defineConfig } = require('tsdown');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const { defineConfig } = require('vite-plus/pack');"#);
    }

    #[test]
    fn test_rewrite_require_tsdown_client() {
        let content = r#"require('tsdown/client');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"require('vite-plus/pack/client');"#);
    }

    #[test]
    fn test_rewrite_require_mixed_in_cjs_config() {
        // Mirrors a realistic `vitest.config.cjs` containing both the config
        // import and direct test specifier requires.
        let content = r#"const { defineConfig } = require('vitest/config');
const vi = require('vitest');
const { context } = require('@vitest/browser/context');
const { defineConfig: defineViteConfig } = require('vite');
const { build } = require('tsdown');

module.exports = defineConfig({});"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const { defineConfig } = require('vite-plus');
const vi = require('vite-plus/test');
const { context } = require('vite-plus/test/browser/context');
const { defineConfig: defineViteConfig } = require('vite-plus');
const { build } = require('vite-plus/pack');

module.exports = defineConfig({});"#
        );
    }

    // -- Negative tests: require-shape rules must NOT match these --

    #[test]
    fn test_rewrite_require_does_not_match_custom_require_function() {
        // `my_require('vitest')` is NOT a literal `require` callee, so the
        // require-shape rules must leave it alone. (The string `'vitest'` is
        // also outside an import_statement / module / require call, so the
        // import-shape rules can't see it either.)
        let content = r#"const x = my_require('vitest');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_require_does_not_match_require_cache_index() {
        // `require.cache['vitest']` is a member access + index — there is no
        // `require(...)` call here, so the require-shape rules must not match.
        let content = r#"const x = require.cache['vitest'];"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_require_does_not_match_require_resolve() {
        // `require.resolve('vitest')` is a method call on `require`, not a
        // bare `require(...)` call — the rule constrains the callee to the
        // literal identifier `require`, so this is left alone.
        let content = r#"const x = require.resolve('vitest');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_require_respects_skip_vitest_peer_dependency() {
        // When vitest is in peerDependencies, the require-shape rule must
        // also be skipped (parity with the import-shape rule).
        let content = r#"const vi = require('vitest');
const { defineConfig } = require('vite');"#;
        let skip_packages =
            SkipPackages { skip_vite: false, skip_vitest: true, skip_tsdown: false };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        // vitest require is NOT rewritten; vite require IS rewritten.
        assert_eq!(
            result.content,
            r#"const vi = require('vitest');
const { defineConfig } = require('vite-plus');"#
        );
    }

    // ====================================
    // Reference Types Rewriting Tests
    // ====================================

    #[test]
    fn test_rewrite_reference_types_vite_client() {
        let content = r#"/// <reference types="vite/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/client" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vite_client_single_quotes() {
        let content = r#"/// <reference types='vite/client' />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types='vite-plus/client' />"#);
    }

    #[test]
    fn test_rewrite_reference_types_bare_vite() {
        let content = r#"/// <reference types="vite" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_bare_vitest() {
        let content = r#"/// <reference types="vitest" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/test" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_globals() {
        let content = r#"/// <reference types="vitest/globals" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/test/globals" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_config() {
        let content = r#"/// <reference types="vitest/config" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_browser() {
        let content = r#"/// <reference types="vitest/browser" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/test/browser" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_flat_subpaths() {
        // `@vitest/browser/{client,locators,matchers,utils}` are exposed at the
        // *bare* `vite-plus/test/{name}` surface — the `/browser/` segment is stripped.
        for (sub, expected) in [
            ("client", "vite-plus/test/client"),
            ("locators", "vite-plus/test/locators"),
            ("matchers", "vite-plus/test/matchers"),
            ("utils", "vite-plus/test/utils"),
        ] {
            let content = format!(r#"/// <reference types="@vitest/browser/{sub}" />"#);
            let result = rewrite_import_content(&content, &SkipPackages::default()).unwrap();
            assert!(result.updated, "@vitest/browser/{sub} reference should be rewritten");
            assert_eq!(result.content, format!(r#"/// <reference types="{expected}" />"#));
        }
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_context() {
        let content = r#"/// <reference types="@vitest/browser/context" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/test/browser/context" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_playwright() {
        let content = r#"/// <reference types="@vitest/browser/providers/playwright" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/test/browser/providers/playwright" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_playwright_pkg() {
        let content = r#"/// <reference types="@vitest/browser-playwright" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/test/browser-playwright" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_webdriverio() {
        let content = r#"/// <reference types="@vitest/browser/providers/webdriverio" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/test/browser/providers/webdriverio" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_provider_context() {
        // `@vitest/browser-{provider}/context` references map onto the shared
        // `vite-plus/test/browser/context` export (the provider segment is dropped).
        for provider in ["playwright", "preview", "webdriverio"] {
            let content =
                format!(r#"/// <reference types="@vitest/browser-{provider}/context" />"#);
            let result = rewrite_import_content(&content, &SkipPackages::default()).unwrap();
            assert!(result.updated, "@vitest/browser-{provider}/context should be rewritten");
            assert_eq!(
                result.content,
                r#"/// <reference types="vite-plus/test/browser/context" />"#
            );
        }
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_provider_entry() {
        // `@vitest/browser-{provider}/provider` references map to the provider
        // entry point under `vite-plus/test/browser/providers/{provider}`.
        for provider in ["playwright", "preview", "webdriverio"] {
            let content =
                format!(r#"/// <reference types="@vitest/browser-{provider}/provider" />"#);
            let result = rewrite_import_content(&content, &SkipPackages::default()).unwrap();
            assert!(result.updated, "@vitest/browser-{provider}/provider should be rewritten");
            assert_eq!(
                result.content,
                format!(r#"/// <reference types="vite-plus/test/browser/providers/{provider}" />"#)
            );
        }
    }

    #[test]
    fn test_rewrite_reference_types_tsdown_client_rewritten() {
        // tsdown/client should be rewritten to vite-plus/pack/client
        let content = r#"/// <reference types="tsdown/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/pack/client" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_not_matching() {
        // Non-enumerated @vitest/* packages should NOT be rewritten
        let content = r#"/// <reference types="@vitest/coverage-v8" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_inside_template_literal_not_rewritten() {
        // Reference-like content inside template literals should NOT be rewritten.
        // The preamble ends at the first non-comment line (`const`), so nothing is processed.
        let content = r#"const template = `
/// <reference types="vite/client" />
`;"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_preamble_only() {
        // Only references in the preamble (before first statement) should be rewritten
        let content = r#"/// <reference types="vite/client" />
// A regular comment

const x = 1;
/// <reference types="vitest" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        // First reference (preamble) is rewritten; last one (after code) is not
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/client" />
// A regular comment

const x = 1;
/// <reference types="vitest" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_after_block_comment() {
        // Block comments (/* ... */) should not end the preamble scan
        let content = "/* License: MIT */\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* License: MIT */\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_after_multiline_block_comment() {
        // Multi-line block comments should be skipped entirely
        let content =
            "/*\n * License header\n * Copyright 2024\n */\n/// <reference types=\"vitest\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/*\n * License header\n * Copyright 2024\n */\n/// <reference types=\"vite-plus/test\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_block_comment_with_trailing_code() {
        // A single-line block comment followed by code should end the preamble
        let content = "/* banner */ const x = 1;\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_block_comment_with_trailing_comment() {
        // A block comment followed by a line comment is still preamble
        let content = "/* banner */ // generated\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* banner */ // generated\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_multiline_block_comment_closes_into_code() {
        // Multi-line block comment closing line has code after */ — end of preamble
        let content = "/*\n * License\n */ const x = 1;\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_multiline_block_comment_closes_into_comment() {
        // Multi-line block comment closing line has only a comment after */ — still preamble
        let content = "/*\n * License\n */ // end\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/*\n * License\n */ // end\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_block_close_into_new_block_comment() {
        // `/* a */ /* b` closes first comment then opens a new multi-line one
        let content = "/* a */ /* b\n * still going */\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* a */ /* b\n * still going */\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_multiple_inline_block_comments_then_code() {
        // `/* a */ /* b */ const x = 1;` — code after two closed block comments ends preamble
        let content = "/* a */ /* b */ const x = 1;\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_multiple_inline_block_comments_no_code() {
        // `/* a */ /* b */` — only block comments, no trailing code, preamble continues
        let content = "/* a */ /* b */\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* a */ /* b */\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_vitest_browser_providers_playwright() {
        // @vitest/browser/providers/playwright → vite-plus/test/browser/providers/playwright
        let content = r#"/// <reference types="@vitest/browser/providers/playwright" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/test/browser/providers/playwright" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_crlf() {
        // CRLF line endings should be preserved
        let content =
            "/// <reference types=\"vite/client\" />\r\n/// <reference types=\"vitest\" />\r\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/// <reference types=\"vite-plus/client\" />\r\n/// <reference types=\"vite-plus/test\" />\r\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_crlf_with_block_comment() {
        // CRLF + block comment header
        let content =
            "/* License */\r\n/// <reference types=\"vite/client\" />\r\nconst x = 1;\r\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* License */\r\n/// <reference types=\"vite-plus/client\" />\r\nconst x = 1;\r\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_no_space_after_slashes() {
        // TypeScript accepts ///<reference without a space
        let content = r#"///<reference types="vite/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"///<reference types="vite-plus/client" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_tab_after_slashes() {
        // TypeScript accepts ///\t<reference with a tab
        let content = "///\t<reference types=\"vite/client\" />";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, "///\t<reference types=\"vite-plus/client\" />");
    }

    #[test]
    fn test_rewrite_reference_types_after_shebang() {
        // Shebang lines should not end the preamble scan
        let content = "#!/usr/bin/env node\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "#!/usr/bin/env node\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_after_bom() {
        // UTF-8 BOM should not end the preamble scan; BOM is stripped during rewrite
        let content = "\u{feff}/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, "/// <reference types=\"vite-plus/client\" />\n");
    }

    #[test]
    fn test_rewrite_reference_types_bare_tsdown() {
        let content = r#"/// <reference types="tsdown" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/pack" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_already_migrated() {
        let content = r#"/// <reference types="vite-plus/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_preserves_non_matching() {
        let content = r#"/// <reference types="node" />
/// <reference lib="es2015" />
/// <reference path="./types.d.ts" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_with_leading_whitespace() {
        let content = r#"  /// <reference types="vite/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"  /// <reference types="vite-plus/client" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_env_d_ts_style() {
        let content = r#"/// <reference types="vite/client" />
/// <reference types="vitest" />
/// <reference types="vitest/globals" />
/// <reference types="vitest/config" />
/// <reference types="@vitest/browser/context" />"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/client" />
/// <reference types="vite-plus/test" />
/// <reference types="vite-plus/test/globals" />
/// <reference types="vite-plus" />
/// <reference types="vite-plus/test/browser/context" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_mixed_with_imports() {
        let content = r#"/// <reference types="vite/client" />
import { defineConfig } from 'vite';

export default defineConfig({});"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/client" />
import { defineConfig } from 'vite-plus';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_skip_vite() {
        let content = r#"/// <reference types="vite/client" />
/// <reference types="vitest" />"#;

        let skip_packages =
            SkipPackages { skip_vite: true, skip_vitest: false, skip_tsdown: false };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite/client" />
/// <reference types="vite-plus/test" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_skip_vitest() {
        let content = r#"/// <reference types="vite/client" />
/// <reference types="vitest" />
/// <reference types="@vitest/browser/matchers" />"#;

        let skip_packages =
            SkipPackages { skip_vite: false, skip_vitest: true, skip_tsdown: false };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/client" />
/// <reference types="vitest" />
/// <reference types="@vitest/browser/matchers" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_skip_tsdown() {
        let content = r#"/// <reference types="tsdown/client" />
/// <reference types="vite/client" />"#;

        let skip_packages =
            SkipPackages { skip_vite: false, skip_vitest: false, skip_tsdown: true };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="tsdown/client" />
/// <reference types="vite-plus/client" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_skip_all() {
        let content = r#"/// <reference types="vite/client" />
/// <reference types="vitest" />
/// <reference types="tsdown/client" />"#;

        let skip_packages = SkipPackages { skip_vite: true, skip_vitest: true, skip_tsdown: true };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    // ====================================
    // Re-export (`export ... from '...'`) Rewriting Tests
    // ====================================
    //
    // These mirror the `import_statement` rewrites for the `export_statement`
    // tree-sitter kind, which covers `export { x } from 'mod'`,
    // `export * from 'mod'`, `export * as ns from 'mod'`, and
    // `export type { x } from 'mod'`. Without these sibling rules, projects
    // that re-export from `@vitest/browser-playwright` (or the other browser
    // provider packages that `rewritePackageJson()` removes from package.json)
    // would silently end up with `ERR_PACKAGE_PATH_NOT_EXPORTED` at runtime.

    #[test]
    fn test_rewrite_export_named_from_vite() {
        let content = r#"export { defineConfig } from 'vite';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export { defineConfig } from 'vite-plus';"#);
    }

    #[test]
    fn test_rewrite_export_named_from_tsdown() {
        let content = r#"export { defineConfig } from 'tsdown';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export { defineConfig } from 'vite-plus/pack';"#);
    }

    #[test]
    fn test_rewrite_export_named_from_vitest_browser_context() {
        // `export { page } from '@vitest/browser/context'` — the regression
        // case called out in the chatgpt-codex review.
        let content = r#"export { page } from '@vitest/browser/context';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export { page } from 'vite-plus/test/browser/context';"#);
    }

    #[test]
    fn test_rewrite_export_star_from_vitest_browser_playwright() {
        // `export * from '@vitest/browser-playwright'` — the package is dropped
        // from package.json by `rewritePackageJson()`, so it MUST be rewritten
        // to avoid `ERR_PACKAGE_PATH_NOT_EXPORTED` under strict pnpm/PnP.
        let content = r#"export * from '@vitest/browser-playwright';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export * from 'vite-plus/test/browser-playwright';"#);
    }

    #[test]
    fn test_rewrite_export_star_as_ns_from_vitest() {
        // `export * as ns from 'vitest'` (namespace re-export).
        let content = r#"export * as vi from 'vitest';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export * as vi from 'vite-plus/test';"#);
    }

    #[test]
    fn test_rewrite_export_type_from_vitest_config() {
        // `export type { Config } from 'vitest/config'` — type-only re-export.
        // `vitest/config` collapses to bare `vite-plus` (matches the
        // import-shape rule).
        let content = r#"export type { Config } from 'vitest/config';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export type { Config } from 'vite-plus';"#);
    }

    #[test]
    fn test_rewrite_export_named_from_vitest_browser_preview() {
        let content = r#"export { preview } from "@vitest/browser-preview";"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export { preview } from "vite-plus/test/browser-preview";"#);
    }

    #[test]
    fn test_rewrite_export_star_from_vitest_browser_webdriverio_provider() {
        // `@vitest/browser-{provider}/provider` maps onto the shared
        // `vite-plus/test/browser/providers/{provider}` entry point.
        let content = r#"export * from '@vitest/browser-webdriverio/provider';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"export * from 'vite-plus/test/browser/providers/webdriverio';"#
        );
    }

    #[test]
    fn test_rewrite_export_named_from_vite_subpath() {
        let content = r#"export { ModuleRunner } from 'vite/module-runner';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export { ModuleRunner } from 'vite-plus/module-runner';"#);
    }

    #[test]
    fn test_rewrite_export_value_with_vitest_string_literal_not_rewritten() {
        // A plain value declaration whose initializer is the string `'vitest'`
        // is NOT a re-export — the string literal is outside any
        // import_statement / export_statement / require call and must be left
        // alone.
        let content = r#"export const foo = 'vitest';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    // =========================================
    // Dynamic import() and TS import-type tests
    // =========================================

    #[test]
    fn test_rewrite_dynamic_import_vite() {
        let content = r#"const m = await import('vite');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import('vite-plus');"#);
    }

    #[test]
    fn test_rewrite_dynamic_import_vite_subpath() {
        let content = r#"const m = await import("vite/module-runner");"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import("vite-plus/module-runner");"#);
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest() {
        let content = r#"const m = await import('vitest');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import('vite-plus/test');"#);
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_config() {
        let content = r#"const m = await import('vitest/config');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import('vite-plus');"#);
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_subpath() {
        let content = r#"const m = await import('vitest/node');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import('vite-plus/test/node');"#);
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_browser() {
        let content = r#"const provider = await import('@vitest/browser');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const provider = await import('vite-plus/test/browser');"#);
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_browser_context() {
        let content = r#"const ctx = await import('@vitest/browser/context');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const ctx = await import('vite-plus/test/browser/context');"#
        );
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_browser_flat_subpath() {
        // `@vitest/browser/{client,locators,matchers,utils}` strips the
        // `/browser/` segment, matching the static-import rule.
        for (sub, expected) in [
            ("client", "vite-plus/test/client"),
            ("locators", "vite-plus/test/locators"),
            ("matchers", "vite-plus/test/matchers"),
            ("utils", "vite-plus/test/utils"),
        ] {
            let content = format!("const x = await import('@vitest/browser/{sub}');");
            let result = rewrite_import_content(&content, &SkipPackages::default()).unwrap();
            assert!(result.updated, "dynamic import of @vitest/browser/{sub} should be rewritten");
            assert_eq!(result.content, format!("const x = await import('{expected}');"));
        }
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_browser_playwright() {
        let content = r#"const p = await import('@vitest/browser-playwright');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const p = await import('vite-plus/test/browser-playwright');"#
        );
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_browser_playwright_context() {
        let content = r#"const p = await import('@vitest/browser-playwright/context');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const p = await import('vite-plus/test/browser/context');"#);
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_browser_playwright_provider() {
        let content = r#"const p = await import('@vitest/browser-playwright/provider');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const p = await import('vite-plus/test/browser/providers/playwright');"#
        );
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_browser_preview_provider() {
        let content = r#"const p = await import('@vitest/browser-preview/provider');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const p = await import('vite-plus/test/browser/providers/preview');"#
        );
    }

    #[test]
    fn test_rewrite_dynamic_import_vitest_browser_webdriverio_provider() {
        let content = r#"const p = await import('@vitest/browser-webdriverio/provider');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"const p = await import('vite-plus/test/browser/providers/webdriverio');"#
        );
    }

    #[test]
    fn test_rewrite_dynamic_import_tsdown() {
        let content = r#"const m = await import('tsdown');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import('vite-plus/pack');"#);
    }

    #[test]
    fn test_rewrite_dynamic_import_tsdown_client() {
        let content = r#"const m = await import('tsdown/client');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import('vite-plus/pack/client');"#);
    }

    #[test]
    fn test_rewrite_ts_import_type_vitest_browser_context() {
        // TS type-position `typeof import('@vitest/browser/context')` is also
        // a `call_expression` with the special `import` token as callee, so
        // the same rule covers it.
        let content = r#"type C = typeof import('@vitest/browser/context');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"type C = typeof import('vite-plus/test/browser/context');"#);
    }

    #[test]
    fn test_rewrite_ts_import_type_vitest_browser_playwright() {
        let content = r#"type P = typeof import('@vitest/browser-playwright');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"type P = typeof import('vite-plus/test/browser-playwright');"#
        );
    }

    #[test]
    fn test_rewrite_dynamic_import_does_not_touch_string_literal_args() {
        // A string literal that *contains* an `import('@vitest/browser')`-looking
        // payload must NOT be rewritten — it is not a real dynamic import.
        let content = r#"const x = "import('@vitest/browser')";"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_dynamic_import_does_not_touch_member_call_named_import() {
        // A property-access call like `obj.import('@vitest/browser')` is a
        // user-defined function, not a dynamic import expression. Its callee
        // node-kind is `member_expression`, not `import`, so the rule must
        // not match.
        let content = r#"const p = obj.import('@vitest/browser');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_dynamic_import_mixed_with_static_imports() {
        // Verifies a mixed file with static, dynamic, and TS-type imports.
        let content = r#"import { describe } from 'vitest';
const browser = await import('@vitest/browser');
type C = typeof import('@vitest/browser/context');
const provider = await import('@vitest/browser-playwright');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { describe } from 'vite-plus/test';
const browser = await import('vite-plus/test/browser');
type C = typeof import('vite-plus/test/browser/context');
const provider = await import('vite-plus/test/browser-playwright');"#
        );
    }

    // `vitest/package.json` is a metadata-access pattern (typically used to
    // read the vitest version). Rewriting it to `vite-plus/test/package.json`
    // would fail at runtime with `ERR_PACKAGE_PATH_NOT_EXPORTED` because
    // `syncTestPackageExports()` deliberately skips the upstream
    // `./package.json` export. The catch-all `vitest/*` subpath rules MUST
    // leave it alone so the original specifier still resolves through the
    // transitively-installed `vitest` dependency.

    #[test]
    fn test_rewrite_import_content_vitest_package_json_static_import() {
        let content = r#"import pkg from 'vitest/package.json';

console.log(pkg.version);"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_import_content_vitest_package_json_static_import_double_quotes() {
        let content = r#"import pkg from "vitest/package.json";

console.log(pkg.version);"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_import_content_vitest_package_json_require() {
        let content = r#"const pkg = require('vitest/package.json');

console.log(pkg.version);"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_import_content_vitest_package_json_dynamic_import() {
        let content = r#"const pkg = await import('vitest/package.json');

console.log(pkg.version);"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_import_content_vitest_package_json_typeof_import() {
        let content = r#"type Pkg = typeof import('vitest/package.json');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_import_content_vitest_package_json_export_from() {
        let content = r#"export { version } from 'vitest/package.json';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_import_content_vitest_config_still_rewritten() {
        // Sanity check: the `package.json` exclusion must not interfere with
        // the existing `vitest/config` → `vite-plus` rule (which produces a
        // bare `vite-plus` target, NOT `vite-plus/test/config`).
        let content = r#"import 'vitest/config';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"import 'vite-plus';"#);
    }

    #[test]
    fn test_rewrite_import_content_vitest_package_json_like_suffix_still_rewritten() {
        // Sanity check: only the exact `vitest/package.json` subpath is
        // skipped. A subpath that merely starts with `package.json` (e.g.,
        // `package.json.js`) MUST still be rewritten by the catch-all rule.
        let content = r#"import 'vitest/package.json.js';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"import 'vite-plus/test/package.json.js';"#);
    }

    // The `rewrite-vitest-subpath-*` rules now explicitly exclude `vitest/config`
    // via `not: regex: ^['"]vitest/(package\.json|config)['"]$` to make the intent
    // robust against rule-reordering or ast-grep conflict-resolution changes.
    // These tests verify the exclusion holds for all 4 import shapes and that
    // a similar-but-distinct path like `vitest/config-extra` is NOT excluded.

    #[test]
    fn test_rewrite_import_content_vitest_config_export_not_subpath_rewritten() {
        // `export ... from 'vitest/config'` must collapse to `vite-plus` (bare),
        // NOT `vite-plus/test/config`. The subpath rule's `not` exclusion prevents
        // the wrong fix from firing.
        let content = r#"export type { Config } from 'vitest/config';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export type { Config } from 'vite-plus';"#);
    }

    #[test]
    fn test_rewrite_import_content_vitest_config_require_not_subpath_rewritten() {
        // `require('vitest/config')` must collapse to `vite-plus` (bare),
        // NOT `vite-plus/test/config`.
        let content = r#"const { defineConfig } = require('vitest/config');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const { defineConfig } = require('vite-plus');"#);
    }

    #[test]
    fn test_rewrite_import_content_vitest_config_dynamic_import_not_subpath_rewritten() {
        // `import('vitest/config')` must collapse to `vite-plus` (bare),
        // NOT `vite-plus/test/config`.
        let content = r#"const m = await import('vitest/config');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import('vite-plus');"#);
    }

    #[test]
    fn test_rewrite_import_content_vitest_config_extra_subpath_static_import() {
        // `vitest/config-extra` is NOT `vitest/config` — the subpath exclusion
        // must not be too broad. It should still be rewritten by the catch-all
        // subpath rule to `vite-plus/test/config-extra`.
        let content = r#"import { something } from 'vitest/config-extra';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"import { something } from 'vite-plus/test/config-extra';"#);
    }

    #[test]
    fn test_rewrite_import_content_vitest_config_extra_subpath_export() {
        // Same sanity check for export_statement shape.
        let content = r#"export { something } from 'vitest/config-extra';"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"export { something } from 'vite-plus/test/config-extra';"#);
    }

    #[test]
    fn test_rewrite_import_content_vitest_config_extra_subpath_require() {
        // Same sanity check for require() shape.
        let content = r#"const m = require('vitest/config-extra');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = require('vite-plus/test/config-extra');"#);
    }

    #[test]
    fn test_rewrite_import_content_vitest_config_extra_subpath_dynamic_import() {
        // Same sanity check for dynamic import() shape.
        let content = r#"const m = await import('vitest/config-extra');"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"const m = await import('vite-plus/test/config-extra');"#);
    }
}
