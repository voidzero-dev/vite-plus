import packageJson from './package.json' assert { type: 'json' };

console.log(packageJson.devEngines.packageManager.version);
