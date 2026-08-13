// Converte os PNG de public/ para WebP e reescreve as referências nos dados.
//
// Motivo: public/equipment tinha 423 PNG somando ~19 MB — que viram ~3,7 MB em WebP
// com as MESMAS dimensões (o app nunca reescala essas imagens pra cima). Isso é
// instalador menor, menos I/O de disco e menos memória de imagem decodificada.
//
// Requer `cwebp` (libwebp) no PATH:
//   Windows: winget install Google.LibWebP     macOS: brew install webp
//
// Uso: npm run optimize-images [-- --dry]

import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const PUBLIC = path.join(ROOT, 'public')
const QUALITY = 82
const DRY = process.argv.includes('--dry')

// icon.png é o ícone da aplicação: o electron-builder exige um PNG grande pra gerar
// .ico/.icns, então ele fica. Em troca geramos tray.png (64px), que é o que o Tray
// realmente desenha — decodificar um bitmap de 1024 pra 16px é desperdício puro.
const KEEP_PNG = new Set(['icon.png'])

// Arquivos que citam caminhos de imagem e precisam ser reescritos junto
const REFERENCE_FILES = [
  'src/renderer/data/equipment.json',
  'src/renderer/data/stratagems.json',
  'src/shared/constants.js'
]

function ensureTool(name, args) {
  try {
    execFileSync(name, args, { stdio: 'ignore' })
    return true
  } catch (e) {
    return false
  }
}

function listPngs(dir) {
  const out = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) out.push(...listPngs(full))
    else if (entry.name.toLowerCase().endsWith('.png') && !KEEP_PNG.has(entry.name)) out.push(full)
  }
  return out
}

function human(bytes) {
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

if (!ensureTool('cwebp', ['-version'])) {
  console.error('cwebp não encontrado no PATH. Instale libwebp:')
  console.error('  Windows: winget install Google.LibWebP')
  console.error('  macOS:   brew install webp')
  process.exit(1)
}

const pngs = listPngs(PUBLIC)
if (!pngs.length) {
  console.log('Nenhum PNG para converter — nada a fazer.')
  process.exit(0)
}

let before = 0
let after = 0
const renames = new Map() // caminho relativo a public/, com / como separador

for (const png of pngs) {
  const webp = png.replace(/\.png$/i, '.webp')
  const relPng = path.relative(PUBLIC, png).split(path.sep).join('/')
  const relWebp = path.relative(PUBLIC, webp).split(path.sep).join('/')
  before += fs.statSync(png).size

  if (!DRY) {
    execFileSync('cwebp', ['-quiet', '-q', String(QUALITY), png, '-o', webp])
    fs.rmSync(png)
  }
  if (fs.existsSync(webp)) after += fs.statSync(webp).size
  renames.set(relPng, relWebp)
}

// Ícone do tray em 64px (cwebp não redimensiona PNG→PNG; usamos o próprio cwebp pra
// gerar e o Electron aceita PNG, então mantemos PNG via sips/magick quando existir)
const trayTarget = path.join(PUBLIC, 'tray.png')
const iconSource = path.join(PUBLIC, 'icon.png')
if (!DRY && fs.existsSync(iconSource) && !fs.existsSync(trayTarget)) {
  const resizer =
    ensureTool('magick', ['-version']) ? ['magick', [iconSource, '-resize', '64x64', trayTarget]] :
    ensureTool('sips', ['--version']) ? ['sips', ['-z', '64', '64', iconSource, '--out', trayTarget]] :
    null
  if (resizer) {
    execFileSync(resizer[0], resizer[1], { stdio: 'ignore' })
    console.log(`tray.png gerado (${(fs.statSync(trayTarget).size / 1024).toFixed(0)} KB)`)
  } else {
    console.warn('Nem magick nem sips disponíveis: tray.png não gerado (o app cai no icon.png)')
  }
}

// Reescreve as referências
for (const rel of REFERENCE_FILES) {
  const file = path.join(ROOT, rel)
  if (!fs.existsSync(file)) continue
  let text = fs.readFileSync(file, 'utf8')
  let hits = 0
  for (const [from, to] of renames) {
    if (!text.includes(from)) continue
    text = text.split(from).join(to)
    hits++
  }
  if (hits && !DRY) fs.writeFileSync(file, text)
  console.log(`${rel}: ${hits} referência(s) ${DRY ? 'seriam ' : ''}atualizada(s)`)
}

console.log(`${pngs.length} PNG ${DRY ? 'seriam convertidos' : 'convertidos'}: ${human(before)} → ${DRY ? '?' : human(after)}`)
