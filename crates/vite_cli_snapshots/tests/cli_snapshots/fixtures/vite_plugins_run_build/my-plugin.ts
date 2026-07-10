export default function myPlugin() {
  return {
    name: 'my-run-build-plugin',
    transformIndexHtml(html: string) {
      return html.replace('</body>', '<!-- run-build-plugin-injected --></body>');
    },
  };
}
