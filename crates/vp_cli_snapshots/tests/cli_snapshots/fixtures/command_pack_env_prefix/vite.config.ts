export default {
  pack: [
    { name: 'string', envPrefix: 'CUSTOM_' },
    { name: 'array', envPrefix: ['CUSTOM_', 'OTHER_'] },
    { name: 'empty', envPrefix: [] },
    { name: 'defaults' },
  ].map(({ name, ...config }) => ({
    ...config,
    entry: ['entry.js'],
    envFile: 'build.env',
    outDir: `dist/${name}`,
  })),
};
