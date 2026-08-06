export default function mySyncPlugin() {
  return {
    name: 'my-sync-plugin',
    transformIndexHtml(html: string) {
      return html.replace('</body>', '<!-- sync-plugin-injected --></body>');
    },
  };
}
