# command_fmt_help

## `vp fmt -h`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp fmt [PATH]... [OPTIONS]

Format code.
Options are forwarded to Oxfmt.

Available positional items:
  [PATH]...  Single file, path or list of paths. Glob patterns are also supported. (Be sure to quote them, otherwise your shell may expand them before passing.) Exclude patterns with `!` prefix like `'!**/fixtures/*.js'` are also supported. If not provided, current working directory is used.

Mode Options:
  --stdin-filepath=PATH  Specify the file name to use to infer which parser to use

Output Options:
  --write           Format and write files in place (default)
  --check           Check if files are formatted, also show statistics
  --list-different  List files that would be changed

Ignore Options:
  --ignore-path=PATH   Path to ignore file(s). Can be specified multiple times. If not specified, .gitignore and .prettierignore in the current directory are used.
  --with-node-modules  Format code in node_modules directory (skipped by default)

Runtime Options:
  --no-error-on-unmatched-pattern  Do not exit with error when pattern is unmatched
  --threads=INT                    Number of threads to use. Set to 1 for using only 1 CPU core.

Available options:
  -h, --help  Prints help information

Examples:
  vp fmt
  vp fmt src --check
  vp fmt . --write

Documentation: https://viteplus.dev/guide/fmt
```

## `vp fmt --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp fmt [PATH]... [OPTIONS]

Format code.
Options are forwarded to Oxfmt.

Available positional items:
  [PATH]...  Single file, path or list of paths. Glob patterns are also supported. (Be sure to quote them, otherwise your shell may expand them before passing.) Exclude patterns with `!` prefix like `'!**/fixtures/*.js'` are also supported. If not provided, current working directory is used.

Mode Options:
  --stdin-filepath=PATH  Specify the file name to use to infer which parser to use

Output Options:
  --write           Format and write files in place (default)
  --check           Check if files are formatted, also show statistics
  --list-different  List files that would be changed

Ignore Options:
  --ignore-path=PATH   Path to ignore file(s). Can be specified multiple times. If not specified, .gitignore and .prettierignore in the current directory are used.
  --with-node-modules  Format code in node_modules directory (skipped by default)

Runtime Options:
  --no-error-on-unmatched-pattern  Do not exit with error when pattern is unmatched
  --threads=INT                    Number of threads to use. Set to 1 for using only 1 CPU core.

Available options:
  -h, --help  Prints help information

Examples:
  vp fmt
  vp fmt src --check
  vp fmt . --write

Documentation: https://viteplus.dev/guide/fmt
```

## `vp help fmt`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp fmt [PATH]... [OPTIONS]

Format code.
Options are forwarded to Oxfmt.

Available positional items:
  [PATH]...  Single file, path or list of paths. Glob patterns are also supported. (Be sure to quote them, otherwise your shell may expand them before passing.) Exclude patterns with `!` prefix like `'!**/fixtures/*.js'` are also supported. If not provided, current working directory is used.

Mode Options:
  --stdin-filepath=PATH  Specify the file name to use to infer which parser to use

Output Options:
  --write           Format and write files in place (default)
  --check           Check if files are formatted, also show statistics
  --list-different  List files that would be changed

Ignore Options:
  --ignore-path=PATH   Path to ignore file(s). Can be specified multiple times. If not specified, .gitignore and .prettierignore in the current directory are used.
  --with-node-modules  Format code in node_modules directory (skipped by default)

Runtime Options:
  --no-error-on-unmatched-pattern  Do not exit with error when pattern is unmatched
  --threads=INT                    Number of threads to use. Set to 1 for using only 1 CPU core.

Available options:
  -h, --help  Prints help information

Examples:
  vp fmt
  vp fmt src --check
  vp fmt . --write

Documentation: https://viteplus.dev/guide/fmt
```

## `vp format --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp fmt [PATH]... [OPTIONS]

Format code.
Options are forwarded to Oxfmt.

Available positional items:
  [PATH]...  Single file, path or list of paths. Glob patterns are also supported. (Be sure to quote them, otherwise your shell may expand them before passing.) Exclude patterns with `!` prefix like `'!**/fixtures/*.js'` are also supported. If not provided, current working directory is used.

Mode Options:
  --stdin-filepath=PATH  Specify the file name to use to infer which parser to use

Output Options:
  --write           Format and write files in place (default)
  --check           Check if files are formatted, also show statistics
  --list-different  List files that would be changed

Ignore Options:
  --ignore-path=PATH   Path to ignore file(s). Can be specified multiple times. If not specified, .gitignore and .prettierignore in the current directory are used.
  --with-node-modules  Format code in node_modules directory (skipped by default)

Runtime Options:
  --no-error-on-unmatched-pattern  Do not exit with error when pattern is unmatched
  --threads=INT                    Number of threads to use. Set to 1 for using only 1 CPU core.

Available options:
  -h, --help  Prints help information

Examples:
  vp fmt
  vp fmt src --check
  vp fmt . --write

Documentation: https://viteplus.dev/guide/fmt
```
