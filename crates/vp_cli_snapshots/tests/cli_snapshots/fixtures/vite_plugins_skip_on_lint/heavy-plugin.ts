throw new Error('Plugins should not be loaded during lint');

export default function heavyPlugin() {
  return { name: 'heavy-plugin' };
}
