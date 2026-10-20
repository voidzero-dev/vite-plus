import assert from 'node:assert/strict';
import { test } from 'node:test';

import { prepareRustCache } from '../../actions/setup-rust/prepare-cache.mjs';

const active = 'nightly-2026-08-02-x86_64-unknown-linux-gnu';

function fixture(installed, env = {}) {
  const commands = [];
  const paths = prepareRustCache({
    env,
    home: '/home/runner',
    run(args) {
      commands.push(args);
      if (args[0] === 'show') {
        return `${active} (overridden by '/workspace/rust-toolchain.toml')`;
      }
      if (args[1] === 'list') {
        return installed.join('\n');
      }
      return '';
    },
  });
  return { commands, paths };
}

test('keeps the pinned compiler and removes unused runner toolchains', () => {
  const extra = ['stable-x86_64-unknown-linux-gnu', '1.99.0-x86_64-unknown-linux-gnu'];
  const { commands } = fixture([...extra, active]);
  assert.deepEqual(commands.slice(2), [
    ['default', active],
    ...extra.map((toolchain) => ['toolchain', 'uninstall', toolchain]),
  ]);
});

test('does not uninstall a toolchain when only the active one is installed', () => {
  const { commands } = fixture([active]);
  assert.deepEqual(commands.slice(2), [['default', active]]);
});

test('unset and explicit default paths produce the same cache environment', () => {
  const implicit = fixture([active]);
  const explicit = fixture([active], {
    CARGO_HOME: '/home/runner/.cargo',
    RUSTUP_HOME: '/home/runner/.rustup',
  });
  assert.deepEqual(implicit.paths, explicit.paths);
});

test('preserves custom Cargo and Rustup paths', () => {
  const paths = {
    CARGO_HOME: 'D:\\Dev Drive\\.cargo',
    RUSTUP_HOME: 'D:\\Dev Drive\\.rustup',
  };
  assert.deepEqual(fixture([active], paths).paths, paths);
});

test('fails before changing toolchains when the active compiler is missing', () => {
  const commands = [];
  assert.throws(
    () =>
      prepareRustCache({
        run(args) {
          commands.push(args);
          return args[0] === 'show' ? active : 'stable-x86_64-unknown-linux-gnu';
        },
      }),
    /must be installed/,
  );
  assert.equal(commands.length, 2);
});
