import { h, Fragment } from 'mini-jsx';
const Card = (props) => <section title={props.title}>{props.children}</section>;
console.log(<Card title="t"><b>bold</b><>frag</></Card>);
