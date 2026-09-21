import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

for (const file of process.argv.slice(2)) {
  const report = JSON.parse(readFileSync(file, 'utf8'));
  assert.equal(report.success, true);
  assert.equal(report.numPassedTests, 1);
  assert.equal(report.testResults.length, 1);
  const [test] = report.testResults[0].assertionResults;
  assert.equal(test.title, 'parse');
  assert.equal(test.status, 'passed');
  assert.equal(test.benchmarks.length, 1);
  const [benchmark] = test.benchmarks;
  assert.equal(benchmark.name, 'utilities > parse');
  assert.equal(benchmark.tasks.length, 1);
  assert.equal(benchmark.tasks[0].name, 'parse');
  assert.ok(benchmark.tasks[0].totalTime > 0);
  console.log(`${file}: migrated benchmark passed and JSON includes its result`);
}
