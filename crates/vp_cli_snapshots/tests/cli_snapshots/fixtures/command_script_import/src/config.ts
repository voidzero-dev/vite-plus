import { value } from '@value';
console.log(JSON.stringify({
  value,
  explicit: process.env.VP_SCRIPT_TSCONFIG ?? null,
  compat: process.env.TSX_TSCONFIG_PATH ?? null,
  upstream: process.env.OXC_TSCONFIG_PATH ?? null,
  tsnode: process.env.TS_NODE_PROJECT ?? null,
}));
