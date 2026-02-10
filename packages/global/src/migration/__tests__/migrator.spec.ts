import { describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.js';
import { rewritePackageJson } from '../migrator.js';

describe('rewritePackageJson', () => {
  it('should rewrite package.json scripts', async () => {
    const pkg = {
      scripts: {
        test: 'vitest',
        test_run: 'vitest run && vitest --ui',
        lint: 'oxlint',
        lint_config: 'oxlint --config .oxlint.json',
        lint_type_aware: 'oxlint --type-aware',
        fmt: 'oxfmt',
        fmt_config: 'oxfmt --config .oxfmt.json',
        pack: 'tsdown',
        pack_watch: 'tsdown --watch',
        preview: 'vite preview',
        optimize: 'vite optimize',
        build: 'pnpm install &&vite build -r && vite run build --watch && tsdown && tsc || exit 1',
        dev: 'vite',
        dev_cjs: 'VITE_CJS_IGNORE_WARNING=true vite',
        dev_cjs_cross_env: 'cross-env VITE_CJS_IGNORE_WARNING=true vite',
        dev_help: 'vite --help && vite -h',
        dev_port: 'vite --port 3000',
        dev_host: 'vite --host 0.0.0.0',
        dev_open: 'vite --open',
        dev_verbose: 'vite --verbose',
        dev_debug: 'vite --debug',
        dev_trace: 'vite --trace',
        dev_profile: 'vite --profile',
        dev_stats: 'vite --stats',
        dev_analyze: 'vite --analyze',
        ready: 'oxlint --fix --type-aware && vitest run && tsdown && oxfmt --fix',
        ready_env:
          'NODE_ENV=test FOO=bar oxlint --fix --type-aware && NODE_ENV=test FOO=bar vitest run && NODE_ENV=test FOO=bar tsdown && NODE_ENV=test FOO=bar oxfmt --fix',
        ready_new:
          'vite install && vite fmt && vite lint --type-aware && vite test -r && vite build -r',
      },
      'lint-staged': {
        '*.js': ['oxlint --fix --type-aware', 'oxfmt --fix'],
        '*.ts': 'oxfmt --fix',
      },
    };
    rewritePackageJson(pkg, PackageManager.npm);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on standalone project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on pnpm monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm, true);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on npm monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.npm, true);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on yarn monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.yarn, true);
    expect(pkg).toMatchSnapshot();
  });
});
