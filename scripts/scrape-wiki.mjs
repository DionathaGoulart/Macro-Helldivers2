// Scraper de equipamentos do Helldivers 2 via API Cargo da wiki (helldivers.wiki.gg).
// Gera assets/data/equipment.json e baixa as imagens pra assets/icons/equipment/.
// Uso: npm run scrape [-- --refresh]   (Node 18+, sem dependências)
//
// Depois do scrape rode `npm run optimize-images`: os PNG baixados aqui viram
// WebP e as referências nos JSON são reescritas junto. Sem esse segundo passo o
// repositório fica com `equipment.json` apontando para `.png` que ninguém commita.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const API = 'https://helldivers.wiki.gg/api.php'
const FILEPATH_URL = 'https://helldivers.wiki.gg/wiki/Special:FilePath/'
const OUT_JSON = path.join(ROOT, 'assets/data/equipment.json')
const IMG_DIR = path.join(ROOT, 'assets/icons/equipment')

const sleep = (ms) => new Promise(r => setTimeout(r, ms))

// --refresh rebaixa tudo, inclusive o que já está em disco. O padrão reaproveita
// PNG já baixado (o mesmo run pode citar a mesma arte duas vezes), mas nunca SVG:
// esses sobrevivem ao `optimize-images` com o nome final, então o teste de
// existência os congelava para sempre — foi assim que o ícone da passiva
// True Grit ficou preso na versão PNG antiga depois que a wiki trocou por SVG.
const REFRESH = process.argv.includes('--refresh')

// A API devolve strings com entidades HTML ("Liberty&#39;s Herald") — decodifica tudo
const decodeEntities = (s) => typeof s === 'string'
  ? s.replace(/&#0?39;|&apos;/g, "'").replace(/&quot;/g, '"').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&')
  : s

async function fetchJson(params) {
  const url = `${API}?${new URLSearchParams({ format: 'json', ...params })}`
  const res = await fetch(url, { headers: { 'User-Agent': 'MacroHelldivers2-scraper/1.0' } })
  if (!res.ok) throw new Error(`HTTP ${res.status} em ${url}`)
  return res.json()
}

// Pagina cargoquery até esgotar; normaliza as chaves ("stam regen" -> "stam_regen")
async function cargoAll(tables, fields, where) {
  const rows = []
  for (let offset = 0; ; offset += 500) {
    const params = { action: 'cargoquery', limit: '500', offset: String(offset), tables, fields }
    if (where) params.where = where
    const json = await fetchJson(params)
    const batch = (json.cargoquery || []).map(r => {
      const out = {}
      for (const [k, v] of Object.entries(r.title)) out[k.replace(/ /g, '_')] = decodeEntities(v)
      return out
    })
    rows.push(...batch)
    if (batch.length < 500) break
  }
  return rows
}

function stripWikitext(s) {
  if (!s) return ''
  return s
    .replace(/\[\[File:[^\]]*\]\]/gi, '')
    .replace(/\[\[[^\]|]*\|([^\]]*)\]\]/g, '$1')
    .replace(/\[\[([^\]]*)\]\]/g, '$1')
    .replace(/\{\{Currency\|([^|}]+)\|([^|}]+)[^}]*\}\}/gi, '$2 $1')
    .replace(/\{\{[^}]*\}\}/g, ' ')
    .replace(/'''?/g, '')
    .replace(/<br\s*\/?\s*>/gi, ' ')
    .replace(/<[^>]+>/g, '')
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/\s+/g, ' ')
    .trim()
}

const slug = (s) => s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '')
  .replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '')

// "[[X Warbond#Page 2|Halo: ODST]] <small>P2</small>" -> "Halo: ODST"; "[[Superstore]]" -> "Superstore"
const parseWarbond = (s) => {
  const cleaned = stripWikitext(s).replace(/\s*P\d+$/, '').trim()
  return cleaned || null
}

const isJunk = (row) => /april fools/i.test(row.page || row.title || '') || /\/zh$/.test(row.page || '')

// ---------- Coleta ----------

async function getWeapons(category) {
  const rows = await cargoAll(
    'Weapons',
    '_pageName=page,title,image,weapon_type,damage,capacity,fire_rate',
    `weapon_category='${category}'`
  )
  const seen = new Set()
  return rows.filter(r => !isJunk(r) && r.title && !seen.has(r.title) && seen.add(r.title)).map(r => ({
    nome: r.title,
    _file: r.image || null,
    tipo: stripWikitext(r.weapon_type) || null,
    dano: stripWikitext(r.damage) || null,
    capacidade: stripWikitext(r.capacity) || null,
    cadencia: stripWikitext(r.fire_rate) || null,
  }))
}

async function getArmor() {
  const rows = await cargoAll(
    'Armor',
    '_pageName=page,title,type,armor,speed,stam_regen,passive,source,image',
    "type IN ('Light','Medium','Heavy')"
  )
  const seen = new Set()
  return rows.filter(r => {
    const key = `${r.title}|${r.type}`
    return !isJunk(r) && r.title && !seen.has(key) && seen.add(key)
  }).map(r => ({
    nome: r.title,
    _file: r.image || null,
    peso: r.type,
    armor: Number(r.armor) || null,
    speed: Number(r.speed) || null,
    stamina: Number(r.stam_regen) || null,
    passive: stripWikitext(r.passive) || null,
    warbond: parseWarbond(r.source),
  }))
}

async function getHelmets() {
  const rows = await cargoAll('Armor', '_pageName=page,title,source,image', "type='Helmet'")
  const seen = new Set()
  return rows.filter(r => !isJunk(r) && r.title && !seen.has(r.title) && seen.add(r.title))
    .map(r => ({ nome: r.title, _file: r.image || null, warbond: parseWarbond(r.source) }))
}

async function getCapes() {
  const rows = await cargoAll('Cosmetics', '_pageName=page,title,source,image', "type='Cape'")
  const seen = new Set()
  return rows.filter(r => !isJunk(r) && r.title && !seen.has(r.title) && seen.add(r.title))
    .map(r => ({ nome: r.title, _file: r.image || null, warbond: parseWarbond(r.source) }))
}

async function getPassives() {
  const rows = await cargoAll('Armor_Passive', '_pageName=page,title,image,description')
  return rows.filter(r => !isJunk(r) && r.title).map(r => ({
    nome: r.title,
    _file: r.image || null,
    descricao: stripWikitext(r.description) || null,
  }))
}

// Metadados dos estratagemas: tipo (Support Weapon/Backpack/Sentry/...) e se ocupa mochila.
// Usado pelos filtros de build (balanceado, máx 1 torreta) — sem imagens, o app já tem as suas.
async function getStratagemInfo() {
  const rows = await cargoAll('Stratagems', '_pageName=page,title,stratagem_type,traits')
  const seen = new Set()
  return rows.filter(r => !isJunk(r) && r.title && !seen.has(r.title) && seen.add(r.title)).map(r => ({
    nome: r.title,
    type: r.stratagem_type || null,
    backpack: r.stratagem_type === 'Backpack' || /backpack/i.test(r.traits || ''),
  }))
}

// Warbonds: a origem de quase todo item. O app já guarda o nome da warbond em
// armor/helmet/cape; aqui vem a ficha (capa, data, tipo, preço) pra exibir.
async function getWarbonds() {
  const rows = await cargoAll('Warbonds', '_pageName=page,title,image,date,type,cost')
  const seen = new Set()
  return rows.filter(r => !isJunk(r) && r.title && !seen.has(r.title) && seen.add(r.title)).map(r => ({
    nome: r.title,
    _file: r.image || null,
    tipo: r.type || null,
    data: (r.date || '').slice(0, 10) || null,
    custo: stripWikitext(r.cost) || null,
  })).sort((a, b) => (a.data || '').localeCompare(b.data || ''))
}

async function getBoosters() {
  const json = await fetchJson({ action: 'parse', page: 'Boosters', prop: 'wikitext' })
  const wikitext = json.parse.wikitext['*']
  const boosters = []
  // Linhas da wikitable: célula do ícone, nome, descrição, warbond, preço
  const rowRe = /\|\[\[File:([^\]|]+?)(?:\|[^\]]*)?\]\]\s*\n\|\s*\[\[([^\]|]+?)(?:\|[^\]]*)?\]\]\s*\n\|([\s\S]*?)\n\|/g
  let m
  while ((m = rowRe.exec(wikitext)) !== null) {
    boosters.push({ nome: m[2].trim(), _file: m[1].trim(), descricao: stripWikitext(m[3]) })
  }
  return boosters
}

// ---------- Imagens ----------

async function downloadImage(file, destBase, width = 200) {
  const isSvg = /\.svg$/i.test(file)
  const ext = isSvg ? '.svg' : '.png'
  const dest = `${destBase}${ext}`
  const destAbs = path.join(IMG_DIR, dest)
  const cacheServe = !REFRESH && !isSvg && fs.existsSync(destAbs) && fs.statSync(destAbs).size > 0
  if (cacheServe) return dest
  const url = `${FILEPATH_URL}${encodeURIComponent(file)}${isSvg ? '' : `?width=${width}`}`
  // A wiki aplica rate-limit agressivo (429): backoff exponencial e paciência
  const backoffs = [2000, 5000, 15000, 30000, 60000]
  for (let attempt = 0; attempt <= backoffs.length; attempt++) {
    try {
      const res = await fetch(url, { headers: { 'User-Agent': 'MacroHelldivers2-scraper/1.0' } })
      if (res.status === 429) {
        const retryAfter = Number(res.headers.get('retry-after')) * 1000 || backoffs[Math.min(attempt, backoffs.length - 1)]
        await sleep(retryAfter)
        continue
      }
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const buf = Buffer.from(await res.arrayBuffer())
      if (buf.length < 100) throw new Error('arquivo suspeito (muito pequeno)')
      fs.writeFileSync(destAbs, buf)
      return dest
    } catch (e) {
      if (attempt === backoffs.length) {
        console.warn(`  ⚠ imagem falhou: ${file} (${e.message})`)
        return null
      }
      await sleep(backoffs[attempt])
    }
  }
  console.warn(`  ⚠ imagem falhou (rate-limit persistente): ${file}`)
  return null
}

async function attachImages(items, categoria) {
  // Capas de warbond são banners 2:1 — 200px de largura fica ilegível
  const width = categoria === 'warbond' ? 400 : 200
  // Sequencial de propósito: paralelismo dispara o rate-limit da wiki (429)
  for (let i = 0; i < items.length; i++) {
    const item = items[i]
    item.id = `${categoria}-${slug(item.nome)}`
    if (item._file) {
      const saved = await downloadImage(item._file, `${categoria}-${slug(item.nome)}`, width)
      item.imagem = saved ? `equipment/${saved}` : null
      await sleep(250)
    } else {
      item.imagem = null
    }
    delete item._file
    process.stdout.write(`\r  ${categoria}: ${i + 1}/${items.length} imagens`)
  }
  process.stdout.write('\n')
  return items
}

// ---------- Main ----------

async function main() {
  fs.mkdirSync(IMG_DIR, { recursive: true })

  console.log('Coletando dados da wiki (Cargo API)...')
  const [primary, secondary, grenade, armor, helmet, cape, passives, booster, warbond, stratagemInfo] = await Promise.all([
    getWeapons('Primary Weapons'),
    getWeapons('Secondary Weapons'),
    getWeapons('Throwables'),
    getArmor(),
    getHelmets(),
    getCapes(),
    getPassives(),
    getBoosters(),
    getWarbonds(),
    getStratagemInfo(),
  ])

  const data = { primary, secondary, grenade, armor, helmet, cape, booster, passives, warbond, stratagemInfo }

  console.log('Contagens:', Object.fromEntries(Object.entries(data).map(([k, v]) => [k, v.length])))
  const empty = Object.entries(data).filter(([, v]) => v.length === 0).map(([k]) => k)
  if (empty.length) {
    console.error(`✖ Categorias vazias: ${empty.join(', ')} — abortando sem escrever.`)
    process.exit(1)
  }

  console.log('Baixando imagens (cache: arquivos existentes são pulados)...')
  for (const [categoria, items] of Object.entries(data)) {
    if (categoria === 'stratagemInfo') continue // só metadados, o app já tem os ícones
    await attachImages(items, categoria)
  }

  fs.writeFileSync(OUT_JSON, JSON.stringify(data, null, 2), 'utf8')
  const semImagem = Object.entries(data).filter(([k]) => k !== 'stratagemInfo')
    .flatMap(([, v]) => v).filter(i => !i.imagem).length
  console.log(`✔ ${OUT_JSON} escrito. Itens sem imagem: ${semImagem}`)
}

main().catch(e => { console.error('✖ Falha no scrape:', e); process.exit(1) })
