// Simulate `lightningcss` not being installed: the dev/CI monorepo always has it
// resolvable from core (it is a transitive dependency), so a resolve hook is the
// deterministic way to exercise the missing optional-peer path. See issue #1586.
export async function resolve(specifier, context, nextResolve) {
  if (specifier === 'lightningcss') {
    const error = new Error("Cannot find package 'lightningcss'");
    error.code = 'ERR_MODULE_NOT_FOUND';
    throw error;
  }
  return nextResolve(specifier, context);
}
