import type { VitestV5Finding } from './ast.ts';

/** Parse only a single literal argv command. Shell expansion, pipelines, and
 * compound commands need review; treating them as argv could change behavior. */
function literalArgv(command: string): Array<{ value: string; end: number }> | undefined {
  const tokens: Array<{ value: string; end: number }> = [];
  let value = '';
  let quote = '';
  let active = false;
  for (let i = 0; i < command.length; i++) {
    const character = command[i];
    if (quote) {
      if (character === quote) {
        quote = '';
      } else if (
        character === '\\' ||
        (quote === '"' && (character === '$' || character === '`'))
      ) {
        return undefined;
      } else {
        value += character;
      }
    } else if (character === "'" || character === '"') {
      quote = character;
      active = true;
    } else if (character === '\n' || character === '\r') {
      return undefined;
    } else if (/\s/.test(character)) {
      if (active) {
        tokens.push({ value, end: i });
      }
      active = false;
      value = '';
    } else if (/[|&;<>($`\\)]/.test(character)) {
      return undefined;
    } else {
      value += character;
      active = true;
    }
  }
  if (quote) {
    return undefined;
  }
  if (active) {
    tokens.push({ value, end: command.length });
  }
  return tokens;
}

export function migrateVitestV5Command(
  file: string,
  command: string,
  preserveV4: boolean,
  line = 1,
) {
  const findings: VitestV5Finding[] = [];
  const report = (
    code: string,
    message: string,
    severity: VitestV5Finding['severity'] = 'review',
  ) => findings.push({ file, line, column: 1, code, message, severity });
  if (!/\bvitest\b|\bvp\s+test\b/.test(command)) {
    return { content: command, findings };
  }
  if (/--reporter[= ](?:json|junit)/.test(command) && !/--outputFile(?:=|\s)/.test(command)) {
    report(
      'reporter-stdout',
      'JSON/JUnit now write to .vitest/json/output.json or .vitest/junit/output.xml. Configure reporter stdout: true if this command needs the old stdout output.',
    );
  }
  if (/(?:^|\s)(?:-t|--testNamePattern)(?:\s|=)/.test(command)) {
    report(
      'test-name-pattern',
      'Review test-name patterns across suite boundaries; full names now use > separators.',
    );
  }
  if (/(?:^|\s)(?:bench|--compare|--outputJson)(?:\s|=|$)/.test(command)) {
    report(
      'benchmark-api',
      'Replace benchmark commands and removed --compare/--outputJson flags with tests using the bench context fixture.',
      'block',
    );
  }
  if (
    /\.vitest-attachements|\.vitest-reports|__screenshots__|html\/index\.html|--reporter[= ](?:json|junit).*\|/.test(
      command,
    )
  ) {
    report(
      'artifact-paths',
      'Review artifact paths and JSON/JUnit stdout consumers; output defaults moved under .vitest.',
    );
  }
  if (!/\blist\b/.test(command)) {
    return { content: command, findings };
  }
  const argv = literalArgv(command);
  if (!argv) {
    report(
      'static-list',
      'Review this wrapped or compound list command and explicitly choose --no-static-parse to retain runtime collection.',
    );
    return { content: command, findings };
  }
  const values = argv.map((token) => token.value);
  let start = 0;
  if (['pnpm', 'npm', 'yarn', 'bun', 'npx', 'bunx'].includes(values[0])) {
    start = 1;
    if (values[start] === 'exec' || values[start] === 'x') {
      start++;
    }
    if (values[start] === '--') {
      start++;
    }
  }
  const list =
    values[start] === 'vitest'
      ? start + 1
      : values[start] === 'vp' && values[start + 1] === 'test'
        ? start + 2
        : -1;
  if (list < 0 || values[list] !== 'list') {
    report(
      'static-list',
      'Resolve this list command wrapper and explicitly choose a static-parse setting.',
    );
    return { content: command, findings };
  }
  const optionsEnd = values.indexOf('--', list + 1);
  const args = values.slice(list + 1, optionsEnd < 0 ? undefined : optionsEnd);
  if (!preserveV4 || args.some((arg) => /^--(?:no-)?static-parse(?:=|$)/.test(arg))) {
    return { content: command, findings };
  }
  const offset = argv[list].end;
  return {
    content: `${command.slice(0, offset)} --no-static-parse${command.slice(offset)}`,
    findings,
  };
}
