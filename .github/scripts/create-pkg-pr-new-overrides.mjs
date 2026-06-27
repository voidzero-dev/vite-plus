const [vite, vitest] = process.argv.slice(2);

if (!vite || !vitest) {
  console.error('Usage: create-pkg-pr-new-overrides.mjs <vite-spec> <vitest-version>');
  process.exit(2);
}

process.stdout.write(JSON.stringify({ vite, vitest }));
