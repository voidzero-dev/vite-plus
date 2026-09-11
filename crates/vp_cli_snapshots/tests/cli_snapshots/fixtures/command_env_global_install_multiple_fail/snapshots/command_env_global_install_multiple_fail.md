# command_env_global_install_multiple_fail

## `vp install -g . voidzero-nonexistent-pkg-xyz-23456`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

info: Installing 2 global packages with Node.js <version>
⠿ Installing global packages (<n>/2)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  npm error code E404
npm error 404 Not Found - GET http://127.0.0.1:<port>/voidzero-nonexistent-pkg-xyz-23456 - <message>
npm error 404
npm error 404  The requested resource 'voidzero-nonexistent-pkg-xyz-23456@*' could not be found or you do not have permission to access it.
npm error 404
npm error 404 Note that you can also install from a
npm error 404 tarball, folder, http url, or git url.
npm error A complete log of this run can be found in: <home>/.npm/_logs/<timestamp>-debug-0.log
✓ Installed install-fail-local-package 0.0.0
  Bins: install-fail-local-package

error: Failed to install voidzero-nonexistent-pkg-xyz-23456: npm install failed with exit status: 1
```

## `install-fail-local-package`

```
The package is installed successfully
```
