import fs from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { expect, test } from 'vitest';

import { runCommand } from '../index.js';

test('should run command successfully', async () => {
  const result = await runCommand({
    binName: 'node',
    args: ['-e', 'console.log("Hello, world!")'],
    envs: {},
    cwd: process.cwd(),
  });
  expect(result).toMatchSnapshot();
});

// write file on the temp directory
test('should write file on the temp directory', async () => {
  const tempDir = await fs.realpath(await fs.mkdtemp(path.join(tmpdir(), 'vite-plus-test-')));
  const result = await runCommand({
    binName: 'node',
    args: ['-e', `fs.writeFileSync("test.txt", "Hello, world!")`],
    envs: {},
    cwd: tempDir,
  });
  expect(result).toMatchSnapshot();
  expect(await fs.readFile(path.join(tempDir, 'test.txt'), 'utf-8')).toBe('Hello, world!');
  await fs.rm(tempDir, { recursive: true });
});

// read file on the temp directory
test('should read file on the temp directory', async () => {
  const tempDir = await fs.realpath(await fs.mkdtemp(path.join(tmpdir(), 'vite-plus-test-')));
  await fs.writeFile(path.join(tempDir, 'test.txt'), 'Hello, world!');
  const result = await runCommand({
    binName: 'node',
    args: ['-e', `fs.readFileSync("test.txt", "utf-8")`],
    envs: {},
    cwd: tempDir,
  });
  expect(result).toMatchSnapshot();
  await fs.rm(tempDir, { recursive: true });
});

// write and read file on the temp directory
test('should write and read file on the temp directory', async () => {
  const tempDir = await fs.realpath(await fs.mkdtemp(path.join(tmpdir(), 'vite-plus-test-')));
  const result = await runCommand({
    binName: 'node',
    args: ['-e', `fs.writeFileSync("test.txt", "Hello, world!"); fs.readFileSync("test.txt", "utf-8")`],
    envs: {},
    cwd: tempDir,
  });
  expect(result).toMatchSnapshot();
  await fs.rm(tempDir, { recursive: true });
});
