try {
  require('./tla.mts');
} catch (error) {
  console.log('require() of top-level await:', (error as NodeJS.ErrnoException).code);
}
