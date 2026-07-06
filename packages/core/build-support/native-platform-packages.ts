const archMap: Record<string, string> = {
  aarch64: 'arm64',
  x86_64: 'x64',
};

export function getNativePlatformPackageName(target: string): string {
  const [archTarget, ...platformParts] = target.split('-');
  const arch = archMap[archTarget];
  const platform = platformParts.join('-');

  if (!arch) {
    throw new Error(`Unsupported NAPI target architecture: ${target}`);
  }

  if (platform === 'apple-darwin') {
    return `@voidzero-dev/vite-plus-darwin-${arch}`;
  }
  if (platform === 'unknown-linux-gnu') {
    return `@voidzero-dev/vite-plus-linux-${arch}-gnu`;
  }
  if (platform === 'unknown-linux-musl') {
    return `@voidzero-dev/vite-plus-linux-${arch}-musl`;
  }
  if (platform === 'pc-windows-msvc') {
    return `@voidzero-dev/vite-plus-win32-${arch}-msvc`;
  }

  throw new Error(`Unsupported NAPI target platform: ${target}`);
}

export function getNativePlatformPackageNames(targets: string[]): string[] {
  return targets.map(getNativePlatformPackageName);
}
