const archMap: Record<string, string> = {
  aarch64: 'arm64',
  x86_64: 'x64',
};

function getNativePlatformPackageSuffix(target: string): string {
  const [archTarget, ...platformParts] = target.split('-');
  const arch = archMap[archTarget];
  const platform = platformParts.join('-');

  if (!arch) {
    throw new Error(`Unsupported NAPI target architecture: ${target}`);
  }

  if (platform === 'apple-darwin') {
    return `darwin-${arch}`;
  }
  if (platform === 'unknown-linux-gnu') {
    return `linux-${arch}-gnu`;
  }
  if (platform === 'unknown-linux-musl') {
    return `linux-${arch}-musl`;
  }
  if (platform === 'pc-windows-msvc') {
    return `win32-${arch}-msvc`;
  }

  throw new Error(`Unsupported NAPI target platform: ${target}`);
}

export function getNativePlatformPackageName(target: string): string {
  return `@voidzero-dev/vite-plus-${getNativePlatformPackageSuffix(target)}`;
}

export function getNativePlatformPackageNames(targets: string[]): string[] {
  return targets.map(getNativePlatformPackageName);
}
