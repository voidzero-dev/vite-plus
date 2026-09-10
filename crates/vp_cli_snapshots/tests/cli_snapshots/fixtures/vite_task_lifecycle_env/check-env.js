// Surfaces the package-manager lifecycle env that `vp run` stamps for
// package.json scripts (#2317): before the fix every variable below printed
// `(undefined)`, so child tooling (npm-run-all, `ni`) could not detect pnpm
// and fell back to npm. The stamp is deliberately limited to the two
// package-manager detection channels; Bun remains outside that narrow
// compatibility contract. The user-agent platform/arch tail (`linux x64`)
// and pnpm native executable suffix are normalized across platforms.
const vars = ['npm_execpath', 'npm_config_user_agent'];
for (const name of vars) {
  let value = process.env[name] ?? '(undefined)';
  if (name === 'npm_config_user_agent') {
    value = value.replace(`${process.platform} ${process.arch}`, '<platform> <arch>');
  } else {
    value = value.replace(/pnpm\.native\.exe$/, 'pnpm.native');
  }
  console.log(`${name}=${value}`);
}
