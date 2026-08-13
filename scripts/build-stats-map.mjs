// Gera src/renderer/data/statsMap.json: mapeia os slugs da API do helldive.live
// (ex.: "sentry_gatling", "coyote", "OXYGENATOR") para os itens do app.
// Uso: npm run stats-map — iterar a tabela ALIAS até o log de não-casados zerar.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const API = 'https://utm7j5pjvi.us-east-1.awsapprunner.com'
const PATCH_ID = 12
const OUT = path.join(ROOT, 'src/renderer/data/statsMap.json')

const stratagems = JSON.parse(fs.readFileSync(path.join(ROOT, 'src/renderer/data/stratagems.json'), 'utf8'))
const equipment = JSON.parse(fs.readFileSync(path.join(ROOT, 'src/renderer/data/equipment.json'), 'utf8'))

const norm = (s) => s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/[^a-z0-9]/g, '')

// Sinônimos de tokens dos slugs → termos que aparecem nos nomes reais.
// '' = token sempre casa (prefixos sem informação, ex.: "sup_")
const TOKEN_SYNONYMS = {
  sup: [''],
  n: [''],
  backpack: ['backpack', 'pack'],
  grenade: ['grenade', 'g'],
  encampment: ['emplacement', 'battlement'],
  gl: ['grenadelauncher', 'gl'],
  mg: ['mg', 'machinegun'],
  hmg: ['hmg', 'heavymachinegun'],
  amr: ['amr', 'antimaterielrifle'],
  inc: ['inc', 'incendiary'],
  at: ['at', 'antitank'],
}

// Casos que o auto-match não resolve (typos da API, apelidos) — slug: nome exato do item
const ALIAS = {
  strategem: {
    backpack_ballistic: 'SH-20 Ballistic Shield Backpack',
    backpack_shield_directional: 'SH-51 Directional Shield',
    backpack_hellbomb: 'B-100 Portable Hellbomb',
    mines_infantry: 'MD-6 Anti-Personnel Minefield',
    eagle_strafe: 'Eagle Strafing Run',
    sentry_arc: 'A/ARC-3 Tesla Tower',
    // A wiki tirou o "Guard Dog" do nome de cada drone, mas os slugs da API mantêm o prefixo
    guard_rover: 'AX/LAS-5 Rover',
    guard_arc: 'AX/ARC-3 K-9',
    guard_hot: 'AX/FLAM-75 Hot Dog',
    guard_breath: 'AX/TX-13 Dog Breath',
  },
  weapons: {
    grenade_termite: 'G-123 Thermite',
    grenade_he: 'G-12 High Explosive',
    grenade_drone: 'G-50 Seeker',
    laser_pistol: 'LAS-7 Dagger',
    shock_batton: 'CQC-30 Stun Baton',
    shock_lance: 'CQC-19 Stun Lance',
    bushwacker: 'SG-22 Bushwhacker',
    adjucator: 'BR-14 Adjudicator',
    sabre: 'CQC-2 Saber',
    axe: 'CQC-5 Combat Hatchet',
    spray_n_pray: 'SG-225SP Breaker Spray&Pray',
  },
}

async function fetchStats(faction, type) {
  const url = `${API}/items_stats?faction=${faction}&patch_id=${PATCH_ID}&difficulty=0&mission=All&modifier=ALL&type=${type}`
  const res = await fetch(url, { headers: { 'User-Agent': 'MacroHelldivers2/1.0 (mapeamento)' } })
  if (!res.ok) throw new Error(`HTTP ${res.status} em ${url}`)
  return res.json()
}

function tokenVariants(token) {
  return TOKEN_SYNONYMS[token] || [token]
}

function slugMatches(slug, nameNorm) {
  return slug.split('_').every(tok => tokenVariants(tok).some(v => nameNorm.includes(v)))
}

// Casa slugs contra candidatos com "claim" guloso: slugs mais específicos primeiro,
// e cada item só pode ser reivindicado por um slug.
function matchSlugs(slugs, candidates, aliasTable) {
  const result = {}
  const claimed = new Set()
  const unmatched = []
  const bySpecificity = [...slugs].sort((a, b) => b.split('_').length - a.split('_').length || b.length - a.length)

  for (const slug of bySpecificity) {
    const alias = aliasTable[slug]
    if (alias) {
      const item = candidates.find(c => c.nome === alias)
      if (item) { result[slug] = item; claimed.add(item); continue }
      console.warn(`  ⚠ ALIAS aponta pra nome inexistente: ${slug} -> ${alias}`)
    }
    const matches = candidates.filter(c => slugMatches(slug, norm(c.nome)))
    if (matches.length === 0) { unmatched.push(slug); continue }
    const free = matches.filter(c => !claimed.has(c))
    const pool = free.length ? free : matches
    // Desempate: nome mais curto = candidato mais específico pro slug
    pool.sort((a, b) => norm(a.nome).length - norm(b.nome).length)
    result[slug] = pool[0]
    claimed.add(pool[0])
  }
  return { result, unmatched }
}

async function main() {
  const factions = ['terminid', 'automaton', 'illuminate']
  const slugSets = { strategem: new Set(), weapons: new Set(), armor: new Set() }

  console.log('Coletando slugs da API do helldive.live...')
  for (const f of factions) {
    for (const t of Object.keys(slugSets)) {
      const data = await fetchStats(f, t)
      Object.keys(data.items || {}).forEach(k => slugSets[t].add(k))
      await new Promise(r => setTimeout(r, 300))
    }
  }
  console.log('Slugs únicos:', Object.fromEntries(Object.entries(slugSets).map(([k, v]) => [k, v.size])))

  // Estratagemas -> id numérico do stratagems.json
  const strat = matchSlugs([...slugSets.strategem], stratagems, ALIAS.strategem)
  // Armas -> {cat, id} do equipment.json
  const weaponCandidates = ['primary', 'secondary', 'grenade'].flatMap(cat =>
    equipment[cat].map(item => ({ ...item, _cat: cat }))
  )
  const weap = matchSlugs([...slugSets.weapons], weaponCandidates, ALIAS.weapons)
  // Passivas -> nome exato (case-insensitive direto)
  const armorMap = {}
  const armorUnmatched = []
  for (const key of slugSets.armor) {
    const p = equipment.passives.find(p => norm(p.nome) === norm(key))
    if (p) armorMap[key] = p.nome
    else armorUnmatched.push(key)
  }

  const out = {
    strategem: Object.fromEntries(Object.entries(strat.result).map(([slug, item]) => [slug, item.id])),
    weapons: Object.fromEntries(Object.entries(weap.result).map(([slug, item]) => [slug, { cat: item._cat, id: item.id }])),
    armor: armorMap,
  }

  console.log('\nNÃO casados (adicionar em ALIAS e rodar de novo):')
  console.log('  strategem:', strat.unmatched.length ? strat.unmatched : 'nenhum ✔')
  console.log('  weapons:', weap.unmatched.length ? weap.unmatched : 'nenhum ✔')
  console.log('  armor:', armorUnmatched.length ? armorUnmatched : 'nenhum ✔')

  console.log('\nAmostra do mapeamento (conferir a olho):')
  Object.entries(strat.result).slice(0, 8).forEach(([s, i]) => console.log(`  ${s} -> ${i.nome}`))
  Object.entries(weap.result).slice(0, 8).forEach(([s, i]) => console.log(`  ${s} -> ${i.nome} (${i._cat})`))

  fs.writeFileSync(OUT, JSON.stringify(out, null, 2), 'utf8')
  console.log(`\n✔ ${OUT} escrito.`)
}

main().catch(e => { console.error('✖ Falha:', e); process.exit(1) })
