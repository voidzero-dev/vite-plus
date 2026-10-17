// Exits with a code of its own once the process that started vpx closes the channel.
process.on('message', (message: unknown) => {
  process.send!({ echo: message });
});
process.on('disconnect', () => {
  process.exit(4);
});
