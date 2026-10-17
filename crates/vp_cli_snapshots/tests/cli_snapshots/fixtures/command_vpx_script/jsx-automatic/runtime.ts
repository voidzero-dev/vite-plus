export function jsx(tag: string, props: { children?: unknown }): string {
  return `<${tag}>${String(props.children ?? '')}</${tag}>`;
}
export const jsxs = jsx;
export const Fragment = 'fragment';
