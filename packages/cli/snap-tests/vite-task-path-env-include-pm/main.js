import assert from 'node:assert/strict';
import { execSync } from 'node:child_process';

const version = execSync('yarn --version').toString().trim();
assert.equal(version, '4.12.0');
