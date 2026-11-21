import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const result = spawnSync(
  process.execPath,
  [fileURLToPath(import.meta.resolve("vite-plus/bin")), "check", "--no-fmt", "--format=json", "error.js"],
  { encoding: "utf8" },
);
assert.equal(result.error, undefined);
assert.equal(result.status, 1, result.stderr);
assert.equal(result.stderr, "");
const report = JSON.parse(result.stdout);
assert.equal(report.diagnostics.length, 1);
assert.equal(report.diagnostics[0].code, "eslint(no-eval)");
assert.equal(report.diagnostics[0].severity, "error");
assert.equal(report.number_of_files, 1);
assert.ok(report.number_of_rules > 0);
assert.ok(report.threads_count > 0);
assert.ok(report.start_time >= 0);
console.log("Lint exited with status 1 and valid JSON:");
console.log(JSON.stringify(report.diagnostics, null, 2));
