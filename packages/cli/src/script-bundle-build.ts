/**
 * Builds the `vpx` bundle cache for one script (see `script-bundle.ts`). The preload
 * starts it at the end of a recorded run as `node dist/script-bundle-build.js <dir>`, in
 * the script's working directory and environment, so the native loader selects the same
 * tsconfig and transforms each module the same way as the run did.
 *
 * 1. Starting at the entry, load every ES module with the loader's own transform, parse
 *    it, and resolve its imports: from the trace when the run made the import, and with
 *    the loader's resolver otherwise (an `import()` the run did not reach).
 * 2. Pick the modules the bundle may contain (`bundledSet`).
 * 3. Bundle them with Rolldown. Each module keeps its own `import.meta.url`, `filename`,
 *    and `dirname`; every other import stays external, at its resolved `file:` URL.
 * 4. Write the chunks and a manifest of every input, stamped before it was read.
 */

import { readdirSync, readFileSync, renameSync, rmSync, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { ignoreInput, ignoreOutput } from '@voidzero-dev/vite-task-client';
import { RolldownMagicString, build } from 'vite/rolldown';
import { parseAst } from 'vite/rolldown/parseAst';

import type { LoadFnOutput, ResolveFnOutput } from '../binding/index.js';
import {
  analyzeModule,
  bundledSet,
  type GraphModule,
  type MetaEdit,
  metaReplacement,
  startupLayout,
  type ResolveEdit,
} from './script-bundle-plan.ts';
import {
  type BundleManifest,
  type BundleTrace,
  BUNDLE_CACHE_VERSION,
  debug,
  EXCLUDED,
  hash,
  isBuiltinSpecifier,
  isPlainDependency,
  MANIFEST,
  readJson,
  releaseBuildLock,
  RESOLVE_FROM_PREFIX,
  stamp,
  TRACE,
  writeJsonAtomic,
} from './script-bundle.ts';
import {
  HELPER_MODULE_NAME,
  loadOxcNode,
  runtimeHelperSpecifier,
  threadScriptConfig,
} from './script-hooks.ts';

interface Target {
  url: string;
  format: string | null;
}

interface ModuleRecord extends GraphModule {
  code: string;
  map: unknown;
  json: boolean;
  metaEdits: MetaEdit[];
  resolveEdits: ResolveEdit[];
  /** Specifier → resolved module, for the specifiers in the code. */
  targets: Map<string, Target>;
  /** The resolved URLs of the static imports and re-exports. */
  staticImports: string[];
  topLevelAwait: boolean;
  /** The module file and its source map, stamped before they were read. */
  stamps: [string, string][];
}

const dir = process.argv[2];
const started = performance.now();
const inputs = new Map<string, string>();
/** Every file the build read, stamped before, for a build that fails partway. */
const attempted: [string, string][] = [];
/** The tsconfig files the loader read, once the binding is set up. */
let takeLoaderInputs: (() => string[]) | undefined;

/** Record `file` as an input, stamped the first time it is seen. */
function addInput(file: string, fileStamp?: string): void {
  if (!inputs.has(file)) {
    inputs.set(file, fileStamp ?? stamp(file));
  }
}

function writeManifest(manifest: Omit<BundleManifest, 'version' | 'inputs'>): void {
  writeJsonAtomic(path.join(dir, MANIFEST), {
    version: BUNDLE_CACHE_VERSION,
    ...manifest,
    inputs: [...inputs],
  } satisfies BundleManifest);
}

const require = createRequire(import.meta.url);

async function buildBundle(trace: BundleTrace): Promise<void> {
  const oxcNode = loadOxcNode();
  // Report the tsconfig files the transforms read, which are inputs too.
  oxcNode.enableWatchReport();
  takeLoaderInputs = () => oxcNode.takeWatchDependencies();
  const tsconfig = threadScriptConfig();
  oxcNode.initializeLoader(tsconfig.path, tsconfig.source);
  oxcNode.setTransformDeps('typescript');

  const observed = new Map<string, Target>();
  const importFormat = new Map<string, string | null>();
  const requireFormat = new Map<string, string | null>();
  const loadedFormat = new Map<string, string | null>();
  const conditionsOf = new Map<string, string[]>();
  const runEdges: [parent: string, specifier: string, kind: string, url: string][] = [];
  for (const [parent, specifier, kind, conditions, url, format] of trace.resolves) {
    // `load` gets the format an `import` resolved to; a `require()` answer is the fallback.
    const formats = kind === 'import' ? importFormat : requireFormat;
    if (!formats.has(url)) {
      formats.set(url, format);
    }
    if (parent === null) {
      continue;
    }
    runEdges.push([parent, specifier, kind, url]);
    if (kind === 'import') {
      observed.set(`${parent}\n${specifier}`, { url, format });
      if (!conditionsOf.has(parent)) {
        conditionsOf.set(parent, trace.conditions[conditions]);
      }
    }
  }
  for (const [url, format] of trace.loads) {
    loadedFormat.set(url, format);
  }
  const entryConditions =
    trace.resolves.find(([parent, , kind]) => parent === null && kind === 'import')?.[3] ?? 0;
  const defaultConditions = trace.conditions[entryConditions] ?? ['node', 'import'];
  // oxc-node sets up its resolver, which `load` also needs, on the first resolve; a run
  // resolves the entry first too.
  void oxcNode.createResolve(
    trace.entry,
    { conditions: defaultConditions, importAttributes: {} },
    (url) => ({ url }),
  );

  /** An import the run did not make, resolved as the loader would resolve it. */
  const resolveUnobserved = (parentURL: string, specifier: string): Target | undefined => {
    if (isBuiltinSpecifier(specifier)) {
      return { url: specifier, format: 'builtin' };
    }
    const helper = runtimeHelperSpecifier(specifier);
    if (helper !== undefined) {
      // The loader resolves runtime helpers from this package, as `import` does here.
      return { url: import.meta.resolve(helper), format: null };
    }
    let result;
    try {
      result = oxcNode.createResolve(
        specifier,
        {
          parentURL,
          conditions: conditionsOf.get(parentURL) ?? defaultConditions,
          importAttributes: {},
        },
        (url) => ({ url }),
      ) as ResolveFnOutput;
    } catch {
      return undefined;
    }
    // A specifier oxc-node leaves to Node.js comes back unresolved.
    if (!result.url.startsWith('file:')) {
      return undefined;
    }
    const format = typeof result.format === 'string' ? result.format : null;
    if (!importFormat.has(result.url)) {
      importFormat.set(result.url, format);
    }
    return { url: result.url, format };
  };

  const decoder = new TextDecoder();
  /** The module as the loader runs it, or `undefined` when it is not an ES module. */
  const loadModule = (
    url: string,
  ): { code: string; format: string; map: unknown; stamps: [string, string][] } | undefined => {
    if (!url.startsWith('file:') || url.includes('?') || url.includes('#')) {
      return undefined;
    }
    const file = fileURLToPath(url);
    const stamps: [string, string][] = [[file, stamp(file)]];
    attempted.push(stamps[0]);
    if (isPlainDependency(url)) {
      // Dependencies run as published; only their format needs deciding.
      const format = loadedFormat.get(url) ?? dependencyFormat(file);
      if (format !== 'module') {
        return undefined;
      }
      const code = readFileSync(file, 'utf8').replace(/^﻿/, '');
      return { code, format, map: dependencyMap(code, file, stamps), stamps };
    }
    const format = importFormat.get(url) ?? requireFormat.get(url);
    if (typeof format !== 'string') {
      // Without a format, the loader would hand the file to Node.js untransformed.
      return undefined;
    }
    const result = oxcNode.load(
      url,
      { format, conditions: conditionsOf.get(url) ?? defaultConditions, importAttributes: {} },
      (loadURL, loadContext) => ({
        format: typeof loadContext?.format === 'string' ? loadContext.format : format,
        source: readFileSync(fileURLToPath(loadURL)),
      }),
      HELPER_MODULE_NAME,
    ) as LoadFnOutput;
    const source = result.source;
    if ((result.format !== 'module' && result.format !== 'json') || source == null) {
      return undefined;
    }
    const text = typeof source === 'string' ? source : decoder.decode(source);
    return { ...splitInlineMap(text), format: result.format, stamps };
  };

  const excluded = new Set((readJson(path.join(dir, EXCLUDED)) as string[] | undefined) ?? []);
  const modules = new Map<string, ModuleRecord>();
  const visited = new Set<string>();
  const queue = [trace.entry];
  while (queue.length > 0) {
    const url = queue.shift()!;
    if (visited.has(url)) {
      continue;
    }
    visited.add(url);
    let loaded;
    try {
      loaded = loadModule(url);
    } catch (error) {
      // The loader would fail on it too: an `import()` the run did not reach, which the
      // bundle leaves to the run.
      loaded = { code: '', format: 'module', map: null, stamps: [], error: describeError(error) };
    }
    if (loaded === undefined) {
      continue;
    }
    const json = loaded.format === 'json';
    const record: ModuleRecord = {
      code: loaded.code,
      map: loaded.map,
      json,
      metaEdits: [],
      resolveEdits: [],
      imports: [],
      targets: new Map(),
      staticImports: [],
      topLevelAwait: false,
      stamps: loaded.stamps,
    };
    modules.set(url, record);
    if ('error' in loaded) {
      record.unsafe = `it does not load: ${loaded.error}`;
      continue;
    }
    if (json) {
      continue;
    }
    let analysis;
    try {
      analysis = analyzeModule(
        parseAst(loaded.code, { lang: 'js', sourceType: 'module' }) as never,
      );
    } catch (error) {
      record.unsafe = `it does not parse: ${(error as Error).message.split('\n', 1)[0]}`;
      continue;
    }
    record.unsafe = analysis.unsafe;
    record.metaEdits = analysis.metaEdits;
    record.resolveEdits = analysis.resolveEdits;
    record.topLevelAwait = analysis.topLevelAwait;
    const resolve = (specifier: string): Target | undefined =>
      observed.get(`${url}\n${specifier}`) ?? resolveUnobserved(url, specifier);
    for (const specifier of analysis.staticSpecifiers) {
      const target = resolve(specifier);
      if (target === undefined) {
        // The module would fail to link, so the run cannot have loaded it like this.
        record.unsafe ??= `it imports ${specifier}, which does not resolve`;
        continue;
      }
      record.targets.set(specifier, target);
      record.staticImports.push(target.url);
    }
    for (const { specifier, start, end } of analysis.dynamicImports) {
      const target = resolve(specifier);
      if (target === undefined) {
        // An optional `import()` that fails today may work later: the run resolves it.
        record.resolveEdits.push({ start, end });
      } else {
        record.targets.set(specifier, target);
      }
    }
    if (record.unsafe === undefined) {
      for (const target of record.targets.values()) {
        if (target.url.startsWith('file:')) {
          queue.push(target.url);
        }
      }
    }
    record.imports = Array.from(record.targets.values(), (target) => target.url);
  }

  // The run's own edges. A `require()`, and an import whose specifier is not in the code
  // (a computed `import()`), resolve outside the bundle's control at run time, so they
  // count as edges from outside (`''`) and keep their targets out of the bundle.
  const edges: [string, string][] = runEdges.map(([parent, specifier, kind, url]) =>
    kind === 'import' && modules.get(parent)?.targets.has(specifier) !== false
      ? [parent, url]
      : ['', url],
  );
  const bundled = bundledSet(trace.entry, modules, edges, excluded);
  for (const [url, record] of modules) {
    if (record.unsafe !== undefined) {
      debug(`left out ${fileURLToPath(url)}: ${record.unsafe}`);
    }
  }
  if (bundled.size === 0) {
    throw new Error(modules.get(trace.entry)?.unsafe ?? 'the entry is not an ES module');
  }

  // What each bundled module is (stamped before it was read), then what resolving its
  // imports depended on.
  for (const url of bundled) {
    for (const [file, fileStamp] of modules.get(url)!.stamps) {
      addInput(file, fileStamp);
    }
  }
  for (const url of bundled) {
    const record = modules.get(url)!;
    const file = fileURLToPath(url);
    addPackageScope(file);
    for (const [specifier, target] of record.targets) {
      if (!target.url.startsWith('file:')) {
        continue;
      }
      const targetFile = fileURLToPath(target.url);
      const dependency = target.url.includes('/node_modules/');
      addInput(targetFile);
      addPackageScope(targetFile);
      if (specifier.startsWith('.') || specifier.startsWith('/')) {
        // A new file beside the target can take its place (`x.ts` for `./x.js`). An
        // installed package changes only through an install, which the package's own
        // files and the `node_modules` lookups record.
        if (!dependency) {
          addInput(path.dirname(targetFile));
        }
      } else if (isBareSpecifier(specifier)) {
        // Also when it resolved outside `node_modules`, through a workspace link or a
        // tsconfig `paths` alias: an install can put a package in that place.
        addPackageLookup(path.dirname(file), specifier);
      }
    }
  }
  for (const file of oxcNode.takeWatchDependencies()) {
    addInput(file);
  }

  const out = `out-${hash(`${process.pid}:${Date.now()}`)}`;
  const temp = path.join(dir, `${out}.tmp`);
  const entryFile = fileURLToPath(trace.entry);
  // The manifest lists the bundled modules in this order; rewritten specifiers refer to it.
  const order = [...bundled];
  const moduleIndex = new Map(order.map((url, index) => [url, index]));
  const layout = startupLayout(
    trace.entry,
    new Map(Array.from(modules, ([url, record]) => [url, record.staticImports])),
    bundled,
  );
  // A startup module that awaits holds its chunk open, and an `import()` it awaits can
  // need that chunk, or a later one that imports it: the bundle would wait for itself.
  // The entry has a chunk of its own, and every other startup chunk runs before it.
  for (const url of layout.modules) {
    if (url !== trace.entry && modules.get(url)!.topLevelAwait) {
      throw new Error(`${fileURLToPath(url)} awaits at its top level`);
    }
  }
  // Each external module that startup code imports goes through a bundled proxy in a
  // chunk of its own, so it runs between the chunks of the modules around it. A proxy id
  // ends in `.mjs`, so Rolldown parses it as an ES module whatever its target is.
  const proxyId = new Map(
    layout.externals.map((url, index) => [url, `${PROXY_PREFIX}${index}.mjs`]),
  );
  const proxyURL = new Map(Array.from(proxyId, ([url, id]) => [id, url]));
  const groupOf = new Map<string, string>();
  layout.segments.forEach((segment, index) => {
    for (const url of segment) {
      groupOf.set(fileURLToPath(url), `startup-${index}`);
    }
  });
  layout.externals.forEach((url, index) => groupOf.set(proxyId.get(url)!, `external-${index}`));
  const groupNames = [...new Set(groupOf.values())];
  const formatOf = (url: string) =>
    loadedFormat.get(url) ?? importFormat.get(url) ?? requireFormat.get(url) ?? null;

  const result = await build({
    input: { entry: entryFile },
    cwd: trace.cwd,
    platform: 'node',
    logLevel: 'silent',
    // Keep every module body and every call Node.js would run.
    treeshake: { moduleSideEffects: true, annotations: false, unknownGlobalSideEffects: true },
    plugins: [
      {
        name: 'vpx-bundle-cache',
        resolveId(specifier, importer) {
          if (importer === undefined) {
            return specifier === entryFile ? { id: entryFile, moduleSideEffects: true } : null;
          }
          if (specifier.startsWith(RESOLVE_FROM_PREFIX) || importer.startsWith(PROXY_PREFIX)) {
            // A rewritten specifier that Rolldown folded to a constant is still the run's,
            // and a proxy imports its module by URL.
            return { id: specifier, external: true };
          }
          const record = modules.get(pathToFileURL(importer).href);
          if (record === undefined) {
            // Rolldown's own runtime module.
            return null;
          }
          const target = record.targets.get(specifier);
          if (target === undefined) {
            throw new Error(`${importer} imports ${specifier}, which the plan does not cover`);
          }
          if (bundled.has(target.url)) {
            return { id: fileURLToPath(target.url), moduleSideEffects: true };
          }
          if (proxyId.has(target.url)) {
            return { id: proxyId.get(target.url)!, moduleSideEffects: true };
          }
          return { id: target.url, external: true };
        },
        load(id) {
          if (id.startsWith(PROXY_PREFIX)) {
            return proxyModule(proxyURL.get(id)!, formatOf);
          }
          const record = modules.get(pathToFileURL(id).href);
          if (record === undefined) {
            return null;
          }
          return {
            code: record.code,
            map: record.map as never,
            moduleType: record.json ? 'json' : 'js',
          };
        },
        transform(code, id) {
          const url = pathToFileURL(id).href;
          const record = modules.get(url);
          if (
            record === undefined ||
            (record.metaEdits.length === 0 && record.resolveEdits.length === 0)
          ) {
            return null;
          }
          if (code !== record.code) {
            throw new Error(`${id} changed between load and transform`);
          }
          const magic = new RolldownMagicString(code);
          for (const edit of record.metaEdits) {
            const replacement = metaReplacement(
              edit.property,
              url,
              id,
              path.dirname(id),
              url === trace.entry,
            );
            if (replacement !== undefined) {
              magic.overwrite(edit.start, edit.end, replacement);
            }
          }
          const prefix = JSON.stringify(`${RESOLVE_FROM_PREFIX}${moduleIndex.get(url)}:`);
          for (const edit of record.resolveEdits) {
            magic.prependLeft(edit.start, `${prefix} + (`);
            magic.appendRight(edit.end, ')');
          }
          // A map for every character, so that composing it with the loader's map keeps
          // stack trace columns.
          const map = magic.generateMap({ hires: true, source: id, includeContent: false });
          return { code: magic.toString(), map: map.toString() };
        },
      },
    ],
    output: {
      dir: temp,
      format: 'esm',
      entryFileNames: 'entry.mjs',
      chunkFileNames: 'chunk-[hash].mjs',
      // Startup code stays in source order without a wrapper per module: its segments and
      // proxies are chunks that the entry imports in that order. A module only `import()`
      // reaches goes to a chunk that loads when the `import()` runs. One file is enough
      // when there is neither.
      codeSplitting:
        layout.externals.length === 0 && layout.modules.size === bundled.size
          ? false
          : {
              groups: groupNames.map((name) => ({
                name,
                test: (id: string) => groupOf.get(id) === name,
              })),
            },
      sourcemap: true,
      sourcemapExcludeSources: true,
      keepNames: true,
    },
  });
  renameSync(temp, path.join(dir, out));
  const chunks = result.output.flatMap((file) => (file.type === 'chunk' ? [file.fileName] : []));
  writeManifest({ key: trace.key, out, entry: 'entry.mjs', chunks, modules: order });
  removeOldOutputs(out);
  debug(
    `bundled ${bundled.size} of ${modules.size} ES modules in ${Math.round(performance.now() - started)} ms`,
  );
}

/** The bundled stand-in for an external module that startup code imports. */
const PROXY_PREFIX = '\0vpx-external/';

/**
 * A module that imports `url` and passes on its exports. A JSON module keeps its import
 * attribute and has only a default export. A module without a default export gets
 * `undefined` for one, where an import of it would fail to link without the bundle.
 */
function proxyModule(url: string, formatOf: (url: string) => string | null): string {
  const target = JSON.stringify(url);
  if (formatOf(url) === 'json') {
    return `import * as ns from ${target} with { type: 'json' };\nexport default ns.default;\n`;
  }
  return `export * from ${target};\nimport * as ns from ${target};\nexport default ns.default;\n`;
}

/**
 * Remove older outputs, but keep the three newest of them: a run that started from one
 * still loads its `import()` chunks from it.
 */
function removeOldOutputs(current: string): void {
  const older = readdirSync(dir)
    .filter((name) => name.startsWith('out-') && name !== current)
    .map((name) => ({ name, time: statSync(path.join(dir, name)).mtimeMs }))
    .toSorted((a, b) => b.time - a.time);
  for (const { name } of older.slice(3)) {
    rmSync(path.join(dir, name), { recursive: true, force: true });
  }
}

/** The format Node.js gives a dependency file it runs as published. */
function dependencyFormat(file: string): string {
  if (file.endsWith('.mjs')) {
    return 'module';
  }
  if (!file.endsWith('.js')) {
    return 'commonjs';
  }
  for (let current = path.dirname(file); ;) {
    const manifest = readJson(path.join(current, 'package.json')) as { type?: string } | undefined;
    if (manifest !== undefined) {
      // A package without `"type"` may still hold ES modules that Node.js detects by
      // syntax; leaving them to Node.js keeps the answer exact.
      return manifest.type === 'module' ? 'module' : 'commonjs';
    }
    const parent = path.dirname(current);
    if (parent === current) {
      return 'commonjs';
    }
    current = parent;
  }
}

const INLINE_MAP = '\n//# sourceMappingURL=data:application/json;';

/** Split oxc-node's inline source map from the code it transformed. */
function splitInlineMap(source: string): { code: string; map: unknown } {
  const index = source.lastIndexOf(INLINE_MAP);
  if (index < 0) {
    return { code: source, map: null };
  }
  const data = source.slice(index + INLINE_MAP.length);
  const base64 = data.slice(data.indexOf('base64,') + 'base64,'.length).trim();
  try {
    const map = JSON.parse(Buffer.from(base64, 'base64').toString('utf8')) as {
      sources?: string[];
    };
    // oxc-node names each source by its `file:` URL, which Rolldown would take for a
    // relative path.
    map.sources = map.sources?.map((name) =>
      name.startsWith('file:') ? fileURLToPath(name) : name,
    );
    return { code: source.slice(0, index), map };
  } catch {
    return { code: source.slice(0, index), map: null };
  }
}

/** A dependency's own source map, so stack traces still reach its sources. */
function dependencyMap(code: string, file: string, stamps: [string, string][]): unknown {
  const match = /\/\/# sourceMappingURL=(\S+)\s*$/.exec(code);
  if (match === null) {
    return null;
  }
  try {
    const url = match[1];
    if (url.startsWith('data:')) {
      return JSON.parse(
        Buffer.from(url.slice(url.indexOf('base64,') + 7), 'base64').toString('utf8'),
      );
    }
    const mapFile = path.resolve(path.dirname(file), decodeURIComponent(url));
    stamps.push([mapFile, stamp(mapFile)]);
    return JSON.parse(readFileSync(mapFile, 'utf8'));
  } catch {
    return null;
  }
}

/**
 * The nearest `package.json` above `file`, which decides its format and its package's
 * `exports` and `imports`, and the project directories in between, where a new
 * `package.json` would take its place.
 */
const scopedDirectories = new Set<string>();
function addPackageScope(file: string): void {
  const between: string[] = [];
  // A directory seen before has its scope recorded already.
  for (let current = path.dirname(file); !scopedDirectories.has(current);) {
    scopedDirectories.add(current);
    const manifest = path.join(current, 'package.json');
    if (stamp(manifest) !== '') {
      addInput(manifest);
      break;
    }
    between.push(current);
    const parent = path.dirname(current);
    if (parent === current) {
      break;
    }
    current = parent;
  }
  for (const directory of between) {
    if (!directory.includes(`${path.sep}node_modules${path.sep}`)) {
      addInput(directory);
    }
  }
}

/** A package name, with or without a subpath: not relative, a URL, `#imports`, or a built-in. */
function isBareSpecifier(specifier: string): boolean {
  return (
    !specifier.startsWith('.') &&
    !specifier.startsWith('/') &&
    !specifier.startsWith('#') &&
    !/^[a-z][a-z0-9+.-]*:/i.test(specifier) &&
    !isBuiltinSpecifier(specifier)
  );
}

/**
 * The `node_modules` lookup for a bare specifier from `directory`: the package directory
 * it found, which a reinstall replaces, and each closer location that must stay empty.
 */
function addPackageLookup(directory: string, specifier: string): void {
  const parts = specifier.split('/');
  const name = specifier.startsWith('@') ? parts.slice(0, 2).join('/') : parts[0];
  for (let current = directory; ;) {
    if (path.basename(current) !== 'node_modules') {
      const candidate = path.join(current, 'node_modules', name);
      addInput(candidate);
      if (inputs.get(candidate) !== '') {
        return;
      }
    }
    const parent = path.dirname(current);
    if (parent === current) {
      return;
    }
    current = parent;
  }
}

// Under `vp run`, the cache is the loader's own state, as for the run that started this.
ignoreInput(path.dirname(dir));
ignoreOutput(path.dirname(dir));

// The loader's own files: a reinstall or an upgrade changes them.
for (const file of [
  fileURLToPath(new URL('../package.json', import.meta.url)),
  fileURLToPath(new URL('./script-preload.cjs', import.meta.url)),
  fileURLToPath(new URL('./script-bundle-hooks.cjs', import.meta.url)),
  fileURLToPath(import.meta.url),
  require.resolve('vite/package.json'),
]) {
  addInput(file);
}

const trace = readJson(path.join(dir, TRACE)) as BundleTrace | undefined;
try {
  if (trace === undefined || trace.version !== BUNDLE_CACHE_VERSION) {
    throw new Error('no trace to build from');
  }
  await buildBundle(trace);
} catch (error) {
  const reason = `the bundle could not be built: ${describeError(error)}`;
  debug(reason);
  if (trace !== undefined) {
    addInput(fileURLToPath(trace.entry));
    // A fix to any file the build read, or a new file beside one, can make it succeed.
    for (const [file, fileStamp] of attempted) {
      addInput(file, fileStamp);
      if (!file.includes(`${path.sep}node_modules${path.sep}`)) {
        addInput(path.dirname(file));
      }
    }
    for (const file of takeLoaderInputs?.() ?? []) {
      addInput(file);
    }
    writeManifest({ key: trace.key, disabled: reason });
  }
} finally {
  releaseBuildLock(dir);
}

/** The first line that says what failed; Rolldown starts with "Build failed with …". */
function describeError(error: unknown): string {
  const first = (error as { errors?: { message?: string }[] }).errors?.[0]?.message;
  const message = first ?? (error instanceof Error ? error.message : String(error));
  const line = message
    // oxlint-disable-next-line no-control-regex -- ANSI color codes
    .replace(/\u001b\[[0-9;]*m/g, '')
    .split('\n')
    .map((text) => text.trim())
    .find((text) => text !== '' && !/^Build failed with \d+ errors?:$/.test(text));
  return line ?? 'unknown error';
}
