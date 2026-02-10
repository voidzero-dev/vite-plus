import { builtinModules } from 'node:module';

import {
  parse,
  type ParseResult,
  Visitor,
  type CallExpression,
  type Expression,
  type StaticMemberExpression,
  type VariableDeclarator,
} from 'oxc-parser';

import { createModuleEntryFileName } from './build-cjs-deps';

// Node.js built-in modules (without node: prefix)
const nodeBuiltins = new Set(builtinModules);

// TODO, analysis the optional peerDependencies in the dependencies tree to exclude them in the future
const optionalCjsExternal = new Set<string>(['oxc-resolver', 'synckit']);

/**
 * Check if a module specifier is a third-party package
 * (not a Node.js built-in, not a relative path)
 */
function isThirdPartyModule(specifier: string): boolean {
  // Filter out relative paths
  if (specifier.startsWith('./') || specifier.startsWith('../')) {
    return false;
  }
  // Filter out Node.js built-ins (with or without node: prefix)
  if (specifier.startsWith('node:')) {
    return false;
  }
  if (nodeBuiltins.has(specifier) || optionalCjsExternal.has(specifier)) {
    return false;
  }
  return true;
}

/**
 * Find and replace all third-party CJS requires with local entry files
 * Returns the modified source code and the set of third-party modules found
 */
export async function replaceThirdPartyCjsRequires(
  source: string,
  filePath: string,
  tsdownExternal: Set<string>,
): Promise<{ code: string; modules: Set<string> }> {
  const ast = await parse(filePath, source, {
    lang: 'js',
    sourceType: 'module',
  });

  const thirdPartyModules = new Set<string>();

  // Find all createRequire patterns and their require calls
  const results = [
    findCreateRequireInStaticImports(ast),
    findCreateRequireInGlobalModule(ast),
  ].filter((result): result is { requireVarName: string; calls: RequireCall[] } => Boolean(result));

  // Collect all third-party require calls
  const replacements: RequireCall[] = [];
  for (const { calls } of results) {
    for (const call of calls) {
      if (isThirdPartyModule(call.module)) {
        const parts = call.module.split('/');
        const moduleName = call.module.startsWith('@') ? parts.slice(0, 2).join('/') : parts[0];
        if (!tsdownExternal.has(moduleName)) {
          thirdPartyModules.add(call.module);
          replacements.push(call);
        }
      }
    }
  }

  // Sort by position descending (process from end to start to avoid offset issues)
  replacements.sort((a, b) => b.start - a.start);

  // Perform replacements
  let code = source;
  for (const { module, start, end } of replacements) {
    const newSpecifier = `"./${createModuleEntryFileName(module)}"`;
    code = code.slice(0, start) + newSpecifier + code.slice(end);
  }

  return { code, modules: thirdPartyModules };
}

interface RequireCall {
  module: string;
  start: number;
  end: number;
}

/**
 * Find all calls to a specific require function and return the module specifiers with positions
 */
function findRequireCalls(ast: ParseResult, requireVarName: string): RequireCall[] {
  const calls: RequireCall[] = [];

  const visitor = new Visitor({
    CallExpression(node: CallExpression) {
      // Check if callee is the require variable
      if (node.callee.type !== 'Identifier' || node.callee.name !== requireVarName) {
        return;
      }

      // Extract the first argument (module specifier)
      if (node.arguments.length === 0) {
        return;
      }
      const arg = node.arguments[0];
      if (arg.type !== 'Literal') {
        return;
      }
      const value = (arg as { value: unknown; start: number; end: number }).value;
      if (typeof value === 'string') {
        calls.push({
          module: value,
          start: arg.start,
          end: arg.end,
        });
      }
    },
  });

  visitor.visit(ast.program);
  return calls;
}

/**
 * Find createRequire from static imports and return the require variable name + all require calls
 * Handles: `import { createRequire } from "node:module"` then `const require = createRequire(...)`
 */
function findCreateRequireInStaticImports(
  ast: ParseResult,
): { requireVarName: string; calls: RequireCall[] } | undefined {
  // Find import from 'module' or 'node:module'
  const importFromModule = ast.module.staticImports.find((imt) => {
    const { value } = imt.moduleRequest;
    return value === 'node:module' || value === 'module';
  });
  if (!importFromModule) {
    return;
  }

  // Find the createRequire import entry
  const createRequireEntry = importFromModule.entries.find((entry) => {
    return entry.importName.name === 'createRequire';
  });
  if (!createRequireEntry) {
    return;
  }

  const createRequireLocalName = createRequireEntry.localName.value;

  // Find the variable that stores the result of createRequire(...)
  // e.g., `const __require = createRequire(import.meta.url)`
  let requireVarName: string | undefined;

  const varVisitor = new Visitor({
    VariableDeclarator(node: VariableDeclarator) {
      if (!node.init || node.init.type !== 'CallExpression') {
        return;
      }
      const call = node.init;
      if (call.callee.type === 'Identifier' && call.callee.name === createRequireLocalName) {
        if (node.id.type === 'Identifier') {
          requireVarName = node.id.name;
        }
      }
    },
  });
  varVisitor.visit(ast.program);

  if (!requireVarName) {
    return;
  }

  // Find all calls to the require variable
  const calls = findRequireCalls(ast, requireVarName);

  return { requireVarName, calls };
}

// Helper to check if an expression is `process` or `globalThis.process`
function isProcessExpression(expr: Expression): boolean {
  // Check for `process`
  if (expr.type === 'Identifier' && expr.name === 'process') {
    return true;
  }
  // Check for `globalThis.process`
  if (expr.type === 'MemberExpression' && !expr.computed) {
    const memberExpr = expr as StaticMemberExpression;
    return (
      memberExpr.object.type === 'Identifier' &&
      memberExpr.object.name === 'globalThis' &&
      memberExpr.property.name === 'process'
    );
  }
  return false;
}

// Helper to check if a CallExpression is `[process|globalThis.process].getBuiltinModule("module")`
function isGetBuiltinModuleCall(expr: Expression): boolean {
  if (expr.type !== 'CallExpression') {
    return false;
  }
  const call = expr;

  // Check callee is a member expression with property `getBuiltinModule`
  if (call.callee.type !== 'MemberExpression' || call.callee.computed) {
    return false;
  }
  const callee = call.callee as StaticMemberExpression;
  if (callee.property.name !== 'getBuiltinModule') {
    return false;
  }

  // Check the object is `process` or `globalThis.process`
  if (!isProcessExpression(callee.object)) {
    return false;
  }

  // Check argument is "module" or "node:module"
  if (call.arguments.length === 0) {
    return false;
  }
  const arg = call.arguments[0];
  if (arg.type !== 'Literal') {
    return false;
  }
  const value = (arg as { value: unknown }).value;
  return value === 'module' || value === 'node:module';
}

/**
 * Find createRequire from getBuiltinModule and return the require variable name + all require calls
 * Handles: `const require = globalThis.process.getBuiltinModule("module").createRequire(import.meta.url)`
 * Or: `const require = process.getBuiltinModule("module").createRequire(import.meta.url)`
 */
function findCreateRequireInGlobalModule(
  ast: ParseResult,
): { requireVarName: string; calls: RequireCall[] } | undefined {
  let requireVarName: string | undefined;

  const visitor = new Visitor({
    VariableDeclarator(node: VariableDeclarator) {
      if (!node.init || node.init.type !== 'CallExpression') {
        return;
      }

      const call = node.init;

      // Check if callee is a MemberExpression with property `createRequire`
      if (call.callee.type !== 'MemberExpression' || call.callee.computed) {
        return;
      }
      const callee = call.callee as StaticMemberExpression;
      if (callee.property.name !== 'createRequire') {
        return;
      }

      // Check if the object is a getBuiltinModule("module") call
      if (!isGetBuiltinModuleCall(callee.object)) {
        return;
      }

      // Extract variable name
      if (node.id.type === 'Identifier') {
        requireVarName = node.id.name;
      }
    },
  });

  visitor.visit(ast.program);

  if (!requireVarName) {
    return;
  }

  // Find all calls to the require variable
  const calls = findRequireCalls(ast, requireVarName);

  return { requireVarName, calls };
}
