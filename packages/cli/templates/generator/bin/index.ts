#!/usr/bin/env node

import fs from 'node:fs';
import { parseArgs } from 'node:util';

import { runTemplate, runTemplateCLI } from 'bingo';
import { z } from 'zod';

import template from '../src/template.ts';

async function main() {
  if (
    process.env.VP_CREATE_INTERACTIVE !== '0' ||
    process.argv.includes('--help') ||
    process.argv.includes('--version')
  ) {
    // @ts-expect-error Bingo's CLI accepts this template but its types require a generic options schema.
    return await runTemplateCLI(template);
  }

  // Add CLI entries here when adding options to src/template.ts.
  const { values } = parseArgs({
    options: {
      directory: { type: 'string' },
      name: { type: 'string' },
      offline: { type: 'boolean' },
      'skip-requests': { type: 'boolean' },
      'skip-files': { type: 'boolean' },
      'skip-scripts': { type: 'boolean' },
    },
  });
  if (!values.directory?.trim()) {
    throw new Error('Missing --directory. Pass generator options after -- in vp create.');
  }
  const options = z.object(template.options).parse(values);
  if (fs.existsSync(values.directory)) {
    throw new Error(`Directory already exists: ${values.directory}`);
  }

  await runTemplate(template, {
    directory: values.directory,
    mode: 'setup',
    options,
    offline: values.offline,
    skips: {
      requests: values['skip-requests'],
      files: values['skip-files'],
      scripts: values['skip-scripts'],
    },
  });
  return 0;
}

try {
  process.exitCode = await main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`${message}\n`);
  process.exitCode = 1;
}
