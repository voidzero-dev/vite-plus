const [, , ...rest] = process.argv;
console.log('node version:', process.version);
console.log('script args:', JSON.stringify(rest));
