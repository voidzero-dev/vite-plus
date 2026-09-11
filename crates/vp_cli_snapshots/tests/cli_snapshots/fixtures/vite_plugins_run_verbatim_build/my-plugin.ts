export default function myPlugin() {
  return {
    name: 'my-run-verbatim-plugin',
    transformIndexHtml(html: string) {
      return html.replace('</body>', '<!-- run-verbatim-plugin-injected --></body>');
    },
  };
}
