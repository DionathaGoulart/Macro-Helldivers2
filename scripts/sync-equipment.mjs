// Gera assets/data/equipment.json e os ícones de assets/icons/equipment/ a partir
// da API de dados (helldivers-api.dionatha.com.br), que raspa a wiki uma vez por dia.
// Uso: npm run scrape [-- --force]   (Node 18+ e o sharp do package.json)
//
// É o mesmo dado que o app baixa sozinho entre um release e outro
// (src/equipment_sync.rs), com as mesmas regras de conversão: o que este script
// grava é o que o app teria montado em runtime. Rodar a cada release traz o
// arsenal inteiro em dia, inclusive o que mudou em item que já existia (stats de
// patch, nome, descrição) e o que saiu do jogo, coisas que o app sozinho não faz.
//
// Regras que o app assume:
// - Id de item que já existe nunca muda. Build salva e statsMap.json apontam para
//   ele; um item renomeado na wiki ("CPH-26 Commandant (Armor)" → "CPH-26
//   Commandant") troca de nome e mantém o id. Item novo ganha `{categoria}-{slug}`,
//   o mesmo id que o app deu a ele quando o baixou antes deste release.
// - Listas em ordem alfabética (warbonds por data de lançamento).
// - Ícone cabe em 256px, o maior tamanho que o app guarda (MAX_EDGE_PX); a capa de
//   warbond tem 400px de largura.
//
// Trava: se uma categoria perderia mais de 10% dos itens, nada é gravado (a API
// quebrada não pode esvaziar o arsenal). `--force` grava assim mesmo.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import sharp from 'sharp'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const API = 'https://helldivers-api.dionatha.com.br'
const OUT_JSON = path.join(ROOT, 'assets/data/equipment.json')
const IMG_DIR = path.join(ROOT, 'assets/icons/equipment')
const HEADERS = { 'User-Agent': 'MacroHelldivers2-sync/2.0' }
const FORCE = process.argv.includes('--force')

const MAX_EDGE = 256
const WARBOND_WIDTH = 400
const QUALITY = 82
const MAX_LOSS = 0.1
const DOWNLOADS_AT_ONCE = 8

// Mesmo `slug` do app (`equipment_sync::slug`) e do scrape antigo: minúsculas sem
// acento, e o resto vira `-`.
const slug = (s) => s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '')
  .replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '')

// Mesma chave do `equipment_sync::name_key`: sem caixa, acento, pontuação nem o
// desambiguador de página da wiki no fim ("(Armor)").
const nameKey = (s) => s.trim().replace(/ \([^()]*\)$/, '').toLowerCase().normalize('NFD')
  .replace(/[̀-ͯ]/g, '').replace(/[^a-z0-9]/g, '')

const capitalized = (s) => (s ? s[0].toUpperCase() + s.slice(1) : '')

// "Halo: ODST P2" → "Halo: ODST", como o app faz com o `source.label`. Com a
// grafia da coleção de warbonds quando é uma: a API escreve "Castellan’s Creed"
// com apóstrofo curvo nas capas e reto nas armaduras, e o sorteio de set casa a
// capa com a armadura por igualdade de texto.
let warbondNames = new Map()
const warbondOf = (item) => {
  const label = (item.source?.label || '').trim().replace(/ P\d+$/, '').trim()
  return warbondNames.get(nameKey(label)) || label
}

const stat = (item, key) => {
  const value = item.statsRaw?.[key]
  if (value === undefined || value === null) return null
  const text = String(value).trim()
  return text || null
}

async function fetchJson(collection) {
  const url = `${API}/v1/${collection}.json`
  const res = await fetch(url, { headers: HEADERS })
  if (!res.ok) throw new Error(`HTTP ${res.status} em ${url}`)
  const json = await res.json()
  return json.data.filter(item => !item.upcoming)
}

// ---------- Conversão (espelho do `from_api` de src/equipment_sync.rs) ----------

const WEAPON_CATEGORIES = { primary: 'primary', secondary: 'secondary', throwable: 'grenade' }

function weapon(item) {
  if (item.category === 'throwable') {
    return {
      nome: item.name,
      tipo: capitalized(item.subcategory || ''),
      dano: stat(item, 'Damage') || '',
      capacidade: stat(item, 'Capacity'),
      cadencia: null,
    }
  }
  return {
    nome: item.name,
    tipo: stat(item, 'Weapon Type') || '',
    dano: stat(item, 'Standard Damage') || '',
    capacidade: stat(item, 'Capacity'),
    cadencia: stat(item, 'Fire Rate'),
  }
}

function armor(item, passives) {
  const passive = passives.get(item.passiveId)
  const numbers = [item.armorRating, item.speed, item.staminaRegen]
  if (!passive || !item.weight || numbers.some(n => typeof n !== 'number')) return null
  return {
    nome: item.name,
    peso: capitalized(item.weight),
    armor: Math.round(item.armorRating),
    speed: Math.round(item.speed),
    stamina: Math.round(item.staminaRegen),
    passive,
    warbond: warbondOf(item),
  }
}

const cosmetic = (item) => ({ nome: item.name, warbond: warbondOf(item) })

const described = (item, text) => ({ nome: item.name, descricao: (text || '').trim() })

function cost(item) {
  if (!item.cost || !item.cost.amount) return 'Free'
  const amount = item.cost.amount.toLocaleString('en-US')
  return item.cost.currency === 'super_credits' ? `${amount} Super Credits` : `${amount} ${item.cost.currency}`
}

const warbond = (item) => ({
  nome: item.name,
  tipo: capitalized(item.type),
  data: item.releaseDate || null,
  custo: cost(item),
})

// O que as regras de build leem: arma de apoio, sentinela e se ocupa a mochila.
const STRATAGEM_TYPES = {
  support_weapon: 'Support Weapon',
  backpack: 'Backpack',
  sentry: 'Sentry',
  orbital: 'Orbital',
  eagle: 'Eagle',
  emplacement: 'Emplacement',
  vehicle: 'Vehicle',
}

const stratagemInfo = (item) => ({
  nome: item.name,
  type: STRATAGEM_TYPES[item.kind] || capitalized(item.kind || '') || null,
  backpack: item.kind === 'backpack' || (item.traitIds || []).includes('backpack'),
})

// ---------- Ids e ordem ----------

// Reaproveita o id do item que já estava no JSON (pelo nome ou por um apelido);
// item novo ganha `{categoria}-{slug}`.
function assignIds(categoria, items, current) {
  const byKey = new Map((current || []).map(old => [nameKey(old.nome), old.id]))
  const used = new Set()
  const out = []
  for (const { api, entry } of items) {
    const keys = [api.name, ...(api.aliases || [])].map(nameKey)
    const reused = keys.map(key => byKey.get(key)).find(id => id && !used.has(id))
    const id = reused || `${categoria}-${slug(api.name)}`
    if (used.has(id)) {
      console.warn(`  ⚠ ${categoria}: id repetido ${id} (${api.name}); item ignorado`)
      continue
    }
    used.add(id)
    out.push({ api, entry: { ...entry, id } })
  }
  return out
}

const byName = (a, b) => (a.entry.nome < b.entry.nome ? -1 : a.entry.nome > b.entry.nome ? 1 : 0)

// ---------- Imagens ----------

async function downloadIcon(url, destAbs, width) {
  const res = await fetch(`${API}${url}`, { headers: HEADERS })
  if (!res.ok) throw new Error(`HTTP ${res.status} em ${url}`)
  const input = Buffer.from(await res.arrayBuffer())
  const resized = width
    ? sharp(input).resize({ width, withoutEnlargement: true })
    : sharp(input).resize({ width: MAX_EDGE, height: MAX_EDGE, fit: 'inside', withoutEnlargement: true })
  const out = await resized.webp({ quality: QUALITY }).toBuffer()
  // Só grava o que mudou: o mesmo ícone reencodado dá os mesmos bytes, e o git
  // não vê diferença.
  if (fs.existsSync(destAbs) && fs.readFileSync(destAbs).equals(out)) return
  fs.writeFileSync(destAbs, out)
}

async function inPool(tasks, size) {
  let next = 0
  const workers = Array.from({ length: size }, async () => {
    while (next < tasks.length) await tasks[next++]()
  })
  await Promise.all(workers)
}

// ---------- Main ----------

async function main() {
  const current = JSON.parse(fs.readFileSync(OUT_JSON, 'utf8'))

  console.log('Lendo coleções da API...')
  const [passivesApi, weaponsApi, armorsApi, helmetsApi, capesApi, boostersApi, warbondsApi, stratagemsApi] =
    await Promise.all(['passives', 'weapons', 'armors', 'helmets', 'capes', 'boosters', 'warbonds', 'stratagems']
      .map(fetchJson))
  const passiveNames = new Map(passivesApi.map(p => [p.id, p.name]))
  warbondNames = new Map(warbondsApi.map(w => [nameKey(w.name), w.name]))

  const pick = (api, convert) => api.map(item => ({ api: item, entry: item.name && item.image?.url ? convert(item) : null }))
    .filter(({ api, entry }) => {
      if (!entry) console.warn(`  ⚠ ${api.id}: dado incompleto; ignorado`)
      return entry
    })
  const weaponsOf = (cat) => pick(weaponsApi.filter(w => WEAPON_CATEGORIES[w.category] === cat), weapon)

  const raw = {
    primary: weaponsOf('primary'),
    secondary: weaponsOf('secondary'),
    grenade: weaponsOf('grenade'),
    armor: pick(armorsApi, item => armor(item, passiveNames)),
    helmet: pick(helmetsApi, cosmetic),
    cape: pick(capesApi, cosmetic),
    booster: pick(boostersApi, item => described(item, item.effect || item.description)),
    passives: pick(passivesApi, item => described(item, item.description)),
    warbond: pick(warbondsApi, warbond),
  }

  const data = {}
  const images = []
  const blocked = []
  for (const [categoria, items] of Object.entries(raw)) {
    const withIds = assignIds(categoria, items, current[categoria])
    if (categoria === 'warbond') withIds.sort((a, b) => (a.entry.data || '').localeCompare(b.entry.data || ''))
    else withIds.sort(byName)

    const before = new Map((current[categoria] || []).map(old => [old.id, old.nome]))
    const after = new Set(withIds.map(({ entry }) => entry.id))
    const added = withIds.filter(({ entry }) => !before.has(entry.id)).map(({ entry }) => entry.nome)
    const removed = [...before].filter(([id]) => !after.has(id)).map(([, nome]) => nome)
    const renamed = withIds.filter(({ entry }) => before.has(entry.id) && before.get(entry.id) !== entry.nome)
      .map(({ entry }) => `${before.get(entry.id)} → ${entry.nome}`)
    console.log(`  ${categoria}: ${withIds.length} (+${added.length} -${removed.length})`)
    for (const nome of added) console.log(`    + ${nome}`)
    for (const nome of removed) console.log(`    - ${nome}`)
    for (const change of renamed) console.log(`    ~ ${change}`)
    if (removed.length > before.size * MAX_LOSS) blocked.push(categoria)

    data[categoria] = withIds.map(({ api, entry }) => {
      const file = `${entry.id}.webp`
      images.push({ url: api.image.url, file, width: categoria === 'warbond' ? WARBOND_WIDTH : null })
      return { ...entry, imagem: `equipment/${file}` }
    })
  }
  data.stratagemInfo = stratagemsApi.filter(s => s.availability === 'loadout').map(stratagemInfo)
    .sort((a, b) => (a.nome < b.nome ? -1 : a.nome > b.nome ? 1 : 0))

  if (blocked.length && !FORCE) {
    console.error(`✖ ${blocked.join(', ')} perderia(m) mais de ${MAX_LOSS * 100}% dos itens. Nada gravado; confira a API ou rode com --force.`)
    process.exit(1)
  }

  console.log(`Baixando ${images.length} ícones...`)
  fs.mkdirSync(IMG_DIR, { recursive: true })
  let failed = 0
  await inPool(images.map(image => async () => {
    try {
      await downloadIcon(image.url, path.join(IMG_DIR, image.file), image.width)
    } catch (e) {
      failed++
      console.warn(`  ⚠ ícone falhou: ${image.file} (${e.message})`)
    }
  }), DOWNLOADS_AT_ONCE)
  if (failed) {
    console.error(`✖ ${failed} ícone(s) falharam. Nada gravado; rode de novo.`)
    process.exit(1)
  }

  // Ícones que nenhum item cita mais.
  const keep = new Set(images.map(image => image.file))
  let pruned = 0
  for (const file of fs.readdirSync(IMG_DIR)) {
    if (!keep.has(file)) {
      fs.rmSync(path.join(IMG_DIR, file))
      pruned++
    }
  }

  fs.writeFileSync(OUT_JSON, JSON.stringify(data, null, 2), 'utf8')
  console.log(`✔ ${path.relative(ROOT, OUT_JSON)} escrito; ${pruned} ícone(s) antigo(s) removido(s).`)
  console.log('  Rode `npm run stats-map` para conferir os slugs do helldive.live.')
}

main().catch(e => { console.error('✖ Falha ao sincronizar o equipamento:', e); process.exit(1) })
