# command_lint_help

## `vp lint -h`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp lint [PATH]... [OPTIONS]

Lint code.
Options are forwarded to Oxlint.

Arguments:
  [PATH]...  Files or directories to lint

Basic Configuration:
  --tsconfig <PATH>  Override the TypeScript config used for import resolution
  --init             Initialize lint configuration in vite.config.ts with Vite+ defaults

Rule Severity:
  -A, --allow <NAME>  Allow a rule or category
  -W, --warn <NAME>   Emit a warning for a rule or category
  -D, --deny <NAME>   Emit an error for a rule or category

Plugins:
  --disable-unicorn-plugin     Disable the unicorn plugin, which is enabled by default
  --disable-oxc-plugin         Disable Oxc-specific rules, which are enabled by default
  --disable-typescript-plugin  Disable the TypeScript plugin, which is enabled by default
  --import-plugin              Enable the import plugin
  --react-plugin               Enable the React plugin
  --jsdoc-plugin               Enable the JSDoc plugin
  --jest-plugin                Enable the Jest plugin
  --vitest-plugin              Enable the Vitest plugin
  --jsx-a11y-plugin            Enable the JSX accessibility plugin
  --nextjs-plugin              Enable the Next.js plugin
  --react-perf-plugin          Enable the React performance plugin
  --promise-plugin             Enable the promise plugin
  --node-plugin                Enable the Node.js plugin
  --vue-plugin                 Enable the Vue plugin

Fix Problems:
  --fix              Fix issues when possible
  --fix-suggestions  Apply auto-fixable suggestions
  --fix-dangerously  Apply dangerous fixes and suggestions

Ignore Files:
  --ignore-path <PATH>        Use the specified .eslintignore file
  --ignore-pattern <PATTERN>  Add file patterns to ignore
  --no-ignore                 Disable file exclusion from ignore rules

Handle Warnings:
  --quiet               Report errors only
  --deny-warnings       Exit non-zero when warnings are reported
  --max-warnings <INT>  Set the warning threshold before exiting non-zero

Output:
  -f, --format <FORMAT>  Set output format: checkstyle, default, agent, github, gitlab, json, junit, sarif, stylish, or unix
  --debug <OPTIONS>      Enable comma-separated debug output options: files or timings

Miscellaneous:
  --silent                         Do not display diagnostics
  --no-error-on-unmatched-pattern  Do not exit with an error when no files are selected for linting
  --threads <INT>                  Number of threads to use; set to 1 to use one CPU core
  --print-config                   Print the resolved configuration without linting

Inline Configuration:
  --report-unused-disable-directives                      Report unused oxlint-disable directives
  --report-unused-disable-directives-severity <SEVERITY>  Report unused disable directives at the specified severity

Options:
  --rules       List all registered rules
  --lsp         Start the language server
  --type-aware  Enable rules requiring type information
  --type-check  Enable experimental type checking and compiler diagnostics
  -h, --help    Print help information

Examples:
  vp lint
  vp lint src --fix
  vp lint --type-aware --tsconfig ./tsconfig.json

Documentation: https://viteplus.dev/guide/lint
```

## `vp lint --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp lint [PATH]... [OPTIONS]

Lint code.
Options are forwarded to Oxlint.

Arguments:
  [PATH]...  Files or directories to lint

Basic Configuration:
  --tsconfig <PATH>  Override the TypeScript config used for import resolution
  --init             Initialize lint configuration in vite.config.ts with Vite+ defaults

Rule Severity:
  -A, --allow <NAME>  Allow a rule or category
  -W, --warn <NAME>   Emit a warning for a rule or category
  -D, --deny <NAME>   Emit an error for a rule or category

Plugins:
  --disable-unicorn-plugin     Disable the unicorn plugin, which is enabled by default
  --disable-oxc-plugin         Disable Oxc-specific rules, which are enabled by default
  --disable-typescript-plugin  Disable the TypeScript plugin, which is enabled by default
  --import-plugin              Enable the import plugin
  --react-plugin               Enable the React plugin
  --jsdoc-plugin               Enable the JSDoc plugin
  --jest-plugin                Enable the Jest plugin
  --vitest-plugin              Enable the Vitest plugin
  --jsx-a11y-plugin            Enable the JSX accessibility plugin
  --nextjs-plugin              Enable the Next.js plugin
  --react-perf-plugin          Enable the React performance plugin
  --promise-plugin             Enable the promise plugin
  --node-plugin                Enable the Node.js plugin
  --vue-plugin                 Enable the Vue plugin

Fix Problems:
  --fix              Fix issues when possible
  --fix-suggestions  Apply auto-fixable suggestions
  --fix-dangerously  Apply dangerous fixes and suggestions

Ignore Files:
  --ignore-path <PATH>        Use the specified .eslintignore file
  --ignore-pattern <PATTERN>  Add file patterns to ignore
  --no-ignore                 Disable file exclusion from ignore rules

Handle Warnings:
  --quiet               Report errors only
  --deny-warnings       Exit non-zero when warnings are reported
  --max-warnings <INT>  Set the warning threshold before exiting non-zero

Output:
  -f, --format <FORMAT>  Set output format: checkstyle, default, agent, github, gitlab, json, junit, sarif, stylish, or unix
  --debug <OPTIONS>      Enable comma-separated debug output options: files or timings

Miscellaneous:
  --silent                         Do not display diagnostics
  --no-error-on-unmatched-pattern  Do not exit with an error when no files are selected for linting
  --threads <INT>                  Number of threads to use; set to 1 to use one CPU core
  --print-config                   Print the resolved configuration without linting

Inline Configuration:
  --report-unused-disable-directives                      Report unused oxlint-disable directives
  --report-unused-disable-directives-severity <SEVERITY>  Report unused disable directives at the specified severity

Options:
  --rules       List all registered rules
  --lsp         Start the language server
  --type-aware  Enable rules requiring type information
  --type-check  Enable experimental type checking and compiler diagnostics
  -h, --help    Print help information

Examples:
  vp lint
  vp lint src --fix
  vp lint --type-aware --tsconfig ./tsconfig.json

Documentation: https://viteplus.dev/guide/lint
```

## `vp help lint`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp lint [PATH]... [OPTIONS]

Lint code.
Options are forwarded to Oxlint.

Arguments:
  [PATH]...  Files or directories to lint

Basic Configuration:
  --tsconfig <PATH>  Override the TypeScript config used for import resolution
  --init             Initialize lint configuration in vite.config.ts with Vite+ defaults

Rule Severity:
  -A, --allow <NAME>  Allow a rule or category
  -W, --warn <NAME>   Emit a warning for a rule or category
  -D, --deny <NAME>   Emit an error for a rule or category

Plugins:
  --disable-unicorn-plugin     Disable the unicorn plugin, which is enabled by default
  --disable-oxc-plugin         Disable Oxc-specific rules, which are enabled by default
  --disable-typescript-plugin  Disable the TypeScript plugin, which is enabled by default
  --import-plugin              Enable the import plugin
  --react-plugin               Enable the React plugin
  --jsdoc-plugin               Enable the JSDoc plugin
  --jest-plugin                Enable the Jest plugin
  --vitest-plugin              Enable the Vitest plugin
  --jsx-a11y-plugin            Enable the JSX accessibility plugin
  --nextjs-plugin              Enable the Next.js plugin
  --react-perf-plugin          Enable the React performance plugin
  --promise-plugin             Enable the promise plugin
  --node-plugin                Enable the Node.js plugin
  --vue-plugin                 Enable the Vue plugin

Fix Problems:
  --fix              Fix issues when possible
  --fix-suggestions  Apply auto-fixable suggestions
  --fix-dangerously  Apply dangerous fixes and suggestions

Ignore Files:
  --ignore-path <PATH>        Use the specified .eslintignore file
  --ignore-pattern <PATTERN>  Add file patterns to ignore
  --no-ignore                 Disable file exclusion from ignore rules

Handle Warnings:
  --quiet               Report errors only
  --deny-warnings       Exit non-zero when warnings are reported
  --max-warnings <INT>  Set the warning threshold before exiting non-zero

Output:
  -f, --format <FORMAT>  Set output format: checkstyle, default, agent, github, gitlab, json, junit, sarif, stylish, or unix
  --debug <OPTIONS>      Enable comma-separated debug output options: files or timings

Miscellaneous:
  --silent                         Do not display diagnostics
  --no-error-on-unmatched-pattern  Do not exit with an error when no files are selected for linting
  --threads <INT>                  Number of threads to use; set to 1 to use one CPU core
  --print-config                   Print the resolved configuration without linting

Inline Configuration:
  --report-unused-disable-directives                      Report unused oxlint-disable directives
  --report-unused-disable-directives-severity <SEVERITY>  Report unused disable directives at the specified severity

Options:
  --rules       List all registered rules
  --lsp         Start the language server
  --type-aware  Enable rules requiring type information
  --type-check  Enable experimental type checking and compiler diagnostics
  -h, --help    Print help information

Examples:
  vp lint
  vp lint src --fix
  vp lint --type-aware --tsconfig ./tsconfig.json

Documentation: https://viteplus.dev/guide/lint
```
