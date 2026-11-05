// Representative project loader: choose a TypeScript variant and record load order.
export async function resolve(specifier, context, nextResolve) {
  return nextResolve(
    specifier === '#variant' ? new URL('../src/variant.ts', import.meta.url).href : specifier,
    context,
  );
}

export async function load(url, context, nextLoad) {
  const result = await nextLoad(url, context);
  return url.endsWith('/order.js')
    ? { ...result, source: `${result.source}\norder.push('variant');` }
    : result;
}
