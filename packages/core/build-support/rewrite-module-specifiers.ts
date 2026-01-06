import { Lang, parse, type Edit, type SgNode } from '@ast-grep/napi';

export interface ReplacementRule {
  /** The module specifier pattern to match (string for exact/prefix, RegExp for pattern) */
  from: string | RegExp;
  /** The replacement (string or function for dynamic replacement) */
  to: string | ((match: string) => string);
}

export interface RewriteOptions {
  rules: ReplacementRule[];
}

/**
 * Get the ast-grep language for a given file path
 */
function getLangForFile(filePath: string): Lang {
  if (filePath.endsWith('.tsx')) {
    return Lang.Tsx;
  }
  if (
    filePath.endsWith('.ts') ||
    filePath.endsWith('.d.ts') ||
    filePath.endsWith('.mts') ||
    filePath.endsWith('.d.mts')
  ) {
    return Lang.TypeScript;
  }
  return Lang.JavaScript;
}

/**
 * Extract the string content from a string literal node (removes quotes)
 */
function getStringContent(node: SgNode): string {
  const text = node.text();
  // Remove surrounding quotes (single, double, or backtick)
  if (
    (text.startsWith('"') && text.endsWith('"')) ||
    (text.startsWith("'") && text.endsWith("'")) ||
    (text.startsWith('`') && text.endsWith('`'))
  ) {
    return text.slice(1, -1);
  }
  return text;
}

/**
 * Get the quote character used in a string literal
 */
function getQuoteChar(node: SgNode): string {
  const text = node.text();
  return text[0] || '"';
}

/**
 * Check if specifier matches the "from" pattern
 * Matches exact, subpath (from/...), or file extension (from.xxx)
 */
function matchesFrom(specifier: string, from: string): boolean {
  if (specifier === from) return true;
  if (!specifier.startsWith(from)) return false;
  // Check the character after the prefix - must be '/', '.', or end of string
  const nextChar = specifier[from.length];
  return nextChar === '/' || nextChar === '.';
}

/**
 * Apply replacement rules to a module specifier
 */
function applyRules(specifier: string, rules: ReplacementRule[]): string | null {
  for (const rule of rules) {
    if (typeof rule.from === 'string') {
      // Exact match or prefix match (e.g., "vite" matches "vite", "vite/...", and "vite.xxx")
      if (matchesFrom(specifier, rule.from)) {
        if (typeof rule.to === 'function') {
          return rule.to(specifier);
        }
        // Replace the "from" prefix with the "to" value
        return rule.to + specifier.slice(rule.from.length);
      }
    } else {
      // RegExp match
      if (rule.from.test(specifier)) {
        if (typeof rule.to === 'function') {
          return rule.to(specifier);
        }
        return specifier.replace(rule.from, rule.to);
      }
    }
  }
  return null;
}

/**
 * Find all string literal children within a node that could be module specifiers
 */
function findStringLiterals(node: SgNode): SgNode[] {
  const results: SgNode[] = [];
  const children = node.children();
  for (const child of children) {
    const kind = child.kind();
    if (kind === 'string' || kind === 'string_literal') {
      results.push(child);
    }
    // Also check nested children (e.g., for call_expression arguments)
    results.push(...findStringLiterals(child));
  }
  return results;
}

/**
 * Rewrite module specifiers in source code using ast-grep
 */
export function rewriteModuleSpecifiers(
  source: string,
  filePath: string,
  options: RewriteOptions,
): string {
  const lang = getLangForFile(filePath);
  const ast = parse(lang, source);
  const root = ast.root();
  const edits: Edit[] = [];
  const processedRanges = new Set<string>();

  // Find all import/export statements, call expressions, and ambient module declarations
  const nodeKinds = ['import_statement', 'export_statement', 'call_expression'];

  // Add TypeScript-specific kinds for .d.ts files
  if (lang === Lang.TypeScript || lang === Lang.Tsx) {
    nodeKinds.push('ambient_declaration'); // For `declare module "..."` in .d.ts files
  }

  for (const kindName of nodeKinds) {
    const matches = root.findAll({ rule: { kind: kindName } });

    for (const match of matches) {
      // For call expressions, check if it's require/__require/import()
      if (kindName === 'call_expression') {
        const text = match.text();
        if (
          !text.startsWith('require(') &&
          !text.startsWith('__require(') &&
          !text.startsWith('import(')
        ) {
          continue;
        }
      }

      // Find string literals within this node
      const stringNodes = findStringLiterals(match);

      for (const stringNode of stringNodes) {
        // Deduplicate by range
        const range = stringNode.range();
        const rangeKey = `${range.start.index}-${range.end.index}`;
        if (processedRanges.has(rangeKey)) {
          continue;
        }
        processedRanges.add(rangeKey);

        const content = getStringContent(stringNode);
        const newContent = applyRules(content, options.rules);

        if (newContent !== null && newContent !== content) {
          const quote = getQuoteChar(stringNode);
          edits.push(stringNode.replace(`${quote}${newContent}${quote}`));
        }
      }
    }
  }

  if (edits.length === 0) {
    return source;
  }

  return root.commitEdits(edits);
}

/**
 * Create replacement rules for rewriting vite imports to a target package
 */
export function createViteRewriteRules(targetPackage: string): ReplacementRule[] {
  return [
    // "vite" -> "targetPackage" (exact match and prefix)
    { from: 'vite', to: targetPackage },
  ];
}

/**
 * Create replacement rules for rewriting rolldown imports to a target package
 */
export function createRolldownRewriteRules(targetPackage: string): ReplacementRule[] {
  return [
    // "rolldown" -> "targetPackage/rolldown"
    { from: 'rolldown', to: `${targetPackage}/rolldown` },
    // "@rolldown/pluginutils" -> "targetPackage/rolldown/pluginutils"
    { from: '@rolldown/pluginutils', to: `${targetPackage}/rolldown/pluginutils` },
  ];
}
