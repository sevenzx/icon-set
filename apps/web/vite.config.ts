import { sveltekit } from '@sveltejs/kit/vite';
import process from 'node:process';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': {
        target: process.env.VITE_API_PROXY_TARGET ?? 'http://127.0.0.1:3000',
        changeOrigin: true
      }
    }
  }
});
