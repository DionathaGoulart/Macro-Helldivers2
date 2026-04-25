import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import electron from 'vite-plugin-electron'
import renderer from 'vite-plugin-electron-renderer'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(), // ATIVAÇÃO: Processamento de estilos do Tailwind v4
    electron([
      {
        entry: 'src/main/index.js',
        vite: {
          build: {
            outDir: 'dist-electron',
            rollupOptions: {
              external: ['@nut-tree-fork/nut-js', 'electron']
            }
          },
        },
      },
      {
        entry: 'src/main/preload.js',
        onready(options) {
          options.reload()
        },
        vite: {
          build: {
            outDir: 'dist-electron',
          },
        },
      },
    ]),
    renderer(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})
