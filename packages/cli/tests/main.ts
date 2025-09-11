import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto'
import fs from 'node:fs'
import cp from 'node:child_process';
import path from 'node:path'

// Create a unique temporary directory for testing
const tempTmpDir = `${tmpdir()}/vite-plus-test-${randomUUID()}`;
fs.mkdirSync(tempTmpDir, { recursive: true });

// Clean up the temporary directory on exit
process.on('exit', () => fs.rmSync(tempTmpDir, { recursive: true, force: true }));

const casesDir = import.meta.dirname + '/cases';

for (const caseName of fs.readdirSync(casesDir)) {
    if (caseName.startsWith('.')) continue; // Skip hidden files like .DS_Store
    runTestCase(caseName);
}

function runTestCase(name: string) {
    const caseTmpDir = `${tempTmpDir}/${name}`;
    fs.cpSync(`${casesDir}/${name}`, caseTmpDir, { recursive: true, errorOnExist: true });

    // Read the snap.txt file to get the commands to execute
    const snap = fs.readFileSync(`${caseTmpDir}/snap.txt`, 'utf-8');
    const commands = snap.split('\n').filter(line => line.startsWith('> ')).map(line => line.slice(2).trim());

    const env = {
        ...process.env,
        // Indicate CLI is running in test mode
        VITE_PLUS_CLI_TEST: '1'
    };
    env['PATH'] = [
        ...env['PATH']!.split(path.delimiter),
         // Extend PATH to include the CLI's bin directory
        path.dirname(import.meta.dirname) + '/bin',
        // Also include node_modules/.bin for local dev dependencies
        path.dirname(import.meta.dirname) + '/node_modules/.bin',
    ].join(path.delimiter);

    const newSnap: string[] = [];

    for (const command of commands) {
        newSnap.push(`> ${command}`);
        const output = cp.execSync(command, { env, cwd: caseTmpDir });
        newSnap.push(output.toString('utf8'));
    }
    const newSnapContent = newSnap.join('\n');

    fs.writeFileSync(`${casesDir}/${name}/snap.txt`, newSnapContent);
}
