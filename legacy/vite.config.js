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
              // electron-updater fica de fora do bundle pra que o require tardio de
              // index.js seja mesmo tardio: embutido, ele (mais js-yaml, semver e
              // lodash) era parseado antes da primeira janela aparecer.
              // O electron-builder já copia as dependências de produção.
              external: ['@nut-tree-fork/nut-js', 'electron', 'electron-updater']
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
  // Os JSON de dados viram JSON.parse('...') em vez de literal de objeto: o V8 parseia
  // string bem mais rápido, e são ~115 KB avaliados no boot de cada janela
  json: {
    stringify: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})
