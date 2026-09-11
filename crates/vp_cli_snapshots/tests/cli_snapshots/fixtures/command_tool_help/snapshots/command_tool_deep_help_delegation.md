# command_tool_deep_help_delegation

Help requests with additional arguments delegate to the underlying tool.

## `vp test --help --coverage`

```
vitest/4.1.11

Usage:
  $ vitest [...filters]

Commands:
  run [...filters]
  related [...filters]
  watch [...filters]
  dev [...filters]
  bench [...filters]
  init <project>
  list [...filters]
  [...filters]
  complete [shell]

For more info, run any command with the `--help` flag:
  $ vitest run --help
  $ vitest related --help
  $ vitest watch --help
  $ vitest dev --help
  $ vitest bench --help
  $ vitest init --help
  $ vitest list --help
  $ vitest --help
  $ vitest complete --help
  $ vitest --help --expand-help

Options:
  --coverage                                                 Enable coverage report. Use '--help --coverage' for more info.
  --coverage.provider <name>                                 Select the tool for coverage collection, available values are: "v8", "istanbul" and "custom"
  --coverage.enabled                                         Enables coverage collection. Can be overridden using the --coverage CLI option (default: false)
  --coverage.include <pattern>                               Files included in coverage as glob patterns. May be specified more than once when using multiple patterns. By default only files covered by tests are included.
  --coverage.exclude <pattern>                               Files to be excluded in coverage. May be specified more than once when using multiple extensions.
  --coverage.clean                                           Clean coverage results before running tests (default: true)
  --coverage.cleanOnRerun                                    Clean coverage report on watch rerun (default: true)
  --coverage.reportsDirectory <path>                         Directory to write coverage report to (default: ./coverage)
  --coverage.reporter <name>                                 Coverage reporters to use. Visit https://vitest.dev/config/coverage#coverage-reporter) for more information (default: ["text", "html", "clover", "json"]
  --coverage.reportOnFailure                                 Generate coverage report even when tests fail (default: false)
  --coverage.allowExternal                                   Collect coverage of files outside the project root (default: false)
  --coverage.skipFull                                        Do not show files with 100% statement, branch, and function coverage (default: false)
  --coverage.thresholds.100                                  Shortcut to set all coverage thresholds to 100 (default: false)
  --coverage.thresholds.perFile                              Check thresholds per file. See --coverage.thresholds.lines, --coverage.thresholds.functions, --coverage.thresholds.branches and --coverage.thresholds.statements for the actual thresholds (default: false)
  --coverage.thresholds.autoUpdate <boolean|function>        Update threshold values: "lines", "functions", "branches" and "statements" to configuration file when current coverage is above the configured thresholds (default: false)
  --coverage.thresholds.lines <number>                       Threshold for lines. Visit https://github.com/istanbuljs/nyc#coverage-thresholds for more information. This option is not available for custom providers
  --coverage.thresholds.functions <number>                   Threshold for functions. Visit https://github.com/istanbuljs/nyc#coverage-thresholds for more information. This option is not available for custom providers
  --coverage.thresholds.branches <number>                    Threshold for branches. Visit https://github.com/istanbuljs/nyc#coverage-thresholds for more information. This option is not available for custom providers
  --coverage.thresholds.statements <number>                  Threshold for statements. Visit https://github.com/istanbuljs/nyc#coverage-thresholds for more information. This option is not available for custom providers
  --coverage.ignoreClassMethods <name>                       Array of class method names to ignore for coverage. Visit https://github.com/istanbuljs/nyc#ignoring-methods) for more information. This option is only available for the istanbul providers (default: []
  --coverage.processingConcurrency <number>                  Concurrency limit used when processing the coverage results. (default min between 20 and the number of CPUs)
  --coverage.customProviderModule <path>                     Specifies the module name or path for the custom coverage provider module. Visit https://vitest.dev/guide/coverage#custom-coverage-provider for more information. This option is only available for custom providers
  --coverage.watermarks.statements <watermarks>              High and low watermarks for statements in the format of <high>,<low>
  --coverage.watermarks.lines <watermarks>                   High and low watermarks for lines in the format of <high>,<low>
  --coverage.watermarks.branches <watermarks>                High and low watermarks for branches in the format of <high>,<low>
  --coverage.watermarks.functions <watermarks>               High and low watermarks for functions in the format of <high>,<low>
  --coverage.changed <commit/branch>                         Collect coverage only for files changed since a specified commit or branch (e.g., origin/main or HEAD~1). Inherits value from --changed by default.
  --coverage.excludeAfterRemap                               Apply exclusions again after coverage has been remapped to original sources. (default: false)
  --coverage.htmlDir <path>                                  Directory of HTML coverage output to be served in UI mode and HTML reporter.
```
