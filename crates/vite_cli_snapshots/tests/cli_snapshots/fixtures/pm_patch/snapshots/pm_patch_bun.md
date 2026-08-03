# pm_patch_bun

## `vp pm patch placeholder -- --help`

Bun receives the patch command and prints its usage

```

Usage: bun patch [flags or options] <package>@<version>

  Prepare a package for patching, or generate and save a patch.

Flags:
  -c, --config=<val>                 Specify path to config file (bunfig.toml)
  -y, --yarn                         Write a yarn.lock file (yarn v1)
  -p, --production                   Don't install devDependencies
      --no-save                      Don't update package.json or save a lockfile
      --save                         Save to package.json (true by default)
      --ca=<val>                     Provide a Certificate Authority signing certificate
      --cafile=<val>                 The same as `--ca`, but is a file path to the certificate
      --dry-run                      Perform a dry run without making changes
      --frozen-lockfile              Disallow changes to lockfile
  -f, --force                        Always request the latest versions from the registry & reinstall all dependencies
      --cache-dir=<val>              Store & load cached data from a specific directory path
      --no-cache                     Ignore manifest cache entirely
      --silent                       Don't log anything
      --quiet                        Only show tarball name when packing
      --verbose                      Excessively verbose logging
      --no-progress                  Disable the progress bar
      --no-summary                   Don't print a summary
      --no-verify                    Skip verifying integrity of newly downloaded packages
      --ignore-scripts               Skip lifecycle scripts in the project's package.json (dependency scripts are never run)
      --trust                        Add to trustedDependencies in the project's package.json and install the package(s)
  -g, --global                       Install globally
      --cwd=<val>                    Set a specific cwd
      --backend=<val>                Platform-specific optimizations for installing dependencies. Possible values: "clonefile" (default), "hardlink", "symlink", "copyfile"
      --registry=<val>               Use a specific registry by default, overriding .npmrc, bunfig.toml and environment variables
      --concurrent-scripts=<val>     Maximum number of concurrent jobs for lifecycle scripts (default 5)
      --network-concurrency=<val>    Maximum number of concurrent network requests (default 48)
      --save-text-lockfile           Save a text-based lockfile
      --omit=<val>                   Exclude 'dev', 'optional', or 'peer' dependencies from install
      --lockfile-only                Generate a lockfile without installing dependencies
      --linker=<val>                 Linker strategy (one of "isolated" or "hoisted")
      --minimum-release-age=<val>    Only install packages published at least N seconds ago (security feature)
      --cpu=<val>                    Override CPU architecture for optional dependencies (e.g., x64, arm64, * for all)
      --os=<val>                     Override operating system for optional dependencies (e.g., linux, darwin, * for all)
  -h, --help                         Print this help menu
      --commit                       Install a package containing modifications in `dir`
      --patches-dir=<val>            The directory to put the patch file in (only if --commit is used)

Examples:
  Prepare jquery for patching
  bun patch jquery

  Generate a patch file for changes made to jquery
  bun patch --commit 'node_modules/jquery'

  Generate a patch file in a custom directory for changes made to jquery
  bun patch --patches-dir 'my-patches' 'node_modules/jquery'

Full documentation is available at https://bun.com/docs/install/patch.
```

## `vp pm patch-commit placeholder -- --help`

Bun receives patch commit through the --commit flag and prints its usage

```

Usage: bun patch [flags or options] <package>@<version>

  Prepare a package for patching, or generate and save a patch.

Flags:
  -c, --config=<val>                 Specify path to config file (bunfig.toml)
  -y, --yarn                         Write a yarn.lock file (yarn v1)
  -p, --production                   Don't install devDependencies
      --no-save                      Don't update package.json or save a lockfile
      --save                         Save to package.json (true by default)
      --ca=<val>                     Provide a Certificate Authority signing certificate
      --cafile=<val>                 The same as `--ca`, but is a file path to the certificate
      --dry-run                      Perform a dry run without making changes
      --frozen-lockfile              Disallow changes to lockfile
  -f, --force                        Always request the latest versions from the registry & reinstall all dependencies
      --cache-dir=<val>              Store & load cached data from a specific directory path
      --no-cache                     Ignore manifest cache entirely
      --silent                       Don't log anything
      --quiet                        Only show tarball name when packing
      --verbose                      Excessively verbose logging
      --no-progress                  Disable the progress bar
      --no-summary                   Don't print a summary
      --no-verify                    Skip verifying integrity of newly downloaded packages
      --ignore-scripts               Skip lifecycle scripts in the project's package.json (dependency scripts are never run)
      --trust                        Add to trustedDependencies in the project's package.json and install the package(s)
  -g, --global                       Install globally
      --cwd=<val>                    Set a specific cwd
      --backend=<val>                Platform-specific optimizations for installing dependencies. Possible values: "clonefile" (default), "hardlink", "symlink", "copyfile"
      --registry=<val>               Use a specific registry by default, overriding .npmrc, bunfig.toml and environment variables
      --concurrent-scripts=<val>     Maximum number of concurrent jobs for lifecycle scripts (default 5)
      --network-concurrency=<val>    Maximum number of concurrent network requests (default 48)
      --save-text-lockfile           Save a text-based lockfile
      --omit=<val>                   Exclude 'dev', 'optional', or 'peer' dependencies from install
      --lockfile-only                Generate a lockfile without installing dependencies
      --linker=<val>                 Linker strategy (one of "isolated" or "hoisted")
      --minimum-release-age=<val>    Only install packages published at least N seconds ago (security feature)
      --cpu=<val>                    Override CPU architecture for optional dependencies (e.g., x64, arm64, * for all)
      --os=<val>                     Override operating system for optional dependencies (e.g., linux, darwin, * for all)
  -h, --help                         Print this help menu
      --commit                       Install a package containing modifications in `dir`
      --patches-dir=<val>            The directory to put the patch file in (only if --commit is used)

Examples:
  Prepare jquery for patching
  bun patch jquery

  Generate a patch file for changes made to jquery
  bun patch --commit 'node_modules/jquery'

  Generate a patch file in a custom directory for changes made to jquery
  bun patch --patches-dir 'my-patches' 'node_modules/jquery'

Full documentation is available at https://bun.com/docs/install/patch.
```
