console.log('args', JSON.stringify(process.argv.slice(2)));
console.log('greeting', process.env.GREETING ?? '(unset)');
console.log('setup ran', (globalThis as { setupRan?: boolean }).setupRan === true);
process.exitCode = 7;
