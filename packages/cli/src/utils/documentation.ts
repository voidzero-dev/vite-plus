import { documentationUrl as nativeDocumentationUrl } from '../../binding/index.js';

export function documentationUrl(path: `/${string}`): string {
  return nativeDocumentationUrl(path);
}

/** Keep generated agent guidance on the same documentation version as the CLI. */
export function rewriteDocumentationLinks(content: string): string {
  return content.replaceAll('https://viteplus.dev/guide/', documentationUrl('/guide/'));
}
