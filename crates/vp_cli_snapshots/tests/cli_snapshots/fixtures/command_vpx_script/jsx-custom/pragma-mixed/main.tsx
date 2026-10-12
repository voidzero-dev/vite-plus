/** @jsx jsx */
/** @jsxImportSource mini-jsx */
// The pragma pair Hono documents. The import source pragma selects the automatic
// runtime, so the factory pragma is ignored and no `jsx` import is needed.
const Card = (props: { title: string; children?: unknown }) => <section title={props.title}>{props.children}</section>;
console.log(<Card title="t"><b>bold</b><>frag</></Card>);
