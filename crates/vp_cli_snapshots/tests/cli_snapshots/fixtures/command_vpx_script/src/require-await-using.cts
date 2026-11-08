try {
  require('./await-using.mts');
} catch (error) {
  console.log('require() of await using:', (error as NodeJS.ErrnoException).code);
}
