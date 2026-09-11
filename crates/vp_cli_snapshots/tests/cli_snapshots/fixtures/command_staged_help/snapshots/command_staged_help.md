# command_staged_help

## `vp staged -h`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp staged [OPTIONS]

Run linters on staged files using staged config from vite.config.ts.

Options:
  --allow-empty                        Allow empty commits when tasks revert all staged changes
  -p, --concurrent [<number|boolean>]  Run tasks at the same time. Use false to run one task at a time
  --no-concurrent                      Run one task at a time
  --continue-on-error                  Run all tasks to completion even if one fails
  --cwd <path>                         Working directory to run all tasks in
  -d, --debug                          Enable debug output
  --diff <string>                      Override the default --staged flag of git diff
  --diff-filter <string>               Override the default --diff-filter=ACMR flag of git diff
  --fail-on-changes                    Fail with exit code 1 when tasks modify tracked files
  --hide-partially-staged              Hide unstaged changes from partially staged files
  --hide-unstaged                      Hide all unstaged changes before running tasks
  --no-stash                           Disable the backup stash
  -q, --quiet                          Disable console output
  -r, --relative                       Pass filepaths relative to cwd to tasks
  --revert                             Revert to original state in case of errors
  -v, --verbose                        Show task output even when tasks succeed
  -h, --help                           Show this help message

Documentation: https://viteplus.dev/guide/commit-hooks
```

## `vp staged --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp staged [OPTIONS]

Run linters on staged files using staged config from vite.config.ts.

Options:
  --allow-empty                        Allow empty commits when tasks revert all staged changes
  -p, --concurrent [<number|boolean>]  Run tasks at the same time. Use false to run one task at a time
  --no-concurrent                      Run one task at a time
  --continue-on-error                  Run all tasks to completion even if one fails
  --cwd <path>                         Working directory to run all tasks in
  -d, --debug                          Enable debug output
  --diff <string>                      Override the default --staged flag of git diff
  --diff-filter <string>               Override the default --diff-filter=ACMR flag of git diff
  --fail-on-changes                    Fail with exit code 1 when tasks modify tracked files
  --hide-partially-staged              Hide unstaged changes from partially staged files
  --hide-unstaged                      Hide all unstaged changes before running tasks
  --no-stash                           Disable the backup stash
  -q, --quiet                          Disable console output
  -r, --relative                       Pass filepaths relative to cwd to tasks
  --revert                             Revert to original state in case of errors
  -v, --verbose                        Show task output even when tasks succeed
  -h, --help                           Show this help message

Documentation: https://viteplus.dev/guide/commit-hooks
```

## `vp help staged`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp staged [OPTIONS]

Run linters on staged files using staged config from vite.config.ts.

Options:
  --allow-empty                        Allow empty commits when tasks revert all staged changes
  -p, --concurrent [<number|boolean>]  Run tasks at the same time. Use false to run one task at a time
  --no-concurrent                      Run one task at a time
  --continue-on-error                  Run all tasks to completion even if one fails
  --cwd <path>                         Working directory to run all tasks in
  -d, --debug                          Enable debug output
  --diff <string>                      Override the default --staged flag of git diff
  --diff-filter <string>               Override the default --diff-filter=ACMR flag of git diff
  --fail-on-changes                    Fail with exit code 1 when tasks modify tracked files
  --hide-partially-staged              Hide unstaged changes from partially staged files
  --hide-unstaged                      Hide all unstaged changes before running tasks
  --no-stash                           Disable the backup stash
  -q, --quiet                          Disable console output
  -r, --relative                       Pass filepaths relative to cwd to tasks
  --revert                             Revert to original state in case of errors
  -v, --verbose                        Show task output even when tasks succeed
  -h, --help                           Show this help message

Documentation: https://viteplus.dev/guide/commit-hooks
```
