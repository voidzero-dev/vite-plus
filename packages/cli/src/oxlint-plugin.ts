import type { RuleTester } from 'oxlint/plugins-dev';

import {
  PREFER_VITE_PLUS_IMPORTS_RULE_NAME,
  VITE_PLUS_OXLINT_PLUGIN_NAME,
} from './oxlint-plugin-config.ts';

type OxlintRule = Parameters<RuleTester['run']>[1];
type CreateFn = Exclude<OxlintRule['create'], undefined>;
type OxlintVisitor = ReturnType<CreateFn>;
type ReportContext = Parameters<CreateFn>[0];
type VisitorNode<K extends keyof OxlintVisitor> = Parameters<NonNullable<OxlintVisitor[K]>>[0];

type StringLiteralLike = VisitorNode<'ImportDeclaration'>['source'];
interface OxlintPlugin {
  meta: {
    name: string;
  };
  rules: Record<string, OxlintRule>;
}

function isStringLiteralLike(value: unknown): value is StringLiteralLike {
  return (
    typeof value === 'object' &&
    value !== null &&
    'type' in value &&
    value.type === 'Literal' &&
    'value' in value &&
    typeof value.value === 'string'
  );
}

function rewriteVitePlusImportSpecifier(specifier: string): string | null {
  if (specifier === 'vite') {
    return 'vite-plus';
  }

  if (specifier.startsWith('vite/')) {
    return `vite-plus/${specifier.slice('vite/'.length)}`;
  }

  if (specifier === 'vitest/config') {
    return 'vite-plus';
  }

  if (specifier === 'vitest') {
    return 'vite-plus/test';
  }

  if (specifier.startsWith('vitest/')) {
    return `vite-plus/test/${specifier.slice('vitest/'.length)}`;
  }

  if (specifier === '@vitest/browser') {
    return 'vite-plus/test/browser';
  }

  const browserSubpathRewrites: Record<string, string> = {
    '@vitest/browser/context': 'vite-plus/test/browser/context',
    '@vitest/browser/client': 'vite-plus/test/client',
    '@vitest/browser/locators': 'vite-plus/test/locators',
  };
  if (specifier in browserSubpathRewrites) {
    return browserSubpathRewrites[specifier];
  }

  for (const [prefix, provider] of [
    ['@vitest/browser-playwright', 'playwright'],
    ['@vitest/browser-preview', 'preview'],
    ['@vitest/browser-webdriverio', 'webdriverio'],
  ] as const) {
    if (specifier === prefix) {
      return `vite-plus/test/${prefix.slice('@vitest/'.length)}`;
    }

    if (specifier === `${prefix}/context`) {
      return 'vite-plus/test/browser/context';
    }

    if (specifier === `${prefix}/provider`) {
      return `vite-plus/test/browser/providers/${provider}`;
    }
  }

  return null;
}

function quoteSpecifier(literal: StringLiteralLike, replacement: string): string {
  const quote = literal.raw?.startsWith("'") ? "'" : '"';
  return `${quote}${replacement}${quote}`;
}

function maybeReportLiteral(context: ReportContext, literal: StringLiteralLike | null | undefined) {
  if (!literal || typeof literal.value !== 'string') {
    return;
  }

  const replacement = rewriteVitePlusImportSpecifier(literal.value);
  if (!replacement) {
    return;
  }

  context.report({
    node: literal,
    messageId: 'preferVitePlusImports',
    data: {
      from: literal.value,
      to: replacement,
    },
    fix(fixer) {
      return fixer.replaceText(literal, quoteSpecifier(literal, replacement));
    },
  });
}

export const preferVitePlusImportsRule: OxlintRule = {
  meta: {
    type: 'problem',
    docs: {
      description: 'Prefer vite-plus module specifiers over vite and vitest packages.',
      recommended: true,
      url: 'https://github.com/voidzero-dev/vite-plus/issues/1301',
    },
    fixable: 'code',
    messages: {
      preferVitePlusImports: "Use '{{to}}' instead of '{{from}}' in Vite+ projects.",
    },
  },
  createOnce(context: ReportContext) {
    return {
      ImportDeclaration(node: VisitorNode<'ImportDeclaration'>) {
        maybeReportLiteral(context, node.source);
      },
      ExportAllDeclaration(node: VisitorNode<'ExportAllDeclaration'>) {
        maybeReportLiteral(context, node.source);
      },
      ExportNamedDeclaration(node: VisitorNode<'ExportNamedDeclaration'>) {
        maybeReportLiteral(context, node.source);
      },
      ImportExpression(node: VisitorNode<'ImportExpression'>) {
        if (!isStringLiteralLike(node.source)) {
          return;
        }
        maybeReportLiteral(context, node.source);
      },
      TSImportType(node: VisitorNode<'TSImportType'>) {
        maybeReportLiteral(context, node.source);
      },
      TSExternalModuleReference(node: VisitorNode<'TSExternalModuleReference'>) {
        maybeReportLiteral(context, node.expression);
      },
      TSModuleDeclaration(node: VisitorNode<'TSModuleDeclaration'>) {
        if (node.global || !isStringLiteralLike(node.id)) {
          return;
        }
        maybeReportLiteral(context, node.id);
      },
    };
  },
};

const plugin: OxlintPlugin = {
  meta: {
    name: VITE_PLUS_OXLINT_PLUGIN_NAME,
  },
  rules: {
    [PREFER_VITE_PLUS_IMPORTS_RULE_NAME]: preferVitePlusImportsRule,
  },
};

export default plugin;
export { rewriteVitePlusImportSpecifier };
