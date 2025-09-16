import { defineConfig } from 'vite';
import solidPlugin from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solidPlugin()],
  server: {
    port: 5173,
    proxy: {
      // Proxy API to Axum during dev to avoid CORS
      '/auth': 'http://localhost:3000',
      '/expense-groups': 'http://localhost:3000',
      '/chat-bind-requests': 'http://localhost:3000',
      '/chat-bindings': 'http://localhost:3000'
    }
  },
  build: {
    target: 'esnext'
  }
});

