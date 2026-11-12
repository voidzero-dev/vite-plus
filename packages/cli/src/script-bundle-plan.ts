/**
 * Pure parts of the `vpx` bundle cache builder (`script-bundle-build.ts`): what one
 * module needs from the bundler, and which modules the bundle may contain.
 */

/** The ESTree subset this analysis reads; Rolldown's `parseAst` produces it. */
interface AstNode {
  type: string;
  start: number;
  end: number;
  [key: string]: unknown;
}

/** An `import.meta.<property>` read to replace with this module's own value. */
export interface MetaEdit {
  start: number;
  end: number;
  property: 'url' | 'filename' | 'dirname' | 'main';
}

/**
 * A specifier that only the run knows: a computed `import()`, or the argument of
 * `import.meta.resolve()`. The bundle prefixes it with `RESOLVE_FROM_PREFIX`, so the run
 * resolves it from this module's own URL instead of the bundle's.
 */
export interface ResolveEdit {
  start: number;
  end: number;
}

export interface ModuleAnalysis {
  /** Every specifier the module imports: static imports, re-exports, literal `import()`. */
  specifiers: string[];
  /** The specifiers of literal `import()` calls, which a static import may also name. */
  dynamicSpecifiers: string[];
  metaEdits: MetaEdit[];
  resolveEdits: ResolveEdit[];
  /** Whether the module awaits at its top level, which a shared chunk cannot do. */
  topLevelAwait: boolean;
  /** Why the module cannot be bundled, or `undefined`. */
  unsafe?: string;
}

const META_PROPERTIES = new Set(['url', 'filename', 'dirname', 'main']);

function isNode(value: unknown): value is AstNode {
  return typeof value === 'object' && value !== null && typeof (value as AstNode).type === 'string';
}

/** The value of a string literal or a template literal without substitutions. */
function staticString(node: unknown): string | undefined {
  if (!isNode(node)) {
    return undefined;
  }
  if (node.type === 'Literal' && typeof node.value === 'string') {
    return node.value;
  }
  if (node.type === 'TemplateLiteral' && (node.expressions as unknown[]).length === 0) {
    const [quasi] = node.quasis as { value: { cooked: string | null } }[];
    return quasi?.value.cooked ?? undefined;
  }
  return undefined;
}

/** Whether `node` is `import.meta`. */
function isImportMeta(node: unknown): node is AstNode {
  return (
    isNode(node) &&
    node.type === 'MetaProperty' &&
    (node.meta as { name: string }).name === 'import'
  );
}

/**
 * Read what bundling one ES module depends on. A module is unsafe to bundle when its
 * behavior depends on its own location in a way the bundle cannot keep: a use of
 * `import.meta` other than reading `url`, `filename`, `dirname`, or `main`, or calling
 * `resolve()`; and a direct `eval`, which sees the renamed bindings of a scope-hoisted
 * bundle.
 */
export function analyzeModule(program: AstNode): ModuleAnalysis {
  const specifiers: string[] = [];
  const dynamicSpecifiers: string[] = [];
  const metaEdits: MetaEdit[] = [];
  const resolveEdits: ResolveEdit[] = [];
  /** `import.meta` nodes already handled as the object of `import.meta.resolve()`. */
  const handled = new WeakSet<AstNode>();
  let unsafe: string | undefined;
  let topLevelAwait = false;
  let functionDepth = 0;

  const visit = (node: AstNode, parent: AstNode | undefined): void => {
    switch (node.type) {
      case 'ImportDeclaration':
      case 'ExportAllDeclaration':
      case 'ExportNamedDeclaration': {
        const source = staticString(node.source);
        if (source !== undefined) {
          specifiers.push(source);
        }
        break;
      }
      case 'ImportExpression': {
        const source = staticString(node.source);
        if (source === undefined) {
          const expression = node.source as AstNode;
          resolveEdits.push({ start: expression.start, end: expression.end });
        } else {
          specifiers.push(source);
          dynamicSpecifiers.push(source);
        }
        break;
      }
      case 'CallExpression': {
        const callee = node.callee as AstNode;
        const [argument] = node.arguments as AstNode[];
        if (callee.type === 'Identifier' && callee.name === 'eval') {
          unsafe ??= 'a direct eval()';
        } else if (
          callee.type === 'MemberExpression' &&
          isImportMeta(callee.object) &&
          !callee.computed &&
          (callee.property as { name?: string }).name === 'resolve' &&
          argument !== undefined &&
          argument.type !== 'SpreadElement'
        ) {
          handled.add(callee.object);
          resolveEdits.push({ start: argument.start, end: argument.end });
        }
        break;
      }
      case 'MetaProperty': {
        if (!isImportMeta(node) || handled.has(node)) {
          break;
        }
        const property =
          parent?.type === 'MemberExpression' && parent.object === node && !parent.computed
            ? (parent.property as { name?: string }).name
            : undefined;
        if (property !== undefined && META_PROPERTIES.has(property)) {
          metaEdits.push({
            start: parent!.start,
            end: parent!.end,
            property: property as MetaEdit['property'],
          });
        } else {
          unsafe ??= property === undefined ? 'import.meta' : `import.meta.${property}`;
        }
        break;
      }
      case 'AwaitExpression':
        topLevelAwait ||= functionDepth === 0;
        break;
      case 'ForOfStatement':
        topLevelAwait ||= functionDepth === 0 && node.await === true;
        break;
      case 'VariableDeclaration':
        topLevelAwait ||= functionDepth === 0 && node.kind === 'await using';
        break;
    }
    const isFunction = FUNCTIONS.has(node.type);
    if (isFunction) {
      functionDepth++;
    }
    for (const key in node) {
      if (key === 'type' || key === 'start' || key === 'end') {
        continue;
      }
      const value = node[key];
      if (Array.isArray(value)) {
        for (const child of value) {
          if (isNode(child)) {
            visit(child, node);
          }
        }
      } else if (isNode(value)) {
        visit(value, node);
      }
    }
    if (isFunction) {
      functionDepth--;
    }
  };
  visit(program, undefined);
  return { specifiers, dynamicSpecifiers, metaEdits, resolveEdits, topLevelAwait, unsafe };
}

const FUNCTIONS = new Set(['FunctionDeclaration', 'FunctionExpression', 'ArrowFunctionExpression']);

/** The replacement for one `import.meta.<property>` read in the module at `url`. */
export function metaReplacement(
  property: MetaEdit['property'],
  url: string,
  filename: string,
  dirname: string,
  isEntry: boolean,
): string | undefined {
  if (property === 'main') {
    // The entry module is in the entry chunk, which Node.js runs as the main module. Any
    // other module is not the main one: `false`, or `undefined` where Node.js has no
    // `import.meta.main`.
    return isEntry ? undefined : '(import.meta.main && false)';
  }
  return JSON.stringify(property === 'url' ? url : property === 'filename' ? filename : dirname);
}

export interface GraphModule {
  /** Why this module cannot be bundled, or `undefined`. */
  unsafe?: string;
  /** The resolved URLs of the module's imports. */
  imports: readonly string[];
}

/**
 * The modules the bundle may contain: the ES modules reachable from `entry` through
 * safe modules, less every module that something outside the bundle imports or
 * requires. A module outside the bundle loads from its own file at run time, so a
 * bundled copy of what it imports would be a second instance. Removing a module puts
 * its own imports outside the bundle's control too, so the removal repeats until no
 * edge enters the set from outside.
 *
 * `modules` holds the analyzed ES modules; anything else is outside the bundle.
 * `edges` adds `[parent, target]` pairs the analysis cannot see, such as the
 * `require()` calls a CommonJS module made in the recorded run.
 */
export function bundledSet(
  entry: string,
  modules: ReadonlyMap<string, GraphModule>,
  edges: Iterable<readonly [string, string]>,
  excluded: ReadonlySet<string>,
): Set<string> {
  const candidate = (url: string) => {
    const module = modules.get(url);
    return module !== undefined && module.unsafe === undefined && !excluded.has(url);
  };
  const set = new Set<string>();
  if (!candidate(entry)) {
    return set;
  }
  const queue = [entry];
  set.add(entry);
  while (queue.length > 0) {
    for (const target of modules.get(queue.pop()!)!.imports) {
      if (!set.has(target) && candidate(target)) {
        set.add(target);
        queue.push(target);
      }
    }
  }

  const targets = new Map<string, Set<string>>();
  const addEdge = (parent: string, target: string) => {
    let list = targets.get(parent);
    if (list === undefined) {
      targets.set(parent, (list = new Set()));
    }
    list.add(target);
  };
  for (const [url, module] of modules) {
    for (const target of module.imports) {
      addEdge(url, target);
    }
  }
  for (const [parent, target] of edges) {
    addEdge(parent, target);
  }

  // Every target of a parent outside the set leaves it, and so do that target's own
  // targets, until the set is closed.
  const removed: string[] = [];
  for (const [parent, list] of targets) {
    if (!set.has(parent)) {
      for (const target of list) {
        if (set.delete(target)) {
          removed.push(target);
        }
      }
    }
  }
  while (removed.length > 0) {
    for (const target of targets.get(removed.pop()!) ?? []) {
      if (set.delete(target)) {
        removed.push(target);
      }
    }
  }
  if (!set.has(entry)) {
    set.clear();
  }
  return set;
}

/**
 * The bundled modules that `entry` reaches through static imports alone. They all run at
 * startup, so the bundle keeps them in one chunk, in their source order; a module that
 * only `import()` reaches goes to a chunk of its own and runs when the `import()` does.
 */
export function staticallyReachable(
  entry: string,
  staticImports: ReadonlyMap<string, readonly string[]>,
  bundled: ReadonlySet<string>,
): Set<string> {
  const reached = new Set<string>();
  if (!bundled.has(entry)) {
    return reached;
  }
  const queue = [entry];
  reached.add(entry);
  while (queue.length > 0) {
    for (const target of staticImports.get(queue.pop()!) ?? []) {
      if (bundled.has(target) && !reached.has(target)) {
        reached.add(target);
        queue.push(target);
      }
    }
  }
  return reached;
}
