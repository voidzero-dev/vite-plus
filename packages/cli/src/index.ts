import yargs from "yargs/yargs";
import { hideBin } from "yargs/helpers";
import { spawnSync } from "node:child_process";
import path from "node:path";

function execPackageBin(binName: string, args: string[]) {
  // TODO: exec from bundled packages
  const program = path.join(
    import.meta.dirname,
    "../node_modules/.bin",
    binName,
  );
  exec(program, args);
}

function exec(program: string, args: string[]) {
  const { status, error } = spawnSync(program, args, { stdio: "inherit" });
  if (error !== undefined) {
    throw error;
  }
  process.exit(status ?? 255);
}

const args = hideBin(process.argv);
const commandArgs = args.slice(1);

const cli = yargs(args).scriptName("vite");

for (const viteCommand of ["build", "optimize", "preview", "dev"]) {
  // register vite command one by one instead of cli.command(['build', 'optimize', 'preview', 'dev'], ..)
  // so that the help message won't list them as aliases (vite build [aliases: optimize, preview, dev])
  cli.command(viteCommand, "", () => {
    execPackageBin("vite", args);
  });
}
cli.command("lib", "", () => {
  execPackageBin("tsdown", commandArgs);
});

cli.command("run", "", () => {
  exec(process.execPath, [
    "--import",
    import.meta.resolve("@oxc-node/core/register"),
    ...commandArgs,
  ]);
});

cli.command("lint", "", () => {
  execPackageBin("oxlint", commandArgs);
});

cli.command("test", "", () => {
  execPackageBin("vitest", commandArgs);
});
cli.command("bench", "", () => {
  execPackageBin("vitest", ["bench", ...commandArgs]);
});

cli.command("docs", "", () => {
  execPackageBin("vitepress", commandArgs);
});

// cli.command('fmt', '')
// cli.command('publish', '')
// cli.command('ui', '')

cli.help().parse();
