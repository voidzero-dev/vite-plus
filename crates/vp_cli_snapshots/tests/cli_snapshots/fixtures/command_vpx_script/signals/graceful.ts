const timer = setInterval(() => {}, 1000);
process.on('SIGINT', () => {
  console.log('cleaned up after SIGINT');
  clearInterval(timer);
});
const pid: number = process.pid;
console.log(`ready ${pid}`);
