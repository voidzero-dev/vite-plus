// Surfaces the package-manager lifecycle env that `vp run` stamps for
// package.json scripts (#2317): before the fix every variable below printed
// `(undefined)`, so child tooling (npm-run-all, `ni`) could not detect pnpm
// and fell back to npm. The user-agent platform/arch tail (`linux x64`) is
// the one machine-dependent value the suite redaction does not mask, so it
// is normalized here from the runtime's own platform/arch.
const vars = ['npm_execpath', 'npm_config_user_agent', 'INIT_CWD'];
for (const name of vars) {
  let value = process.env[name] ?? '(undefined)';
  if (name === 'npm_config_user_agent') {
    value = value.replace(`${process.platform} ${process.arch}`, '<platform> <arch>');
  }
  console.log(`${name}=${value}`);
}
