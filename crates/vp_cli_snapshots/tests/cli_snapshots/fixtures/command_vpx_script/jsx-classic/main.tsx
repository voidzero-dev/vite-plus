function h(tag: string, props: Record<string, unknown> | null, ...children: unknown[]): string {
  const id = props?.id ? ` id="${String(props.id)}"` : '';
  return `<${tag}${id}>${children.join('')}</${tag}>`;
}
const Fragment = 'fragment';

console.log('classic:', <div id="greeting">{'hello'}</div>, <>{'fragment child'}</>);
