// Rolldown 1.3 gives each preload marker a unique hash so minification cannot
// merge calls with different dependencies. Vite 8.3.6 only consumes the prefix.
export function patchViteImportAnalysis(source: string): string {
  const pattern = "const preloadMarkerRE = new RegExp(preloadMarker, 'g')";
  const markerEnd = 'markerStartPos + preloadMarker.length';
  if (source.split(pattern).length !== 2 || source.split(markerEnd).length !== 5) {
    throw new Error('Vite preload marker anchors changed; update patch-vite-import-analysis.ts');
  }

  return source
    .replace(
      pattern,
      [
        "const preloadMarkerRE = new RegExp(`${preloadMarker}(?:[0-9a-f]{32})?`, 'g')",
        '',
        'function getPreloadMarkerEnd(code: string, start: number): number {',
        '  preloadMarkerRE.lastIndex = start',
        '  const match = preloadMarkerRE.exec(code)',
        '  if (match?.index !== start) throw new Error("Missing Vite preload marker")',
        '  return start + match[0].length',
        '}',
      ].join('\n'),
    )
    .replaceAll(markerEnd, 'getPreloadMarkerEnd(code, markerStartPos)');
}
