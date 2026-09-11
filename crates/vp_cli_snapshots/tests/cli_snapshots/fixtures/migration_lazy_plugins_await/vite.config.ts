import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

async function loadPlugin() {
  return { name: 'loaded-plugin' };
}

export default defineConfig({
  plugins: [react(), await loadPlugin()],
});
