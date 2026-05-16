import type { OxlintConfig } from 'oxlint';

import { VITE_PLUS_NAME } from './utils/constants.ts';

export const VITE_PLUS_OXLINT_PLUGIN_NAME = VITE_PLUS_NAME;
export const VITE_PLUS_OXLINT_PLUGIN_SPECIFIER = `${VITE_PLUS_NAME}/oxlint-plugin`;
export const PREFER_VITE_PLUS_IMPORTS_RULE_NAME = 'prefer-vite-plus-imports';
export const PREFER_VITE_PLUS_IMPORTS_RULE = `${VITE_PLUS_OXLINT_PLUGIN_NAME}/${PREFER_VITE_PLUS_IMPORTS_RULE_NAME}`;

type JsPluginEntry = NonNullable<OxlintConfig['jsPlugins']>[number];

function hasVitePlusPlugin(entry: JsPluginEntry): boolean {
  if (typeof entry === 'string') {
    return entry === VITE_PLUS_OXLINT_PLUGIN_SPECIFIER;
  }

  return entry.specifier === VITE_PLUS_OXLINT_PLUGIN_SPECIFIER;
}

function isRuleRecord(
  value: OxlintConfig['rules'] | undefined,
): value is NonNullable<OxlintConfig['rules']> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function ensureVitePlusImportRuleDefaults<
  T extends Pick<OxlintConfig, 'jsPlugins' | 'rules'>,
>(config: T): T {
  const jsPlugins = Array.isArray(config.jsPlugins) ? [...config.jsPlugins] : [];
  if (!jsPlugins.some(hasVitePlusPlugin)) {
    jsPlugins.push({
      name: VITE_PLUS_OXLINT_PLUGIN_NAME,
      specifier: VITE_PLUS_OXLINT_PLUGIN_SPECIFIER,
    });
  }

  const rules = isRuleRecord(config.rules) ? { ...config.rules } : {};
  if (!(PREFER_VITE_PLUS_IMPORTS_RULE in rules)) {
    rules[PREFER_VITE_PLUS_IMPORTS_RULE] = 'error';
  }

  return {
    ...config,
    jsPlugins,
    rules,
  };
}

export function createDefaultVitePlusLintConfig(options?: {
  includeTypeAwareDefaults?: boolean;
}): Pick<OxlintConfig, 'jsPlugins' | 'options' | 'rules'> {
  const config: Pick<OxlintConfig, 'jsPlugins' | 'options' | 'rules'> =
    ensureVitePlusImportRuleDefaults({});
  if (options?.includeTypeAwareDefaults) {
    config.options = {
      typeAware: true,
      typeCheck: true,
    };
  }
  return config;
}
