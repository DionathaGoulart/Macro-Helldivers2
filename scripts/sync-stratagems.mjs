// Sincroniza assets/data/stratagems.json e os ícones de assets/icons/stratagems/
// com a API de dados (helldivers-api.dionatha.com.br), que é alimentada pela wiki.
// Uso: npm run sync-stratagems   (Node 18+, sem dependências)
//
// A ordem do arquivo é a do jogo e é curada à mão: a wiki não a tem. O script
// nunca reordena o que já existe. Estratagema novo entra no fim do subgrupo dele
// (cor → tipo → corpo a corpo / comuns / descartáveis / com mochila), a mesma
// regra que o app aplica em runtime (`src/data_sync.rs`). Depois é só mover a
// entrada para a posição exata do jogo.
//
// O id de entrada nova sai do slug (`stableId`, igual ao `stable_id` do app). É o
// número que o app já deu ao estratagema quando o baixou da API antes deste
// release, então slot e build salvos por usuários continuam apontando para ele.
// Nunca renumere à mão.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const API = 'https://helldivers-api.dionatha.com.br'
const OUT_JSON = path.join(ROOT, 'assets/data/stratagems.json')
const ICON_DIR = path.join(ROOT, 'assets/icons/stratagems')
const HEADERS = { 'User-Agent': 'MacroHelldivers2-sync/2.0' }

// Menos que isto e a API está quebrada: aborta sem escrever. Mesmo valor do app.
const MIN_LOADOUT = 80

const PERMIT_TAG = { offensive: 'Offensive', supply: 'Supply', defensive: 'Defensive' }

// Reforço, Ressuprimento e Rearme da Águia: os mesmos de SUPPORT_STRATS em src/data.rs.
const SUPPORT_CODEX = {
  Reinforce: ['UP', 'DOWN', 'RIGHT', 'LEFT', 'UP'],
  Resupply: ['DOWN', 'DOWN', 'UP', 'RIGHT'],
  'Eagle Rearm': ['UP', 'UP', 'LEFT', 'UP', 'RIGHT'],
}
const DIR = { up: 'UP', down: 'DOWN', left: 'LEFT', right: 'RIGHT' }

// FNV-1a de 32 bits com o bit alto ligado; tem que dar o mesmo que o
// `stable_id` de src/data_sync.rs (o teste de lá fixa dois valores).
function stableId(slug) {
  let hash = 0x811c9dc5
  for (const byte of Buffer.from(slug, 'utf8')) {
    hash ^= byte
    hash = Math.imul(hash, 0x01000193) >>> 0
  }
  return (hash | 0x80000000) >>> 0
}

const slugHasWord = (item, prefix) => item.id.split('-').some(w => w.startsWith(prefix))
const hasTrait = (item, name) => (item.traitIds || []).includes(name)

// Espelho do `group_of` do app.
function groupOf(item) {
  let family = item.kind
  if (item.kind === 'support_weapon') {
    if (hasTrait(item, 'melee')) family = 'melee'
    else if (hasTrait(item, 'expendable')) family = 'expendable'
    else if (hasTrait(item, 'backpack')) family = 'backpack_weapon'
    else family = 'weapon'
  } else if (item.kind === 'vehicle' && slugHasWord(item, 'exosuit')) {
    family = 'exosuit'
  } else if (item.kind === 'emplacement' && slugHasWord(item, 'mine')) {
    family = 'mines'
  }
  return `${PERMIT_TAG[item.permitType]}/${family}`
}

// Fim da primeira sequência do subgrupo; subgrupo inédito vai pro fim da cor.
function insertionPoint(groups, list, group, permit) {
  const start = groups.indexOf(group)
  if (start >= 0) {
    let end = start
    while (end + 1 < groups.length && groups[end + 1] === group) end++
    return end + 1
  }
  for (let i = list.length - 1; i >= 0; i--) if (list[i].tag[0] === permit) return i + 1
  return list.length
}

// Só um veículo e só um exo por loadout. Vale para entrada nova; nas antigas a
// curadoria é manual (a Bastion, por exemplo, não divide slot com os FRV).
function extraTags(item) {
  if (item.kind !== 'vehicle') return []
  return slugHasWord(item, 'exosuit') ? ['Mecha'] : ['Vehicle']
}

const codexOf = (item) => item.code.map(step => DIR[step])

// O jogo não tem codex repetido nem um que comece com outro (o menor dispararia
// no meio do maior). A mesma regra trava as trocas de codex no app.
function codexClashes(list) {
  const all = [...list.map(s => [s.nome, s.codex]), ...Object.entries(SUPPORT_CODEX)]
  const clashes = []
  for (const [a, x] of all) {
    for (const [b, y] of all) {
      if (a !== b && x.length <= y.length && x.every((d, i) => y[i] === d)) clashes.push(`${a} / ${b}`)
    }
  }
  return clashes
}

async function fetchJson(url) {
  const res = await fetch(url, { headers: HEADERS })
  if (!res.ok) throw new Error(`HTTP ${res.status} em ${url}`)
  return res.json()
}

async function downloadIcon(item, destAbs) {
  const url = `${API}${item.image.url}`
  const res = await fetch(url, { headers: HEADERS })
  if (!res.ok) throw new Error(`HTTP ${res.status} em ${url}`)
  fs.writeFileSync(destAbs, Buffer.from(await res.arrayBuffer()))
}

async function main() {
  console.log('Lendo estratagemas da API...')
  const { meta, data } = await fetchJson(`${API}/v1/stratagems.json`)
  // Anunciado (`upcoming`) ainda não está no jogo: entra quando lançar.
  const api = data.filter(s =>
    !s.upcoming && s.availability === 'loadout' && PERMIT_TAG[s.permitType] &&
    s.code?.length && s.code.every(d => DIR[d]))

  if (api.length < MIN_LOADOUT) {
    console.error(`✖ Só ${api.length} estratagemas de loadout. A API mudou de formato? Abortando sem escrever.`)
    process.exit(1)
  }
  console.log(`  dados de ${meta?.dataVersion ?? '?'}`)

  const current = JSON.parse(fs.readFileSync(OUT_JSON, 'utf8'))
  const semSlug = current.filter(s => !s.slug)
  if (semSlug.length) {
    console.error(`✖ Entradas sem slug: ${semSlug.map(s => s.nome).join(', ')}. Preencha antes de sincronizar.`)
    process.exit(1)
  }

  const bySlug = new Map(api.map(s => [s.id, s]))
  const result = []
  const groups = []
  const codexMudou = []
  const nomeDiferente = []
  const sumiram = []

  // O que já existe: ordem, nome, tags e ícone curados ficam; o codex segue a API.
  for (const strat of current) {
    const item = bySlug.get(strat.slug)
    if (!item) {
      sumiram.push(strat.nome)
      result.push(strat)
      groups.push(null)
      continue
    }
    const codex = codexOf(item)
    if (codex.join(',') !== strat.codex.join(',')) {
      codexMudou.push(`${strat.nome}: ${strat.codex.join(' ')} -> ${codex.join(' ')}`)
    }
    if (item.name !== strat.nome) nomeDiferente.push(`${strat.nome} (API: ${item.name})`)
    result.push({ ...strat, codex })
    groups.push(groupOf(item))
  }

  const known = new Set(current.map(s => s.slug))
  const ids = new Set(current.map(s => s.id))
  const novos = []
  const semIcone = []
  for (const item of api) {
    if (known.has(item.id)) continue
    // A wiki ainda não tem a arte (a API manda `image: null`); o app também
    // espera por ela.
    if (!item.image?.url) {
      semIcone.push(item.name)
      continue
    }
    const id = stableId(item.id)
    if (ids.has(id)) {
      console.error(`✖ Id ${id} de ${item.id} já está em uso. Abortando sem escrever.`)
      process.exit(1)
    }
    ids.add(id)
    const permit = PERMIT_TAG[item.permitType]
    const group = groupOf(item)
    const at = insertionPoint(groups, result, group, permit)
    result.splice(at, 0, {
      id,
      slug: item.id,
      nome: item.name,
      imagem: `stratagems/${item.id}.webp`,
      tag: [permit, ...extraTags(item)],
      codex: codexOf(item),
    })
    groups.splice(at, 0, group)
    novos.push(item)
  }

  const clashes = codexClashes(result)
  if (clashes.length) {
    console.error(`✖ Codex em conflito (dado errado na API?): ${clashes.join(', ')}. Abortando sem escrever.`)
    process.exit(1)
  }

  console.log(`Estratagemas: ${current.length} -> ${result.length}`)
  if (codexMudou.length) console.log('  ⚠ Codex mudou (confira no jogo):\n    ' + codexMudou.join('\n    '))
  if (sumiram.length) console.log('  ⚠ Fora da API (mantidos):', sumiram.join(', '))
  if (nomeDiferente.length) console.log('  Nome diferente na API (mantido o daqui):', nomeDiferente.join(', '))
  if (semIcone.length) console.log('  ⚠ Novos sem ícone na API (fora até ter):', semIcone.join(', '))

  if (novos.length) {
    console.log('Baixando ícones dos novos...')
    fs.mkdirSync(ICON_DIR, { recursive: true })
    for (const item of novos) {
      await downloadIcon(item, path.join(ICON_DIR, `${item.id}.webp`))
    }
    console.log('  Novos (entraram no fim do subgrupo; mova para a posição do jogo):')
    for (const item of novos) {
      const at = result.findIndex(s => s.slug === item.id)
      console.log(`    #${at} ${item.name} (depois de ${result[at - 1]?.nome ?? 'nada'})`)
    }
  }

  fs.writeFileSync(OUT_JSON, JSON.stringify(result, null, 2) + '\n', 'utf8')
  console.log(`✔ ${OUT_JSON} escrito.`)
}

main().catch(e => { console.error('✖ Falha no sync:', e); process.exit(1) })
