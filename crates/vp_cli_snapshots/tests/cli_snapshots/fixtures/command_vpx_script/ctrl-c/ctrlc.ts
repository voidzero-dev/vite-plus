// Cleans up for 300 ms after Ctrl+C before it exits, so a CLI that exits first shows.
import { emitMilestone } from './milestone.ts';

const timer = setInterval(() => {}, 1000);
process.on('SIGINT', () => {
  console.log('SIGINT: cleaning up');
  setTimeout(() => {
    console.log('cleaned up');
    clearInterval(timer);
  }, 300);
});
emitMilestone('ready');
