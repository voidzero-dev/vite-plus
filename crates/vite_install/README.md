# vite_install

- Auto-detects package manager type and version from package.json's `packageManager` field
- Downloads and caches the specified version
- Handles install, add, etc. commands for pnpm/yarn/npm.

## Testing

This crate includes both unit tests and integration/e2e tests:

- **Unit tests**: Run without network access, test core logic
- **E2E tests**: Marked with `#[ignore]`, require network to download real package managers

To run only unit tests (default):
```bash
cargo test -p vite_install
```

To run all tests including e2e tests:
```bash
cargo test -p vite_install -- --ignored --test-threads=1
```

Note: Use `--test-threads=1` when running e2e tests to prevent race conditions in concurrent downloads to the shared cache directory.
