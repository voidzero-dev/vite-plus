// Counts the SIGINTs it gets and stops 300 ms after the first, long after a watcher
// would have passed on a second one.
let count = 0;
const timer = setInterval(() => {}, 1000);
process.on('SIGINT', () => {
  count++;
  console.log(`SIGINT ${count}`);
  setTimeout(() => clearInterval(timer), 300);
});
const pid: number = process.pid;
console.log(`ready ${pid}`);
