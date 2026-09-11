throw new Error('Plugins should not be loaded during vp run lint');

export default function heavyPlugin() {
  return { name: 'heavy-plugin' };
}
