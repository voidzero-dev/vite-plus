export default function myLazyPlugin() {
  return {
    name: 'my-lazy-plugin',
    transformIndexHtml(html: string) {
      return html.replace('</body>', '<!-- lazy-plugin-injected --></body>');
    },
  };
}
