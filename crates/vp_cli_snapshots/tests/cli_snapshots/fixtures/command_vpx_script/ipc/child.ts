enum Reply {
  Echo = 'echo',
}

process.on('message', (message: unknown) => {
  if (message === 'burst') {
    // Sent right before exiting: vpx must deliver every one before it exits.
    for (let i = 0; i < 100; i++) {
      process.send!({ n: i });
    }
    process.exitCode = 3;
    process.disconnect();
    return;
  }
  process.send!({ [Reply.Echo]: message });
});
