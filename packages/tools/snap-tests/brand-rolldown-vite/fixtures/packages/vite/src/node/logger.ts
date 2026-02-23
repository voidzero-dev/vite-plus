export function createLogger(level = 'info', prefix = '[vite]') {
  return { level, prefix };
}
