import type { RewriteResult, VitestV5Finding } from './ast.ts';

interface CommandToken {
  value: string;
  start: number;
  end: number;
}

/** Parse only a single literal argv command. Shell expansion, pipelines, and
 * compound commands need review; treating them as argv could change behavior. */
export function literalArgv(command: string): CommandToken[] | undefined {
  const tokens: CommandToken[] = [];
  let value = '';
  let start = 0;
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
      if (!active) {
        start = i;
      }
      quote = character;
      active = true;
    } else if (character === '\n' || character === '\r') {
      return undefined;
    } else if (/\s/.test(character)) {
      if (active) {
        tokens.push({ value, start, end: i });
      }
      active = false;
      value = '';
    } else if (/[|&;<>($`\\)]/.test(character)) {
      return undefined;
    } else {
      if (!active) {
        start = i;
      }
      value += character;
      active = true;
    }
  }
  if (quote) {
    return undefined;
  }
  if (active) {
    tokens.push({ value, start, end: command.length });
  }
  return tokens;
}

/** Read command lists without executing a shell. Directory-changing builtins,
 * expansions, pipelines, and shell programs remain unresolved. */
export function literalTestCommands(command: string): string[][] | undefined {
  const commands: string[][] = [];
  let quote = '';
  let start = 0;
  const append = (end: number) => {
    const tokens = literalArgv(command.slice(start, end));
    if (!tokens?.length) {
      return false;
    }
    const values = tokens.map(({ value }) => value);
    while (/^[A-Za-z_]\w*=/.test(values[0] ?? '')) {
      values.shift();
    }
    if (
      !values.length ||
      ['cd', 'pushd', 'popd', '.', 'source', 'eval', 'export', 'set', 'unset'].includes(values[0])
    ) {
      return false;
    }
    commands.push(values);
    return true;
  };
  for (let index = 0; index < command.length; index++) {
    const char = command[index];
    if (char === '\\') {
      return undefined;
    }
    if (quote) {
      if (char === quote) {
        quote = '';
      }
    } else if (char === '"' || char === "'") {
      quote = char;
    } else if (char === '#') {
      return undefined;
    } else if (
      char === ';' ||
      command.slice(index, index + 2) === '&&' ||
      command.slice(index, index + 2) === '||'
    ) {
      if (!append(index)) {
        return undefined;
      }
      if (char !== ';') {
        index++;
      }
      start = index + 1;
    }
  }
  return !quote && append(command.length) ? commands : undefined;
}

/** Locate the runner's arguments after a supported package-manager wrapper. */
export function vitestCommandArgsStart(values: readonly string[]): number {
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
  if (values[start] === 'vitest') {
    return start + 1;
  }
  if (values[start] === 'vp' && values[start + 1] === 'test') {
    return start + 2;
  }
  return -1;
}

function migrateBenchmarkOutput(command: string): string {
  const argv = literalArgv(command);
  if (!argv) {
    return command;
  }
  const values = argv.map(({ value }) => value);
  const start = vitestCommandArgsStart(values);
  if (start < 0) {
    return command;
  }
  const end = values.indexOf('--', start);
  const args = argv.slice(start, end < 0 ? undefined : end);
  // A quoted option-looking token can be another flag's value. Restrict this
  // rewrite to known argv shapes instead of guessing how an unknown flag parses.
  for (let index = 0; index < args.length; index++) {
    const { value } = args[index];
    if (/^--(?:outputJson|outputFile|reporters?)$/.test(value)) {
      const next = args[++index]?.value;
      if (!next || next.startsWith('-')) {
        return command;
      }
    } else if (
      value.startsWith('-') &&
      !/^--(?:run|watch)$/.test(value) &&
      !/^--(?:outputJson|outputFile|reporters?)=/.test(value)
    ) {
      return command;
    }
  }
  const legacy = args.filter(({ value }) => /^--outputJson(?:=|$)/.test(value));
  if (legacy.length !== 1) {
    return command;
  }
  const flag = legacy[0];
  const value = flag.value.startsWith('--outputJson=')
    ? flag.value.slice('--outputJson='.length)
    : args[args.indexOf(flag) + 1]?.value;
  if (!value || value.startsWith('-')) {
    return command;
  }
  const last = flag.value.includes('=') ? flag : args[args.indexOf(flag) + 1];
  const raw =
    flag === last
      ? command.slice(flag.start + '--outputJson='.length, flag.end)
      : command.slice(last.start, last.end);
  // Moving an unquoted glob behind '=' can change shell expansion.
  if (!/^[\w./:@+-]+$/.test(raw) && !/^(['"]).*\1$/.test(raw)) {
    return command;
  }
  const output = args.filter(({ value }) => /^--outputFile(?:[.=]|$)/.test(value));
  if (output.length > 1) {
    return command;
  }
  if (output.length) {
    const token = output[0];
    if (!/^--outputFile(?:=|$)/.test(token.value)) {
      return command;
    }
    const destination = token.value.includes('=')
      ? token.value.slice('--outputFile='.length)
      : args[args.indexOf(token) + 1]?.value;
    if (destination !== value) {
      return command;
    }
  }
  const reporters: string[] = [];
  for (const [index, { value }] of args.entries()) {
    if (/^--reporters?=/.test(value)) {
      reporters.push(value.slice(value.indexOf('=') + 1));
    } else if (/^--reporters?$/.test(value)) {
      reporters.push(args[index + 1]?.value ?? '');
    }
  }
  if (reporters.some((name) => !['default', 'verbose', 'json'].includes(name))) {
    return command;
  }
  const replacement = [
    ...(!reporters.length ? ['--reporter=default'] : []),
    ...(!reporters.includes('json') ? ['--reporter=json'] : []),
    ...(!output.length ? [`--outputFile=${raw}`] : []),
  ].join(' ');
  return command.slice(0, flag.start) + replacement + command.slice(last.end);
}

export function migrateVitestV5Command(
  file: string,
  command: string,
  preserveV4: boolean,
  line = 1,
  reviewV4 = preserveV4,
): RewriteResult {
  const findings: VitestV5Finding[] = [];
  const report = (
    code: string,
    message: string,
    severity: VitestV5Finding['severity'] = 'review',
  ) => findings.push({ file, line, column: 1, code, message, severity });
  if (!/\bvitest\b|\bvp\s+test\b/.test(command)) {
    return { content: command, findings };
  }
  command = migrateBenchmarkOutput(command);
  // The bench subcommand still exists in v5. Only its removed flags block
  // migration; benchmark source changes are handled by the source pass.
  const argv = literalArgv(command);
  const values = argv?.map(({ value }) => value);
  const runner = values ? vitestCommandArgsStart(values) : -1;
  let removedFlag = /(?:^|\s)(?:--compare|--outputJson)(?:\s|=|$)/.test(command);
  let reviewNamePattern = /(?:^|\s)(?:-t|--testNamePattern|--test-name-pattern)(?:\s|=)/.test(
    command,
  );
  if (values && runner >= 0) {
    const terminator = values.indexOf('--', runner);
    const args = values.slice(runner, terminator === -1 ? undefined : terminator);
    removedFlag = args.some((value) => /^(?:--compare|--outputJson)(?:=|$)/.test(value));
    reviewNamePattern = args.some((value, index) => {
      if (!/^(?:-t|--testNamePattern|--test-name-pattern)(?:=|$)/.test(value)) {
        return false;
      }
      const pattern = value.includes('=') ? value.slice(value.indexOf('=') + 1) : args[index + 1];
      // A plain single-segment substring is unaffected by suite separators.
      return !pattern || !/^[\w-]+$/.test(pattern);
    });
  }
  if (reviewV4 && reviewNamePattern) {
    report(
      'test-name-pattern',
      'Review test-name patterns across suite boundaries; full names now use > separators.',
    );
  }
  if (removedFlag) {
    report(
      'benchmark-api',
      'Replace removed --compare/--outputJson flags with bench context comparisons or regular JSON reporter output. The bench subcommand is still supported.',
      'block',
    );
  }
  if (!reviewV4 || !/\blist\b/.test(command)) {
    return { content: command, findings };
  }
  if (!argv || !values) {
    report(
      'static-list',
      'Review this wrapped or compound list command and explicitly choose --no-static-parse to retain runtime collection.',
    );
    return { content: command, findings };
  }
  const list = runner;
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
