function cwdJsx(tag: string, props: { value: string }) {
  return { config: 'cwd', tag, value: props.value };
}

function entryJsx(tag: string, props: { value: string }) {
  return { config: 'entry', tag, value: props.value };
}

function projectJsx(tag: string, props: { value: string }) {
  return { config: 'project', tag, value: props.value };
}

// A type check must reject this assignment; transpile-only execution preserves it.
const value: number = 'typescript';
console.log(JSON.stringify(<result value={value} />));
