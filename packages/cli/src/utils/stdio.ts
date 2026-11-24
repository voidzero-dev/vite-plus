interface BlockingHandle {
  setBlocking(blocking: boolean): void;
}

function isBlockingHandle(value: unknown): value is BlockingHandle {
  return (
    value !== null &&
    typeof value === 'object' &&
    'setBlocking' in value &&
    typeof value.setBlocking === 'function'
  );
}

export function setBlocking(
  stream: NodeJS.ReadStream | NodeJS.WriteStream,
  blocking: boolean,
): void {
  // oxlint-disable-next-line no-underscore-dangle -- Node exposes blocking mode only through the stream handle.
  const handle = '_handle' in stream ? stream._handle : undefined;
  if (isBlockingHandle(handle)) {
    handle.setBlocking(blocking);
  }
}
