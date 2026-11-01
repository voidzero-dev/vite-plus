/// <reference types="node" />

import { expect, onTestFinished, test, vi } from 'vite-plus/test';

import {
  findLatestStableVersionForMajor,
  getLatestNodeVersion,
  setViteTaskRevision,
} from '../upgrade-deps-utils.ts';

const VITE_TASK_GIT = 'git = "https://github.com/voidzero-dev/vite-task.git"';
const OLD_REV = 'a'.repeat(40);
const NEW_REV = 'b'.repeat(40);

test('selects the highest stable version from the supported major', () => {
  expect(
    findLatestStableVersionForMajor(
      ['3.2.4', '4.1.11', '5.0.0', '4.10.0', '4.11.0-beta.1', '4.2.12'],
      4,
    ),
  ).toBe('4.10.0');
});

test('returns undefined when the supported major has no stable release', () => {
  expect(findLatestStableVersionForMajor(['4.2.0-beta.1', '5.0.0'], 4)).toBeUndefined();
});

test('selects the latest stable Node.js 24 release independently of the LTS tag and index order', async () => {
  onTestFinished(() => {
    vi.unstubAllGlobals();
  });
  const fetchMock = vi.fn().mockResolvedValue(
    Response.json([
      { version: 'v26.13.0', lts: 'Lithium' },
      { version: 'v24.9.0', lts: 'Krypton' },
      { version: 'v24.11.0-rc.1', lts: false },
      { version: 'v24.10.2', lts: 'Krypton' },
      { version: 'v24.10.1', lts: 'Krypton' },
    ]),
  );
  vi.stubGlobal('fetch', fetchMock);
  await expect(getLatestNodeVersion(24)).resolves.toBe('24.10.2');
  expect(fetchMock).toHaveBeenCalledWith('https://nodejs.org/dist/index.json');
});

test.each([
  {
    response: () => new Response('Unavailable', { status: 503, statusText: 'Unavailable' }),
    error: 'Failed to fetch Node.js releases: 503 Unavailable',
  },
  {
    response: () => Response.json({ version: 'v24.10.2' }),
    error: 'Invalid Node.js release index: expected an array',
  },
  {
    response: () => Response.json([{ version: 'v26.13.0' }, { version: 'v24.10.0-rc.1' }]),
    error: 'No stable 24.x version found for Node.js',
  },
])('rejects an unusable Node.js release index: $error', async ({ response, error }) => {
  onTestFinished(() => {
    vi.unstubAllGlobals();
  });
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response()));
  await expect(getLatestNodeVersion(24)).rejects.toThrow(error);
});

test('moves every vite-task dependency to the new revision', () => {
  const cargoToml = (rev: string) =>
    [
      `fspy = { ${VITE_TASK_GIT}, rev = "${rev}" }`,
      `vt = { ${VITE_TASK_GIT}, rev = "${rev}", features = ["serde"] }`,
      `other = { git = "https://github.com/example/other.git", rev = "${OLD_REV}" }`,
      '# [patch."https://github.com/voidzero-dev/vite-task.git"]',
      '# vt = { path = "../vite-task/crates/vt" }',
      '',
    ].join('\n');
  expect(setViteTaskRevision(cargoToml(OLD_REV), NEW_REV)).toEqual({
    content: cargoToml(NEW_REV),
    previous: OLD_REV,
  });
});

test.each([
  { case: 'no vite-task dependency', lines: [] },
  {
    case: 'diverging revisions',
    lines: [
      `fspy = { ${VITE_TASK_GIT}, rev = "${OLD_REV}" }`,
      `vt = { ${VITE_TASK_GIT}, rev = "${'c'.repeat(40)}" }`,
    ],
  },
  {
    case: 'an entry without a pinned revision',
    lines: [
      `fspy = { ${VITE_TASK_GIT}, rev = "${OLD_REV}" }`,
      `vt = { ${VITE_TASK_GIT}, branch = "main" }`,
    ],
  },
])('rejects $case', ({ lines }) => {
  expect(() => setViteTaskRevision(lines.join('\n'), NEW_REV)).toThrow('one shared revision');
});
