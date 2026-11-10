const [packageManager, version] = process.env.npm_config_user_agent.split(' ')[0].split('/');

require('node:fs').writeFileSync(
  'installed-with.json',
  JSON.stringify({ packageManager, version }, null, 2) + '\n',
);
