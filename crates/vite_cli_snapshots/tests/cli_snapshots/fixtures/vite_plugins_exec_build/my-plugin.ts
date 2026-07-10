export default function myPlugin() {
  return {
    name: 'my-exec-build-plugin',
    transformIndexHtml(html: string) {
      return html.replace('</body>', '<!-- exec-build-plugin-injected --></body>');
    },
  };
}
