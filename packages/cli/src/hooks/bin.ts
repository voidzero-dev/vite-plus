import { parseHooksArgs } from '../../binding/index.js';
import { disable, enable, status } from '../config/hooks.ts';
import { unwrapCliParseOutcome } from '../utils/cli-parse.ts';
import { log } from '../utils/terminal.ts';

function applyResult(result: { message: string; isError: boolean }): void {
  if (result.message) {
    log(result.message);
  }
  if (result.isError) {
    process.exit(1);
  }
}

async function main() {
  const args = unwrapCliParseOutcome(parseHooksArgs(process.argv.slice(3)));

  switch (args.command) {
    case 'enable':
      applyResult(enable(args.hooksDir));
      return;
    case 'disable':
      applyResult(disable(args.hooksDir));
      return;
    case 'status':
      applyResult(status(args.hooksDir));
      return;
  }
}

void main();
