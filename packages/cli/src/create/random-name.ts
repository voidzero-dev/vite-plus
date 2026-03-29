import { getRandomWord } from '@nkzw/safe-word-list';

const isTest = process.env.VP_CLI_TEST === '1';

export default function getRandomWords(): ReadonlyArray<string> {
  const first = getRandomWord();
  let second: string;
  do {
    second = getRandomWord();
  } while (second === first);
  return [first, second];
}

export function getRandomProjectName(
  options: {
    scope?: string;
    fallbackName?: string;
  } = {},
): string {
  const { scope, fallbackName } = options;
  const projectName = isTest && fallbackName ? fallbackName : getRandomWords().join('-');
  return scope ? `${scope}/${projectName}` : projectName;
}
