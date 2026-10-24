import { chmodSync, mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vite-plus/test';

import {
  cargoCompletionConsumer,
  resolveCompletionConsumer,
} from '../../../tools/src/completion-consumer.ts';

const temporary: string[] = [];
afterEach(() => {
  for (const directory of temporary.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

function workspace() {
  const directory = mkdtempSync(path.join(os.tmpdir(), 'vp-consumer-selection-'));
  temporary.push(directory);
  return directory;
}

function artifact(root: string, relative: string, time = 100) {
  const name = process.platform === 'win32' ? 'vp.exe' : 'vp';
  const file = path.join(root, relative, name);
  mkdirSync(path.dirname(file), { recursive: true });
  // Path fixtures exercise handoff only; integration tests execute the real artifact.
  writeFileSync(file, 'Completion consumer path fixture');
  chmodSync(file, 0o755);
  utimesSync(file, time, time);
  return file;
}

function message(executable: string | null, extra = {}) {
  return JSON.stringify({
    reason: 'compiler-artifact',
    target: { name: 'vp', kind: ['bin'] },
    executable,
    ...extra,
  });
}

describe('completion Rust consumer handoff', () => {
  it('requires build provenance even when default artifacts and PATH entries exist', () => {
    const root = workspace();
    artifact(root, 'target/debug');
    expect(() => resolveCompletionConsumer(root, { PATH: root })).toThrow(
      'Missing VP_COMPLETION_CONSUMER_BIN',
    );
  });

  it('uses the supplied artifact despite newer old builds, other targets and PATH shadows', () => {
    const root = workspace();
    const supplied = artifact(root, 'custom/host/ci', 100);
    artifact(root, 'target/release', 900);
    artifact(root, 'target/other-triple/debug', 1000);
    const shadow = path.join(root, 'node_modules/.bin');
    mkdirSync(shadow, { recursive: true });
    writeFileSync(path.join(shadow, 'vp'), '#!/usr/bin/env node\nthrow new Error("Wrong entry");');
    expect(
      resolveCompletionConsumer(root, {
        VP_COMPLETION_CONSUMER_BIN: supplied,
        PATH: shadow,
        CARGO_TARGET_DIR: 'target',
        CARGO_BUILD_TARGET: 'other-triple',
      }),
    ).toBe(supplied);
  });

  it('does not inspect newer invalid foreign artifacts', () => {
    const root = workspace();
    const supplied = artifact(root, 'target/debug', 100);
    const foreign = artifact(root, 'target/foreign/release', 200);
    writeFileSync(foreign, '#!/bin/sh\nexit 93\n');
    expect(resolveCompletionConsumer(root, { VP_COMPLETION_CONSUMER_BIN: supplied })).toBe(
      supplied,
    );
  });

  it.each(['target/debug', 'target/host-triple/release', 'custom build/host-triple/ci'])(
    'accepts an exact build/restored path in %s without directory discovery',
    (layout) => {
      const root = workspace();
      const binary = artifact(root, layout);
      expect(
        resolveCompletionConsumer(root, {
          VP_COMPLETION_CONSUMER_BIN: path.relative(root, binary),
        }),
      ).toBe(binary);
      expect(resolveCompletionConsumer(root, { VP_COMPLETION_CONSUMER_BIN: binary })).toBe(binary);
    },
  );

  it('fails on a missing file or directory without borrowing a cached binary', () => {
    const root = workspace();
    artifact(root, 'target/debug');
    expect(() =>
      resolveCompletionConsumer(root, { VP_COMPLETION_CONSUMER_BIN: 'missing' }),
    ).toThrow('does not exist');
    expect(() => resolveCompletionConsumer(root, { VP_COMPLETION_CONSUMER_BIN: root })).toThrow(
      'does not exist',
    );
  });

  it.skipIf(process.platform === 'win32')(
    'rejects a supplied binary without execute permission',
    () => {
      const root = workspace();
      const binary = artifact(root, 'target/debug');
      chmodSync(binary, 0o644);
      expect(() => resolveCompletionConsumer(root, { VP_COMPLETION_CONSUMER_BIN: binary })).toThrow(
        'must be executable',
      );
    },
  );

  it('forwards the executable reported by Cargo including configured output dirs and fresh cached builds', () => {
    const root = workspace();
    const executable = path.join(root, 'configured output/host/custom-profile/vp');
    const output =
      [
        JSON.stringify({ reason: 'compiler-message' }),
        message(null),
        message('/irrelevant', { target: { name: 'vp-shim', kind: ['bin'] } }),
        message(executable, { fresh: true }),
        JSON.stringify({ reason: 'build-finished', success: true }),
      ].join('\n') + '\n';
    expect(cargoCompletionConsumer(output, root)).toBe(executable);
    expect(cargoCompletionConsumer(message(executable, { fresh: false }), root)).toBe(executable);
  });

  it('rejects missing, ambiguous or malformed Cargo artifact output', () => {
    const root = workspace();
    expect(() => cargoCompletionConsumer('', root)).toThrow('received 0');
    expect(() => cargoCompletionConsumer(message(null), root)).toThrow('received 0');
    expect(() =>
      cargoCompletionConsumer([message('/first/vp'), message('/second/vp')].join('\n'), root),
    ).toThrow('received 2');
    expect(() => cargoCompletionConsumer('invalid json', root)).toThrow();
  });
});
