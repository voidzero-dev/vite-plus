try {
  await import(process.argv[2]);
} catch (error) {
  console.log(error.message);
  // Keep the source location visible: the runner removes ordinary file:// stack frames.
  console.log('source:', error.stack.split('\n')[1].trim().replace(/^at /, ''));
  process.exitCode = 1;
}
