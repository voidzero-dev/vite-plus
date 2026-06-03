import { definePlugin } from '@oxlint/plugins';

import {
  PREFER_VITE_PLUS_IMPORTS_RULE_NAME,
  REQUIRE_PNPM_VITE_ALIAS_RULE_NAME,
  VITE_PLUS_OXLINT_PLUGIN_NAME,
} from './oxlint-plugin-config.ts';
import {
  preferVitePlusImportsRule,
  rewriteVitePlusImportSpecifier,
} from './oxlint-plugin/rules/prefer-vite-plus-imports.ts';
import {
  pnpmWorkspaceAliasesViteToVitePlusCore,
  requirePnpmViteAliasRule,
  shouldRequirePnpmViteAlias,
} from './oxlint-plugin/rules/require-pnpm-vite-alias.ts';

const plugin = definePlugin({
  meta: {
    name: VITE_PLUS_OXLINT_PLUGIN_NAME,
  },
  rules: {
    [PREFER_VITE_PLUS_IMPORTS_RULE_NAME]: preferVitePlusImportsRule,
    [REQUIRE_PNPM_VITE_ALIAS_RULE_NAME]: requirePnpmViteAliasRule,
  },
});

export default plugin;
export {
  pnpmWorkspaceAliasesViteToVitePlusCore,
  preferVitePlusImportsRule,
  requirePnpmViteAliasRule,
  rewriteVitePlusImportSpecifier,
  shouldRequirePnpmViteAlias,
};
