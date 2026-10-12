console.log('format', typeof require === 'function' ? 'CommonJS' : 'ESM');
const Card = (props: { title: string; children?: unknown }) => <section title={props.title}>{props.children}</section>;
console.log(<Card title="t"><b>bold</b><>frag</></Card>);
