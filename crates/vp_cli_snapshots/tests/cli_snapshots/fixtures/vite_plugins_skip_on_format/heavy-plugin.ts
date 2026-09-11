throw new Error('Plugins should not be loaded during vp format');

export default function heavyPlugin() {
  return { name: 'heavy-plugin' };
}
