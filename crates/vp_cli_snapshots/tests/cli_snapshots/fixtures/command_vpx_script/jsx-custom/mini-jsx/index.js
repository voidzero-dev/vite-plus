// A tiny JSX runtime that renders to strings: classic `h`, automatic `jsx`/`jsxs`, and
// development `jsxDEV`. The snapshot case copies it into jsx-custom/node_modules.
function render(tag, props, children) {
  const list = children.flat().map((child) => (typeof child === 'string' ? child : String(child)));
  if (tag === Fragment) return list.join('');
  if (typeof tag === 'function') return tag({ ...props, children: list });
  const attrs = Object.entries(props ?? {})
    .filter(([key]) => key !== 'children' && key !== '__self' && key !== '__source')
    .map(([key, value]) => ` ${key}="${value}"`)
    .join('');
  return `<${tag}${attrs}>${list.join('')}</${tag}>`;
}
export const Fragment = Symbol.for('mini.fragment');
export function h(tag, props, ...children) { return render(tag, props, children); }
export function jsx(tag, props) {
  const { children = [] } = props;
  return render(tag, props, Array.isArray(children) ? children : [children]);
}
export const jsxs = jsx;
export function jsxDEV(tag, props, key, isStatic, source) {
  return `${jsx(tag, props)} @${source ? source.lineNumber : '?'}`;
}
