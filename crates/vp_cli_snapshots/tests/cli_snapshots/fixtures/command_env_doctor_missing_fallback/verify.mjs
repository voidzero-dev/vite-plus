import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";

const bin = path.join(process.env.VP_HOME, "bin");
const result = spawnSync(
  path.join(bin, process.platform === "win32" ? "vp.exe" : "vp"),
  ["env", "doctor", "node"],
  {
    encoding: "utf8",
    env: { ...process.env, PATH: bin },
  },
);
assert.equal(result.status, 1, `${result.stdout}\n${result.stderr}`);
const lines = result.stdout.replace(/\u001B\[[0-9;]*m/g, "").split(/\r?\n/);
const index = lines.findIndex((line) => line.includes("Fallback dir"));
assert.ok(index >= 0, result.stdout);
assert.match(lines[index], /not in PATH/);
assert.match(lines[index + 1], /note: Expected: .*[/\\]fallback-bin$/);
console.log(lines.slice(index, index + 2).join("\n"));
