// The in-thread hooks report the same error for a `.cts` with ES module syntax.
try {
  await import('./both.cts');
} catch (error) {
  console.log((error as Error).message);
}
