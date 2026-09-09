// Converte os PNG de assets/icons/ para WebP e reescreve as referências nos dados.
//
// Motivo: a pasta de equipamentos tinha 423 PNG somando ~19 MB — que viram ~3,7 MB em WebP
// com as MESMAS dimensões (o app nunca reescala essas imagens pra cima). Isso é
// instalador menor, menos I/O de disco e menos memória de imagem decodificada.
//
// A conversão é feita por sharp (libwebp embutida), sem binário externo: antes
// isto exigia `cwebp` no PATH e `magick`/`sips` só para o tray.png, e cada um
// que faltasse abortava ou degradava a etapa em silêncio.
//
// Uso: npm run optimize-images [-- --dry]

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import sharp from 'sharp'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const ICONS = path.join(ROOT, 'assets/icons')
const QUALITY = 82
const DRY = process.argv.includes('--dry')

// icon.png é o ícone da aplicação: assets/icon.ico é gerado a partir dele, então o
// PNG grande fica. Em troca geramos tray.png (64px), que é o que o ícone da bandeja
// realmente desenha — decodificar um bitmap de 1024 pra 16px é desperdício puro.
const KEEP_PNG = new Set(['icon.png'])

// Arquivos que citam caminhos de imagem e precisam ser reescritos junto.
// `src/data.rs` é onde moram os ícones dos três estratagemas de apoio fixos,
// que não estão em nenhum JSON.
const REFERENCE_FILES = [
  'assets/data/equipment.json',
  'assets/data/stratagems.json',
  'src/data.rs'
]

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

const pngs = listPngs(ICONS)
if (!pngs.length) {
  console.log('Nenhum PNG para converter — nada a fazer.')
  process.exit(0)
}

let before = 0
let after = 0
const renames = new Map() // caminho relativo a assets/icons/, com / como separador

for (const png of pngs) {
  const webp = png.replace(/\.png$/i, '.webp')
  const relPng = path.relative(ICONS, png).split(path.sep).join('/')
  const relWebp = path.relative(ICONS, webp).split(path.sep).join('/')
  before += fs.statSync(png).size

  if (!DRY) {
    await sharp(png).webp({ quality: QUALITY }).toFile(webp)
    fs.rmSync(png)
  }
  if (fs.existsSync(webp)) after += fs.statSync(webp).size
  renames.set(relPng, relWebp)
}

// Ícone da bandeja em 64px: continua PNG porque é o que o decode →
// CreateIconIndirect consome; só o tamanho muda.
const trayTarget = path.join(ICONS, 'tray.png')
const iconSource = path.join(ICONS, 'icon.png')
if (!DRY && fs.existsSync(iconSource) && !fs.existsSync(trayTarget)) {
  await sharp(iconSource).resize(64, 64).png().toFile(trayTarget)
  console.log(`tray.png gerado (${(fs.statSync(trayTarget).size / 1024).toFixed(0)} KB)`)
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
