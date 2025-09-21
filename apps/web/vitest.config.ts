/// <reference types="vitest" />
import { defineConfig } from 'vite'
import solidPlugin from 'vite-plugin-solid'

export default defineConfig({
  plugins: [solidPlugin()],
  test: {
    environment: 'jsdom',
    globals: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.d.ts',
        'cypress/',
        'test/',
        'tests/',
        '**/*.config.*',
        '**/*.config.ts',
        'dist/',
        '.yarn/',
      ],
    },
  },
})