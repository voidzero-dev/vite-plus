import type * as t from '@oxc-project/types';

import { SourceEditor, isString, objectProperty, staticObject } from './ast.ts';

const REPORTERS = new Set([
  'default',
  'verbose',
  'dot',
  'json',
  'junit',
  'tap',
  'tap-flat',
  'hanging-process',
  'github-actions',
]);

function reporterNames(node: t.Node): string[] | undefined {
  const entries = node.type === 'ArrayExpression' ? node.elements : [node];
  return entries.every(isString) ? entries.map((entry) => entry.value) : undefined;
}

/** Move only resolved built-in output settings. The normal config pass reports
 * any retained legacy options as blockers; no partial output plan is applied. */
export function migrateBenchmarkConfig(editor: SourceEditor, test: t.ObjectExpression): void {
  const benchmark = objectProperty(test, 'benchmark')?.value;
  if (!staticObject(benchmark)) {
    return;
  }
  const oldReporters = objectProperty(benchmark, 'reporters');
  const oldOutput = objectProperty(benchmark, 'outputFile');
  const oldJson = objectProperty(benchmark, 'outputJson');
  if (!oldReporters && !oldOutput && !oldJson) {
    return;
  }
  const benchmarkNames = oldReporters ? reporterNames(oldReporters.value) : ['default'];
  if (
    !benchmarkNames?.length ||
    benchmarkNames.some((name) => !['default', 'verbose'].includes(name))
  ) {
    return;
  }
  const destinations = [oldOutput, oldJson].filter((prop) => prop !== undefined);
  if (destinations.some((prop) => !isString(prop.value) || !prop.value.value)) {
    return;
  }
  const destination = (destinations[0]?.value as t.StringLiteral | undefined)?.value;
  if (destinations.some((prop) => (prop.value as t.StringLiteral).value !== destination)) {
    return;
  }
  const reporters = objectProperty(test, 'reporters');
  const names = reporters ? reporterNames(reporters.value) : benchmarkNames;
  if (!names || names.some((name) => !REPORTERS.has(name))) {
    return;
  }
  const terminal = names.filter((name) => ['default', 'verbose', 'dot'].includes(name));
  if (reporters && oldReporters && terminal.some((name) => !benchmarkNames.includes(name))) {
    return;
  }
  const output = objectProperty(test, 'outputFile');
  const jsonOutput =
    output &&
    (staticObject(output.value) ? objectProperty(output.value, 'json')?.value : output.value);
  if (destination) {
    if (
      (jsonOutput && (!isString(jsonOutput) || jsonOutput.value !== destination)) ||
      (output && !isString(output.value) && !staticObject(output.value))
    ) {
      return;
    }
    // A pre-existing JSON reporter without a file writes to stdout in v4.
    // Do not redirect that stream to the benchmark's file during consolidation.
    if (names.includes('json') && !jsonOutput) {
      return;
    }
  }
  const combined = [
    ...new Set([
      ...names,
      ...(terminal.length && !oldReporters ? [] : benchmarkNames),
      ...(destination ? ['json'] : []),
    ]),
  ];
  if (reporters) {
    if (combined.join('\0') !== names.join('\0')) {
      // Refuse to discard comments while replacing a literal reporter list.
      if (
        editor.comments.some(
          ({ start, end }) => start >= reporters.value.start && end <= reporters.value.end,
        )
      ) {
        return;
      }
      editor.replace(reporters.value, JSON.stringify(combined));
    }
  } else {
    editor.add(test, 'reporters', JSON.stringify(combined));
  }
  if (destination && !jsonOutput) {
    if (output && staticObject(output.value)) {
      editor.add(output.value, 'json', JSON.stringify(destination));
    } else {
      editor.add(test, 'outputFile', `{ json: ${JSON.stringify(destination)} }`);
    }
  }
  for (const prop of [oldReporters, oldOutput, oldJson]) {
    if (prop) {
      editor.remove(benchmark, prop);
    }
  }
  if (destination) {
    editor.report(
      oldJson ?? oldOutput,
      'benchmark-output',
      'Review consumers of this benchmark JSON file: the v5 JSON reporter includes test results and per-test benchmarks, not the v4 baseline format.',
    );
  }
}
