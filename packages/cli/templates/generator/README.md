# Vite+ Code Generator Starter

A starter for creating a Vite+ code generator.

## Usage

From monorepo root:

```bash
# run and select the generator
vp create
```

For automation, provide the directory and every required template option:

```bash
vp create <generator-name> --no-interactive -- --directory new-package --name new-package
```

Vite+ sets `VP_CREATE_INTERACTIVE=0` for non-interactive local Bingo generators.
This starter then validates the arguments and runs Bingo's programmatic API.
Missing options and existing directories fail before any files are generated.
Existing generators are copied project files and are not updated by upgrading Vite+.
To adopt this behavior, update their entrypoint to match this starter.
Direct invocation uses the interactive CLI unless this variable is set to `0`.
When adding template options, also add their CLI types in `bin/index.ts`.

## Development

```bash
# Edit the template
code src/template.ts

# Test the generator CLI
vp run dev

# Run tests
vp run test
```

## Customization

Edit `src/template.ts` to customize:

- Options schema (using Zod)
- File generation logic
- Scripts and suggestions

More information about the [Bingo Templates](https://create.bingo/) can be found [here](https://create.bingo/build/concepts/creations).
