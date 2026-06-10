import { createTemplate } from 'bingo';
import { z } from 'zod';

import pkgJson from '../package.json' with { type: 'json' };

export default createTemplate({
  about: {
    name: pkgJson.name,
    description: pkgJson.description,
  },

  // Define your options using Zod schemas
  options: {
    name: z.string().describe('Package name'),
    // TODO: Add more options as needed
  },

  // Generate files based on options
  async produce({ options }) {
    return {
      // see https://www.create.bingo/build/concepts/creations#files
      files: {
        'package.json': JSON.stringify(
          {
            name: options.name,
            version: '0.0.0',
            type: 'module',
            // TODO: Add more package.json fields
          },
          null,
          2,
        ),
        src: {
          'index.ts': `export const name = '${options.name}';
`,
        },
        'tsconfig.json': JSON.stringify(
          {
            compilerOptions: {
              declaration: true,
              esModuleInterop: true,
              module: 'NodeNext',
              moduleResolution: 'NodeNext',
              outDir: 'lib',
              skipLibCheck: true,
              strict: true,
              target: 'ES2022',
            },
            include: ['src'],
          },
          null,
          2,
        ),
        // TODO: Add more files
      },
      // see https://www.create.bingo/build/concepts/creations#scripts
      scripts: [
        // Optional: Add scripts to run after generation
      ],
      // see https://www.create.bingo/build/concepts/creations#suggestions
      suggestions: [
        // Optional: Add suggestions for users
      ],
    };
  },
});
