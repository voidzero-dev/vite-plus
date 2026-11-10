// The program a cmd-shim style .cmd forwards its arguments to.
console.log('fake-tool:', JSON.stringify(process.argv.slice(2)));
process.exitCode = 3;
