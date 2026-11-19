import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';

const source = readFileSync('source.txt', 'utf8');
if (existsSync('manifest.txt') && readFileSync('manifest.txt', 'utf8') === source) {
  console.log('Skipped unchanged message');
} else {
  mkdirSync('generated', { recursive: true });
  writeFileSync('generated/message.txt', source.toUpperCase());
}
writeFileSync('manifest.txt', source);
