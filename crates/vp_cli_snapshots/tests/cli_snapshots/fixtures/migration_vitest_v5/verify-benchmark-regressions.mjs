import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';

const [file, ...names] = process.argv.slice(2);
const report = JSON.parse(readFileSync('benchmark.json', 'utf8'));
assert.equal(report.success, true);
assert.equal(report.numPassedTests, names.length);
assert.equal(report.testResults.length, 1);
const [result] = report.testResults;
assert.equal(path.normalize(result.name), path.resolve(file));
assert.deepEqual(result.assertionResults.map(test => test.title), names);
for (const test of result.assertionResults) {
  assert.equal(test.status, 'passed');
  assert.equal(test.benchmarks.length, 1);
  assert.equal(test.benchmarks[0].tasks.length, 1);
  assert.equal(test.benchmarks[0].tasks[0].name, test.title);
  assert.ok(test.benchmarks[0].tasks[0].totalTime > 0);
}
console.log(`Vitest 5 executed ${names.length} migrated benchmarks in ${file}`);
