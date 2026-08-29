export function emitMilestone(name: string): void {
  setImmediate(() => {
    const encodedName = Buffer.from(name).toString('base64url');
    process.stdout.write(
      `\x1b]2;pty-terminal-test:${'0'.repeat(32)}:${encodedName}\x1b\\`,
    );
  });
}
