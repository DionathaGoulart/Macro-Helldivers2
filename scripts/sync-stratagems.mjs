// Sincroniza assets/data/stratagems.json e os ícones de assets/icons/stratagems/
// com a wiki (helldivers.wiki.gg) via API Cargo.
// Uso: npm run sync-stratagems   (Node 18+; `npm install` na pasta scripts/)
//
// Os ícones da wiki são SVG; o app usa WebP com nomes de arquivo estáveis porque
// builds salvas guardam ids, mas o nome do arquivo é o que o JSON aponta.
// Rasterizar mantendo o nome evita reescrever a lista inteira a cada sync.
//
// A rasterização é feita por resvg, não por ImageMagick. O renderer SVG interno
// do ImageMagick (o que roda quando ele é compilado sem o delegate do librsvg)
// descarta elementos com `transform="rotate(a x y) scale(...)"`, e é justamente
// assim que a wiki posiciona a carga das Eagles: a Strafing Run saía sem as
// rajadas e a Napalm Airstrike sem as bombas, só com a silhueta da aeronave.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import sharp from 'sharp'
import { Resvg } from '@resvg/resvg-js'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const API = 'https://helldivers.wiki.gg/api.php'
const FILEPATH_URL = 'https://helldivers.wiki.gg/wiki/Special:FilePath/'
const OUT_JSON = path.join(ROOT, 'assets/data/stratagems.json')
const ICON_DIR = path.join(ROOT, 'assets/icons/stratagems')
const ICON_SIZE = 256

// Permits e tipos que o jogador escolhe no loadout. Fora daqui ficam os de missão
// (Reinforce, Resupply, SEAF Artillery...), que o app já trata como apoio fixo.
const LOADOUT_PERMITS = ['Offensive', 'Supply', 'Defensive']
const NON_LOADOUT_TYPES = ['Ship', 'Objective', 'Other']

const sleep = (ms) => new Promise(r => setTimeout(r, ms))

const decodeEntities = (s) => typeof s === 'string'
  ? s.replace(/&#0?39;|&apos;/g, "'").replace(/&quot;/g, '"').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&')
  : s

async function fetchJson(params) {
  const url = `${API}?${new URLSearchParams({ format: 'json', ...params })}`
  const res = await fetch(url, { headers: { 'User-Agent': 'MacroHelldivers2-scraper/1.0' } })
  if (!res.ok) throw new Error(`HTTP ${res.status} em ${url}`)
  return res.json()
}

async function cargoAll(tables, fields) {
  const rows = []
  for (let offset = 0; ; offset += 500) {
    const json = await fetchJson({ action: 'cargoquery', limit: '500', offset: String(offset), tables, fields })
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

// "<span>[[File:Stratagem Arrow Down.svg|link=]]</span>..." -> ['DOWN', ...]
const parseCodex = (html) =>
  [...(html || '').matchAll(/Stratagem Arrow (Up|Down|Left|Right)\.svg/g)].map(m => m[1].toUpperCase())

// Tags extras controlam exclusividade de slot no app (só um veículo / só um mecha por
// build). Só valem pra entradas novas: nas que já existem a curadoria é manual, porque
// a wiki não distingue quem divide slot de quem (ex.: a Bastion não conta como veículo).
function extraTags(row) {
  if (row.stratagem_type !== 'Vehicle') return []
  return /exosuit/i.test(row.page) ? ['Mecha'] : ['Vehicle']
}

// "Autocannon Stratagem Icon Background.svg" -> "Autocannon_Stratagem_Icon.webp"
const iconFileName = (wikiFile) =>
  wikiFile.replace(/\.svg$/i, '').replace(/ Background$/, '').replace(/[^A-Za-z0-9._-]/g, '_') + '.webp'

async function fetchSvg(wikiFile) {
  const res = await fetch(`${FILEPATH_URL}${encodeURIComponent(wikiFile)}`,
    { headers: { 'User-Agent': 'MacroHelldivers2-scraper/1.0' } })
  if (res.status === 429) return { retryAfter: Number(res.headers.get('retry-after')) * 1000 }
  if (!res.ok) return { erro: `HTTP ${res.status}` }
  const buf = Buffer.from(await res.arrayBuffer())
  if (!buf.slice(0, 512).toString().includes('<svg')) return { erro: 'resposta não é SVG' }
  return { buf }
}

async function downloadIcon(wikiFile, destName) {
  const destAbs = path.join(ICON_DIR, destName)
  // Só a variante "Background" traz o fundo escuro e a moldura da cor da permissão —
  // sem ela o ícone sai com fundo branco e destoa da grade. A Cargo às vezes aponta
  // pro arquivo sem fundo (ex.: o genérico de arma de apoio), daí a tentativa dupla.
  const candidatos = /Background\.svg$/i.test(wikiFile)
    ? [wikiFile]
    : [wikiFile.replace(/\.svg$/i, ' Background.svg'), wikiFile]
  // A wiki aplica rate-limit agressivo (429): backoff exponencial e paciência
  const backoffs = [2000, 5000, 15000, 30000, 60000]
  for (let attempt = 0; attempt <= backoffs.length; attempt++) {
    try {
      let r
      for (const candidato of candidatos) {
        r = await fetchSvg(candidato)
        if (r.buf || r.retryAfter) break
      }
      if (r.retryAfter !== undefined) {
        await sleep(r.retryAfter || backoffs[Math.min(attempt, backoffs.length - 1)])
        continue
      }
      if (r.erro) throw new Error(r.erro)
      // resvg rasteriza a partir do viewBox na largura final, então a borda sai
      // nítida sem o passo de -density alto + resize que o ImageMagick exigia.
      // WebP lossless: são formas chapadas, e com perda a borda ganha franja.
      const png = new Resvg(r.buf.toString('utf8'), {
        fitTo: { mode: 'width', value: ICON_SIZE },
      }).render().asPng()
      await sharp(png)
        .resize(ICON_SIZE, ICON_SIZE, { fit: 'fill' })
        .webp({ lossless: true })
        .toFile(destAbs)
      return true
    } catch (e) {
      if (attempt === backoffs.length) {
        console.warn(`  ⚠ ícone falhou: ${wikiFile} (${e.message})`)
        return false
      }
      await sleep(backoffs[attempt])
    }
  }
  return false
}

async function main() {
  fs.mkdirSync(ICON_DIR, { recursive: true })

  console.log('Coletando estratagemas da wiki (Cargo API)...')
  const rows = await cargoAll('Stratagems', '_pageName=page,title,image,permit_type,stratagem_type,stratagem_code')

  // Uma página pode render várias linhas na Cargo; fica a que tem permit preenchido.
  // "Offense" é typo de uma entrada na wiki — normaliza pra não virar categoria fantasma.
  const byPage = new Map()
  for (const row of rows) {
    if (!row.page || !row.image) continue
    const permit = row.permit_type === 'Offense' ? 'Offensive' : row.permit_type
    const prev = byPage.get(row.page)
    if (!prev || (!prev.permit_type && permit)) byPage.set(row.page, { ...row, permit_type: permit })
  }

  const wiki = [...byPage.values()].filter(r =>
    LOADOUT_PERMITS.includes(r.permit_type) && !NON_LOADOUT_TYPES.includes(r.stratagem_type))

  if (wiki.length < 80) {
    console.error(`✖ Só ${wiki.length} estratagemas de loadout — a wiki mudou de formato? Abortando sem escrever.`)
    process.exit(1)
  }

  const current = JSON.parse(fs.readFileSync(OUT_JSON, 'utf8'))
  // O codex é único por estratagema e não muda em rename — casa melhor que o nome
  const byCodex = new Map(current.map(s => [s.codex.join(','), s]))
  let nextId = Math.max(...current.map(s => s.id)) + 1

  const novos = []
  const renomeados = []
  const result = wiki.map(row => {
    const codex = parseCodex(row.stratagem_code)
    const prev = byCodex.get(codex.join(','))
    if (prev) {
      // Nome canônico da wiki é o título da página; o app já usa esse formato
      if (prev.nome !== row.page) renomeados.push(`${prev.nome} -> ${row.page}`)
      // Permit vem da wiki; as tags de exclusividade seguem a curadoria já existente
      return { ...prev, nome: row.page, tag: [row.permit_type, ...prev.tag.slice(1)] }
    }
    novos.push(`${row.page} (${codex.join(' ')})`)
    return {
      id: nextId++,
      nome: row.page,
      imagem: `stratagems/${iconFileName(row.image)}`,
      tag: [row.permit_type, ...extraTags(row)],
      codex,
    }
  })

  // Mantém a ordem atual (curada) e joga os novos no fim
  const ordem = new Map(current.map((s, i) => [s.id, i]))
  result.sort((a, b) => (ordem.get(a.id) ?? Infinity) - (ordem.get(b.id) ?? Infinity) || a.id - b.id)

  console.log(`Estratagemas: ${current.length} -> ${result.length}`)
  if (novos.length) console.log('  Novos:', novos.join(', '))
  if (renomeados.length) console.log('  Renomeados:', renomeados.join(', '))

  console.log('Baixando ícones (SVG da wiki -> WebP)...')
  const fileByPage = new Map(wiki.map(r => [r.page, r.image]))
  let falhas = 0
  for (let i = 0; i < result.length; i++) {
    const strat = result[i]
    const wikiFile = fileByPage.get(strat.nome)
    if (wikiFile && !await downloadIcon(wikiFile, path.basename(strat.imagem))) falhas++
    await sleep(250)
    process.stdout.write(`\r  ${i + 1}/${result.length} ícones`)
  }
  process.stdout.write('\n')

  fs.writeFileSync(OUT_JSON, JSON.stringify(result, null, 2) + '\n', 'utf8')
  console.log(`✔ ${OUT_JSON} escrito. Ícones que falharam: ${falhas}`)
}

main().catch(e => { console.error('✖ Falha no sync:', e); process.exit(1) })
