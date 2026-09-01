import { useState, useEffect, useCallback, useMemo, memo } from 'react'
import StratagemCard from './StratagemCard'
import stratagemsData from '../data/stratagems.json'
import equipmentData from '../data/equipment.json'
import statsMap from '../data/statsMap.json'
import { EQUIPMENT_SLOTS, hasExclusiveConflict, normalizeText } from '../lib/build'

// Este arquivo é carregado sob demanda (React.lazy no App): equipment.json e
// statsMap.json somam ~100 KB e antes eram parseados no boot das DUAS janelas,
// mesmo pra quem só usa a aba de macro.

const BuildItemCard = memo(function BuildItemCard({ label, name, image, subtitle, description, badge, locked, onToggleLock, lockTitle }) {
  return (
    <div className={`relative rounded-2xl border-2 overflow-hidden bg-slate-900/40 ${locked ? 'border-yellow-500/60 shadow-[0_0_20px_rgba(234,179,8,0.15)]' : 'border-slate-800/50'}`}>
      <button
        onClick={onToggleLock}
        title={lockTitle}
        className={`absolute top-1.5 right-1.5 z-20 w-7 h-7 rounded-lg flex items-center justify-center text-[12px] border ${locked
          ? 'bg-yellow-500 border-yellow-600 text-slate-950'
          : 'bg-slate-950/70 border-slate-800 text-slate-500 hover:text-white hover:border-slate-600'}`}
      >
        {locked ? '🔒' : '🔓'}
      </button>
      <div className="absolute top-0 inset-x-0 p-2 z-10 bg-gradient-to-b from-slate-950/90 to-transparent pointer-events-none flex items-center gap-1.5">
        <span className="text-[8px] font-black uppercase tracking-widest text-cyan-500">{label}</span>
        {badge && (
          <span className="text-[7px] font-black uppercase tracking-widest bg-yellow-500 text-slate-950 px-1.5 py-0.5 rounded">{badge}</span>
        )}
      </div>
      <div className="aspect-square flex items-center justify-center p-4 pt-8">
        {image
          ? <img src={image} alt={name} loading="lazy" decoding="async" className="max-w-full max-h-full object-contain drop-shadow-lg" />
          : <span className="text-4xl text-slate-700">▣</span>}
      </div>
      <div className="px-3 pb-3 text-center min-h-[52px]">
        <p className="text-[10px] font-black uppercase tracking-tight text-slate-100 leading-tight">{name}</p>
        {subtitle && <p className="text-[8px] font-bold text-cyan-500/80 uppercase mt-1 leading-tight">{subtitle}</p>}
        {description && <p className="text-[8px] text-slate-500 mt-1 leading-snug">{description}</p>}
      </div>
    </div>
  )
})

function BuildTab({ t, settings, onSettingChange, slots, updateSlots, loadouts, persistLoadouts, isOverlay }) {
  const [loadoutName, setLoadoutName] = useState('')
  const [randomBuild, setRandomBuild] = useState(null)
  const [buildLocks, setBuildLocks] = useState({})
  const [buildSubTab, setBuildSubTab] = useState('meta')
  const [customSlot, setCustomSlot] = useState(0)
  const [buildSearch, setBuildSearch] = useState('')
  const [metaFaction, setMetaFaction] = useState('terminid')
  const [metaDifficulty, setMetaDifficulty] = useState(0)
  const [metaStats, setMetaStats] = useState(null)
  const [metaLoading, setMetaLoading] = useState(false)
  const [metaError, setMetaError] = useState(false)

  const stratById = useMemo(() => {
    const m = {}
    stratagemsData.forEach(s => { m[s.id] = s })
    return m
  }, [])

  const equipById = useMemo(() => {
    const m = {}
    EQUIPMENT_SLOTS.forEach(c => (equipmentData[c] || []).forEach(i => { m[i.id] = i }))
    return m
  }, [])

  // Metadados da wiki (tipo do estratagema + se ocupa mochila) casados por nome normalizado
  const stratMeta = useMemo(() => {
    const infos = equipmentData.stratagemInfo || []
    const norm = (s) => s.toLowerCase().normalize('NFD').replace(/[^a-z0-9]/g, '')
    // Casamento por substring aceitava nome curto dentro de nome longo e podia casar o
    // item errado (o que contamina as regras de apoio/mochila/sentinela da build
    // balanceada). Agora: exato primeiro, e substring só a partir de 6 caracteres,
    // ficando com o candidato mais longo — o mais específico.
    const MIN_PARTIAL = 6
    const map = {}
    stratagemsData.forEach(s => {
      const n = norm(s.nome)
      let info = infos.find(i => norm(i.nome) === n)
      if (!info) {
        let best = null
        for (const i of infos) {
          const w = norm(i.nome)
          const shorter = Math.min(w.length, n.length)
          if (shorter < MIN_PARTIAL) continue
          if (!(n.includes(w) || w.includes(n))) continue
          if (!best || w.length > norm(best.nome).length) best = i
        }
        info = best
      }
      if (!info) {
        // Fallback pela designação (ex.: "AX/ARC-3") — a wiki às vezes omite apelidos como "Guard Dog"
        const des = norm(s.nome.split(' ')[0])
        if (des.length >= 4) info = infos.find(i => norm(i.nome).includes(des))
      }
      map[s.id] = info || null
    })
    return map
  }, [])

  const isSupportStrat = (s) => stratMeta[s.id]?.type === 'Support Weapon'
  const isBackpackStrat = (s) => !!stratMeta[s.id]?.backpack
  const isSentryStrat = (s) => stratMeta[s.id]?.type === 'Sentry'

  const passiveByName = useMemo(() => {
    const map = {}
    equipmentData.passives.forEach(p => { map[p.nome] = p })
    return map
  }, [])

  // Salva a build exibida (estratagemas + equipamento) como build nomeada
  const handleSaveBuild = () => {
    if (!randomBuild?.stratagems?.some(Boolean)) return
    const name = loadoutName.trim() || `Build ${loadouts.length + 1}`
    const slotIds = randomBuild.stratagems.map(s => s?.id ?? null)
    const equip = {}
    EQUIPMENT_SLOTS.forEach(slot => { if (randomBuild[slot]) equip[slot] = randomBuild[slot].id })
    const existing = loadouts.find(l => l.name.toLowerCase() === name.toLowerCase())
    if (existing) {
      persistLoadouts(loadouts.map(l => l.id === existing.id ? { ...l, slotIds, equip } : l))
    } else {
      persistLoadouts([...loadouts, { id: Date.now().toString(), name, slotIds, equip }])
    }
    setLoadoutName('')
  }

  // Aplica build salva: estratagemas vão pros slots de macro e a build inteira volta pra tela
  const handleApplyLoadout = (loadout) => {
    const resolved = loadout.slotIds.map(sid =>
      sid != null ? (stratById[sid] || null) : null
    )
    resolved.forEach((s, i) => {
      if (s && hasExclusiveConflict(s, resolved.slice(0, i), i)) resolved[i] = null
    })
    updateSlots(resolved)
    const build = { stratagems: resolved }
    EQUIPMENT_SLOTS.forEach(slot => {
      build[slot] = loadout.equip?.[slot] ? (equipById[loadout.equip[slot]] || null) : null
    })
    // Builds antigas (salvas antes do equipamento existir) não têm `equip` e mostram
    // só os estratagemas; builds novas (inclusive personalizadas sem equipamento)
    // sobrescrevem a tela inteira
    if (loadout.equip) setRandomBuild(build)
    else if (randomBuild) setRandomBuild({ ...randomBuild, stratagems: resolved })
    else setRandomBuild(build)
  }

  const handleDeleteLoadout = (id) => {
    persistLoadouts(loadouts.filter(l => l.id !== id))
  }

  // Sets de armadura: capacete do mesmo set (nome idêntico na wiki) e capa
  // da mesma warbond da armadura; set sem capa correspondente → mantém a aleatória
  const applySetMatching = (build) => {
    if (!(settings.buildMatchSet ?? true) || !build.armor) return
    if (!buildLocks.helmet) {
      const setHelmet = equipmentData.helmet.find(h => h.nome === build.armor.nome)
      if (setHelmet) build.helmet = setHelmet
    }
    if (!buildLocks.cape && build.armor.warbond) {
      const setCapes = equipmentData.cape.filter(c => c.warbond && c.warbond === build.armor.warbond)
      if (setCapes.length) build.cape = setCapes[Math.floor(Math.random() * setCapes.length)]
    }
  }

  const generateFullBuild = () => {
    const prev = randomBuild || {}
    const balanced = settings.buildBalanced ?? false
    const maxOneSentry = settings.buildMaxOneSentry ?? false

    // Estratagemas: mantém os travados, sorteia o resto respeitando as regras ativas
    const strats = Array.from({ length: 4 }, (_, i) =>
      (buildLocks[`strat${i}`] && prev.stratagems?.[i]) ? prev.stratagems[i] : null
    )
    const equipped = () => strats.filter(Boolean)
    const canAdd = (s) => {
      if (equipped().some(x => x.id === s.id)) return false
      if (hasExclusiveConflict(s, equipped(), -1)) return false
      if (maxOneSentry && isSentryStrat(s) && equipped().some(isSentryStrat)) return false
      if (balanced && isSupportStrat(s) && equipped().some(isSupportStrat)) return false
      if (balanced && isBackpackStrat(s) && equipped().some(isBackpackStrat)) return false
      return true
    }
    const pool = [...stratagemsData].sort(() => Math.random() - 0.5)
    const placeInEmptySlot = (predicate) => {
      const idx = strats.findIndex(x => !x)
      if (idx === -1) return
      const s = pool.find(c => predicate(c) && canAdd(c))
      if (s) strats[idx] = s
    }
    if (balanced) {
      // Garante 1 arma de apoio e 1 item de mochila (apoio que vem com mochila conta pros dois)
      if (!equipped().some(isSupportStrat)) placeInEmptySlot(isSupportStrat)
      if (!equipped().some(isBackpackStrat)) placeInEmptySlot(isBackpackStrat)
    }
    for (let i = 0; i < 4; i++) {
      if (strats[i]) continue
      const s = pool.find(c => canAdd(c))
      if (s) strats[i] = s
    }
    const build = { stratagems: strats }
    EQUIPMENT_SLOTS.forEach(slot => {
      const items = equipmentData[slot] || []
      build[slot] = (buildLocks[slot] && prev[slot])
        ? prev[slot]
        : items[Math.floor(Math.random() * items.length)] || null
    })
    applySetMatching(build)
    setRandomBuild(build)
  }

  const toggleBuildLock = (key) => setBuildLocks(prev => ({ ...prev, [key]: !prev[key] }))

  const applyBuildStratagems = () => {
    if (randomBuild?.stratagems) updateSlots([...randomBuild.stratagems])
  }

  // ---- Build personalizada (montada na mão) ----

  const customStrats = randomBuild?.stratagems || [null, null, null, null]

  const patchBuild = (patch) => setRandomBuild(prev => ({
    stratagems: [null, null, null, null],
    ...prev,
    ...patch
  }))

  const isCustomCardDisabled = useCallback((strat) => {
    const list = randomBuild?.stratagems || [null, null, null, null]
    if (list[customSlot]?.id === strat.id) return false
    if (list.some(s => s && s.id === strat.id)) return true
    return hasExclusiveConflict(strat, list, customSlot)
  }, [randomBuild, customSlot])

  const handleCustomAssign = useCallback((strat) => {
    setRandomBuild(prev => {
      const base = prev || {}
      const list = [...(base.stratagems || [null, null, null, null])]
      // Clicar no que já está no slot ativo remove
      if (list[customSlot]?.id === strat.id) {
        list[customSlot] = null
        return { ...base, stratagems: list }
      }
      if (list.some((s, i) => i !== customSlot && s?.id === strat.id)) return base
      if (hasExclusiveConflict(strat, list, customSlot)) return base
      list[customSlot] = strat
      return { ...base, stratagems: list }
    })
    setCustomSlot(s => (s < 3 ? s + 1 : s))
  }, [customSlot])

  const handleCustomClearSlot = (i) => {
    const list = [...customStrats]
    list[i] = null
    patchBuild({ stratagems: list })
  }

  // Traz o que já está nos slots do macro pra edição
  const handleCustomImportSlots = () => patchBuild({ stratagems: [...slots] })

  const handleCustomReset = () => {
    setRandomBuild(null)
    setCustomSlot(0)
    setBuildSearch('')
  }

  const handleCustomEquipChange = (slot, id) => patchBuild({ [slot]: id ? (equipById[id] || null) : null })

  const customStratagemList = useMemo(() => {
    const order = ['Offensive', 'Supply', 'Defensive']
    const q = normalizeText(buildSearch.trim())
    return stratagemsData
      .filter(s => !q || normalizeText(s.nome).includes(q))
      .sort((a, b) => {
        const ia = order.indexOf(a.tag?.[0]), ib = order.indexOf(b.tag?.[0])
        if (ia !== ib) return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib)
        return a.nome.localeCompare(b.nome)
      })
  }, [buildSearch])

  // ---- Meta por Facção (dados do helldive.live via main process, cache 6h) ----

  const loadMetaStats = useCallback(async (faction, difficulty) => {
    const cacheKey = `${faction}|${difficulty}`
    try {
      const cache = JSON.parse(localStorage.getItem('helldivers-meta-cache') || '{}')
      const hit = cache[cacheKey]
      if (hit && Date.now() - hit.at < 6 * 3600 * 1000) {
        setMetaStats(hit.data)
        setMetaError(false)
        return
      }
    } catch (e) { console.error(e) }
    setMetaLoading(true)
    setMetaError(false)
    const res = await window.api?.fetchMetaStats?.(faction, difficulty)
    setMetaLoading(false)
    if (res?.ok) {
      setMetaStats(res.data)
      try {
        const cache = JSON.parse(localStorage.getItem('helldivers-meta-cache') || '{}')
        cache[cacheKey] = { at: Date.now(), data: res.data }
        localStorage.setItem('helldivers-meta-cache', JSON.stringify(cache))
      } catch (e) { console.error(e) }
    } else {
      setMetaStats(null)
      setMetaError(true)
    }
  }, [])

  useEffect(() => {
    loadMetaStats(metaFaction, metaDifficulty)
  }, [metaFaction, metaDifficulty, loadMetaStats])

  // Estatísticas cruas → listas ordenadas por pick rate, já resolvidas pros nossos itens
  const metaLists = useMemo(() => {
    if (!metaStats) return null
    const strat = Object.entries(metaStats.strategem?.items || {})
      .map(([slug, st]) => ({ item: stratById[statsMap.strategem[slug]], st }))
      .filter(x => x.item)
      .sort((a, b) => b.st.loadouts_percentage - a.st.loadouts_percentage)
    const weap = { primary: [], secondary: [], grenade: [] }
    Object.entries(metaStats.weapons?.items || {}).forEach(([slug, st]) => {
      const m = statsMap.weapons[slug]
      if (m && equipById[m.id]) weap[m.cat].push({ item: equipById[m.id], st })
    })
    Object.values(weap).forEach(l => l.sort((a, b) => b.st.loadouts_percentage - a.st.loadouts_percentage))
    const passives = Object.entries(metaStats.armor?.items || {})
      .map(([key, st]) => ({ nome: statsMap.armor[key], st }))
      .filter(x => x.nome)
      .sort((a, b) => b.st.loadouts_percentage - a.st.loadouts_percentage)
    return { strat, weap, passives, games: metaStats.strategem?.total?.games || 0 }
  }, [metaStats, stratById, equipById])

  // Sorteio ponderado por pick rate; entradas no formato { item|nome, st }
  const weightedFrom = (list) => {
    const total = list.reduce((a, x) => a + x.st.loadouts_percentage, 0)
    if (total <= 0) return list[0] || null
    let r = Math.random() * total
    for (const x of list) {
      r -= x.st.loadouts_percentage
      if (r <= 0) return x
    }
    return list[list.length - 1] || null
  }

  const generateMetaBuild = () => {
    if (!metaLists) return
    const prev = randomBuild || {}
    const balanced = settings.buildBalanced ?? false
    const maxOneSentry = settings.buildMaxOneSentry ?? false

    const strats = Array.from({ length: 4 }, (_, i) =>
      (buildLocks[`strat${i}`] && prev.stratagems?.[i]) ? prev.stratagems[i] : null
    )
    const equipped = () => strats.filter(Boolean)
    const canAdd = (s) => {
      if (equipped().some(x => x.id === s.id)) return false
      if (hasExclusiveConflict(s, equipped(), -1)) return false
      if (maxOneSentry && isSentryStrat(s) && equipped().some(isSentryStrat)) return false
      if (balanced && isSupportStrat(s) && equipped().some(isSupportStrat)) return false
      if (balanced && isBackpackStrat(s) && equipped().some(isBackpackStrat)) return false
      return true
    }
    // Só os itens do TOP mostrado na tela entram na build meta (prioridade pro topo via peso).
    // Fallback: se as regras ativas exigirem algo fora do top (ex.: balanceado precisa de
    // mochila e o top 10 não tem), pega o MAIS USADO da lista completa que atenda.
    const TOP_STRATS = 10
    const pickWeighted = (predicate) => {
      const top = metaLists.strat.slice(0, TOP_STRATS)
      let pool = top.filter(x => (!predicate || predicate(x.item)) && canAdd(x.item))
      if (!pool.length) {
        pool = metaLists.strat
          .filter(x => (!predicate || predicate(x.item)) && canAdd(x.item))
          .slice(0, 3)
      }
      return pool.length ? weightedFrom(pool)?.item : null
    }
    const placeInEmptySlot = (predicate) => {
      const idx = strats.findIndex(x => !x)
      if (idx === -1) return
      const s = pickWeighted(predicate)
      if (s) strats[idx] = s
    }
    if (balanced) {
      if (!equipped().some(isSupportStrat)) placeInEmptySlot(isSupportStrat)
      if (!equipped().some(isBackpackStrat)) placeInEmptySlot(isBackpackStrat)
    }
    for (let i = 0; i < 4; i++) {
      if (strats[i]) continue
      const s = pickWeighted(null)
      if (s) strats[i] = s
    }

    const build = { stratagems: strats }
    // Armas: só o TOP 3 exibido de cada categoria (peso maior pro nº 1)
    ;['primary', 'secondary', 'grenade'].forEach(cat => {
      if (buildLocks[cat] && prev[cat]) { build[cat] = prev[cat]; return }
      const list = metaLists.weap[cat].slice(0, 3)
      build[cat] = list.length
        ? weightedFrom(list)?.item
        : equipmentData[cat][Math.floor(Math.random() * equipmentData[cat].length)]
    })
    // Armadura: só armaduras que tenham uma das TOP 5 passivas exibidas (peso pelo pick rate)
    if (buildLocks.armor && prev.armor) {
      build.armor = prev.armor
    } else {
      const topPassives = metaLists.passives.slice(0, 5)
      const p = topPassives.length ? weightedFrom(topPassives) : null
      const candidates = p ? equipmentData.armor.filter(a => a.passive === p.nome) : []
      build.armor = candidates.length
        ? candidates[Math.floor(Math.random() * candidates.length)]
        : equipmentData.armor[Math.floor(Math.random() * equipmentData.armor.length)]
    }
    ;['helmet', 'cape', 'booster'].forEach(slot => {
      build[slot] = (buildLocks[slot] && prev[slot])
        ? prev[slot]
        : equipmentData[slot][Math.floor(Math.random() * equipmentData[slot].length)]
    })
    applySetMatching(build)
    setRandomBuild(build)
  }

  // Loadout cujos ids batem com os slots atuais (para destacar o chip ativo)
  const activeLoadoutId = useMemo(() => {
    const current = slots.map(s => s?.id ?? null)
    return loadouts.find(l =>
      l.slotIds.length === current.length && l.slotIds.every((sid, i) => sid === current[i])
    )?.id ?? null
  }, [loadouts, slots])

  return (
    <div className="max-w-5xl w-full mx-auto px-6 space-y-6 pt-6 pb-24">
      {/* SUB-ABAS: META | ALEATÓRIA | PERSONALIZADA */}
      <div className="flex justify-center">
        <div className="flex bg-slate-950/60 border border-white/5 rounded-2xl p-1 gap-1">
          {['meta', 'random', 'custom'].map((sub) => (
            <button
              key={sub}
              onClick={() => setBuildSubTab(sub)}
              className={`py-2.5 px-8 rounded-xl text-[11px] font-black uppercase tracking-widest ${buildSubTab === sub
                ? 'bg-yellow-500 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]'
                : 'text-slate-500 hover:text-white'
                }`}
            >
              {sub === 'meta' ? t.build.subMeta : sub === 'random' ? t.build.subRandom : t.build.subCustom}
            </button>
          ))}
        </div>
      </div>

      {/* OPÇÕES DA BUILD (só valem para os sorteios) */}
      {buildSubTab !== 'custom' && (
      <div className="hd-card p-4 space-y-2">
        {[
          { key: 'buildMatchSet', def: true, title: t.build.matchSet, on: t.build.matchSetOn, off: t.build.matchSetOff },
          { key: 'buildBalanced', def: false, title: t.build.balanced, on: t.build.balancedOn, off: t.build.balancedOff },
          { key: 'buildMaxOneSentry', def: false, title: t.build.maxSentry, on: t.build.maxSentryOn, off: t.build.maxSentryOff },
        ].map((opt) => {
          const enabled = settings[opt.key] ?? opt.def
          return (
            <button
              key={opt.key}
              onClick={() => onSettingChange(opt.key, !enabled)}
              className={`w-full p-4 rounded-xl border-2 flex items-center justify-between group ${enabled
                ? 'bg-yellow-500/5 border-yellow-500/50'
                : 'bg-slate-950/40 border-slate-800 hover:border-yellow-500/30'
              }`}
            >
              <div className="flex flex-col items-start gap-1">
                <span className={`text-[10px] font-black uppercase tracking-widest ${enabled ? 'text-yellow-500' : 'text-slate-400'}`}>
                  {opt.title}
                </span>
                <span className="text-[9px] text-slate-500 uppercase text-left leading-tight">
                  {enabled ? opt.on : opt.off}
                </span>
              </div>
              <div className={`w-10 h-5 rounded-full border-2 flex items-center px-1 shrink-0 ${enabled ? 'border-yellow-500 bg-yellow-500/20' : 'border-slate-700 bg-slate-900'}`}>
                <div className={`w-2 h-2 rounded-full ${enabled ? 'translate-x-5 bg-yellow-500 shadow-[0_0_8px_rgba(234,179,8,0.8)]' : 'bg-slate-600'}`}></div>
              </div>
            </button>
          )
        })}
      </div>
      )}

      {/* SUB-ABA PERSONALIZADA */}
      {buildSubTab === 'custom' && (
        <div className="hd-card p-5 border-l-4 border-l-slate-700 hover:border-l-cyan-500/50 space-y-4">
          <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400">
            <div className="w-2 h-2 rounded-full border bg-cyan-500 shadow-[0_0_12px_rgba(34,211,238,0.8)] border-cyan-400/50"></div>
            {t.build.customTitle}
            <div className="ml-auto flex gap-2">
              <button
                onClick={handleCustomImportSlots}
                className="py-2 px-4 rounded-xl text-[9px] font-black uppercase tracking-widest border-2 bg-slate-900 border-slate-800 text-slate-300 hover:border-cyan-500/50 hover:text-white hover:bg-cyan-500/5"
              >
                {t.build.customImport}
              </button>
              <button
                onClick={handleCustomReset}
                className="py-2 px-4 rounded-xl text-[9px] font-black uppercase tracking-widest border-2 bg-slate-900 border-slate-800 text-slate-500 hover:border-red-500/50 hover:text-white hover:bg-red-500/5"
              >
                {t.build.customClear}
              </button>
            </div>
          </h2>
          <p className="text-[9px] text-slate-600 uppercase tracking-widest leading-relaxed">{t.build.customHint}</p>

          {/* Slots em edição */}
          <div className="grid grid-cols-4 gap-3">
            {customStrats.map((s, i) => (
              <button
                key={i}
                onClick={() => setCustomSlot(i)}
                className={`relative rounded-2xl border-2 p-3 flex flex-col items-center gap-2 ${customSlot === i
                  ? 'bg-cyan-500/5 border-cyan-500/60 shadow-[0_0_20px_rgba(34,211,238,0.15)]'
                  : 'bg-slate-950/40 border-slate-800 hover:border-cyan-500/30'
                  }`}
              >
                <span className={`text-[8px] font-black uppercase tracking-widest ${customSlot === i ? 'text-cyan-400' : 'text-slate-500'}`}>
                  {t.build.stratagem} {i + 1}
                </span>
                <div className="w-14 h-14 flex items-center justify-center">
                  {s
                    ? <img src={s.imagem} alt={s.nome} loading="lazy" decoding="async" className="max-w-full max-h-full object-contain" />
                    : <span className="text-2xl text-slate-700">▣</span>}
                </div>
                <span className="text-[9px] font-black uppercase tracking-tight text-slate-300 leading-tight text-center min-h-[22px]">
                  {s?.nome || '—'}
                </span>
                {s && (
                  <span
                    onClick={(e) => { e.stopPropagation(); handleCustomClearSlot(i) }}
                    title={t.macro.clearSlot}
                    className="absolute -top-1.5 -right-1.5 w-4 h-4 bg-red-600 hover:bg-red-500 text-white rounded-full flex items-center justify-center text-[9px] font-bold leading-none"
                  >
                    ×
                  </span>
                )}
              </button>
            ))}
          </div>

          {/* Busca + grade de estratagemas */}
          <div className="relative">
            <span className="absolute left-4 top-1/2 -translate-y-1/2 text-slate-600 text-sm pointer-events-none">⌕</span>
            <input
              type="text"
              value={buildSearch}
              onChange={(e) => setBuildSearch(e.target.value)}
              placeholder={t.macro.searchPlaceholder}
              className="w-full py-3 pl-10 pr-10 rounded-xl text-[11px] font-bold uppercase tracking-wider bg-slate-950/60 border-2 border-slate-800 text-slate-200 placeholder:text-slate-600 focus:border-cyan-500/50 focus:outline-none"
            />
            {buildSearch && (
              <button
                onClick={() => setBuildSearch('')}
                className="absolute right-3 top-1/2 -translate-y-1/2 w-5 h-5 flex items-center justify-center text-slate-500 hover:text-white text-xs font-bold"
              >
                ×
              </button>
            )}
          </div>

          {customStratagemList.length === 0 ? (
            <p className="text-center text-[10px] text-slate-600 uppercase tracking-widest py-8">
              {t.macro.searchNoResults} “{buildSearch}”
            </p>
          ) : (
            <div className="grid grid-cols-5 gap-3 max-h-[420px] overflow-y-auto scrollbar-hd pr-1">
              {customStratagemList.map((strat) => (
                <StratagemCard
                  key={strat.id}
                  strat={strat}
                  tag={strat.tag?.[0]}
                  isInActiveSlot={customStrats[customSlot]?.id === strat.id}
                  disabled={isCustomCardDisabled(strat)}
                  clearLabel={t.macro.clearSlot}
                  onAssign={handleCustomAssign}
                />
              ))}
            </div>
          )}

          {/* Equipamento opcional */}
          <div>
            <h3 className="text-[9px] font-black uppercase tracking-widest text-slate-500 mb-2">{t.build.customEquipment}</h3>
            <div className="grid grid-cols-4 gap-2">
              {EQUIPMENT_SLOTS.map((slot) => (
                <label key={slot} className="flex flex-col gap-1">
                  <span className="text-[8px] font-black uppercase tracking-widest text-slate-600">{t.build[slot]}</span>
                  <select
                    value={randomBuild?.[slot]?.id ?? ''}
                    onChange={(e) => handleCustomEquipChange(slot, e.target.value)}
                    className="w-full py-2 px-2 rounded-xl text-[9px] font-bold uppercase bg-slate-950/60 border-2 border-slate-800 text-slate-200 focus:border-cyan-500/50 focus:outline-none"
                  >
                    <option value="">{t.build.equipNone}</option>
                    {(equipmentData[slot] || []).map((item) => (
                      <option key={item.id} value={item.id}>{item.nome}</option>
                    ))}
                  </select>
                </label>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* SUB-ABA ALEATÓRIA */}
      {buildSubTab === 'random' && (
        <div className="hd-card p-5 text-center">
          <button
            onClick={generateFullBuild}
            className="py-4 px-10 rounded-2xl text-sm font-black uppercase tracking-widest border-2 bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_0_25px_rgba(234,179,8,0.35)] hover:bg-yellow-400"
          >
            🎲 {t.build.generate}
          </button>
          <p className="text-[9px] text-slate-600 uppercase tracking-widest mt-3">{t.build.hint}</p>
        </div>
      )}

      {/* SUB-ABA META */}
      {buildSubTab === 'meta' && (
      <div className="hd-card p-5 border-l-4 border-l-slate-700 hover:border-l-red-500/50">
        <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-4">
          <div className="w-2 h-2 rounded-full border bg-red-500 shadow-[0_0_12px_rgba(239,68,68,0.8)] border-red-400/50"></div>
          {t.build.meta}
        </h2>
        <div className="grid grid-cols-3 gap-2 mb-2">
          {['terminid', 'automaton', 'illuminate'].map((f) => (
            <button
              key={f}
              onClick={() => setMetaFaction(f)}
              className={`py-3 rounded-xl text-[10px] font-black uppercase border-2 ${metaFaction === f
                ? 'bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]'
                : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5'
                }`}
            >
              {t.build[`faction_${f}`]}
            </button>
          ))}
        </div>
        <div className="grid grid-cols-5 gap-2 mb-4">
          {[0, 7, 8, 9, 10].map((d) => (
            <button
              key={d}
              onClick={() => setMetaDifficulty(d)}
              className={`py-2 rounded-xl text-[9px] font-black uppercase border-2 ${metaDifficulty === d
                ? 'bg-cyan-500/20 border-cyan-500/60 text-cyan-300'
                : 'bg-slate-900 border-slate-800 text-slate-500 hover:border-cyan-500/40 hover:text-slate-300'
                }`}
            >
              {d === 0 ? t.build.metaDifficultyAll : `D${d}`}
            </button>
          ))}
        </div>

        {metaLoading && (
          <p className="text-center text-[10px] text-cyan-500/70 uppercase tracking-widest py-6 animate-pulse">{t.build.metaLoading}</p>
        )}
        {metaError && !metaLoading && (
          <p className="text-center text-[10px] text-red-400/80 uppercase tracking-widest py-6">{t.build.metaError}</p>
        )}
        {metaLists && !metaLoading && !metaError && (
          <>
            <button
              onClick={generateMetaBuild}
              className="w-full py-4 mb-4 rounded-2xl text-sm font-black uppercase tracking-widest border-2 bg-slate-900 border-yellow-500/50 text-yellow-400 hover:bg-yellow-500 hover:text-slate-950 hover:border-yellow-600"
            >
              🏆 {t.build.metaGenerate}
            </button>
            <div className="grid grid-cols-2 gap-6">
              <div>
                <h3 className="text-[9px] font-black uppercase tracking-widest text-slate-500 mb-2">{t.build.metaTopStrats}</h3>
                <div className="space-y-1">
                  {metaLists.strat.slice(0, 10).map(({ item, st }) => (
                    <div key={item.id} className="flex items-center gap-2">
                      <img src={item.imagem} alt="" className="w-6 h-6 object-contain shrink-0" loading="lazy" />
                      <span className="flex-1 text-[9px] font-bold uppercase text-slate-300 truncate">{item.nome}</span>
                      {st.isNew && <span className="text-[7px] font-black bg-yellow-500 text-slate-950 px-1 rounded shrink-0">{t.build.metaNew}</span>}
                      <span className={`text-[8px] font-black shrink-0 w-9 text-right ${st.change > 0 ? 'text-green-400' : st.change < 0 ? 'text-red-400' : 'text-slate-600'}`}>
                        {st.change > 0 ? '▲' : st.change < 0 ? '▼' : ''}{Math.abs(st.change || 0).toFixed(1)}
                      </span>
                      <div className="w-14 h-1.5 bg-slate-800 rounded overflow-hidden shrink-0">
                        <div className="h-full bg-cyan-500 rounded" style={{ width: `${(st.loadouts_percentage / (metaLists.strat[0]?.st.loadouts_percentage || 1)) * 100}%` }}></div>
                      </div>
                      <span className="w-11 text-right text-[9px] font-black text-cyan-400 shrink-0">{st.loadouts_percentage.toFixed(1)}%</span>
                    </div>
                  ))}
                </div>
              </div>
              <div className="space-y-4">
                {['primary', 'secondary', 'grenade'].map((cat) => (
                  <div key={cat}>
                    <h3 className="text-[9px] font-black uppercase tracking-widest text-slate-500 mb-2">{t.build[cat]}</h3>
                    <div className="space-y-1">
                      {metaLists.weap[cat].slice(0, 3).map(({ item, st }) => (
                        <div key={item.id} className="flex items-center gap-2">
                          {item.imagem && <img src={item.imagem} alt="" className="w-6 h-6 object-contain shrink-0" loading="lazy" />}
                          <span className="flex-1 text-[9px] font-bold uppercase text-slate-300 truncate">{item.nome}</span>
                          <span className="text-[9px] font-black text-cyan-400 shrink-0">{st.loadouts_percentage.toFixed(1)}%</span>
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
                <div>
                  <h3 className="text-[9px] font-black uppercase tracking-widest text-slate-500 mb-2">{t.build.metaTopPassives}</h3>
                  <div className="space-y-1">
                    {metaLists.passives.slice(0, 5).map(({ nome, st }) => (
                      <div key={nome} className="flex items-center gap-2">
                        <span className="flex-1 text-[9px] font-bold uppercase text-slate-300 truncate">{nome}</span>
                        <span className="text-[9px] font-black text-cyan-400 shrink-0">{st.loadouts_percentage.toFixed(1)}%</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </div>
            <p className="text-center text-[8px] text-slate-600 uppercase tracking-widest mt-4">
              {t.build.metaCredit} · {metaLists.games.toLocaleString()} {t.build.metaGames}
            </p>
          </>
        )}
      </div>
      )}

      {/* BUILDS SALVAS */}
      <div className="hd-card p-4 border-l-4 border-l-slate-700 hover:border-l-yellow-500/50">
        <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-4">
          <span className="w-2.5 h-2.5 rounded-sm border bg-yellow-500 shadow-[0_0_12px_rgba(234,179,8,0.8)] border-yellow-400/50"></span>
          {t.build.saved}
        </h2>
        <div className="flex flex-wrap items-center gap-2">
          {loadouts.map((loadout) => (
            <div key={loadout.id} className="relative group">
              <button
                onClick={() => handleApplyLoadout(loadout)}
                className={`py-2.5 px-4 rounded-xl text-[10px] font-black uppercase border-2 ${activeLoadoutId === loadout.id
                  ? 'bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]'
                  : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5'
                  }`}
              >
                {loadout.name}
              </button>
              <button
                onClick={() => handleDeleteLoadout(loadout.id)}
                title={t.build.deleteBuild}
                className="absolute -top-1.5 -right-1.5 w-4 h-4 bg-red-600 hover:bg-red-500 text-white rounded-full items-center justify-center text-[9px] font-bold leading-none hidden group-hover:flex z-10"
              >
                ×
              </button>
            </div>
          ))}
          {loadouts.length === 0 && (
            <span className="text-[9px] text-slate-600 uppercase tracking-widest py-2">{t.build.savedEmpty}</span>
          )}
          {!isOverlay && (
            <div className="flex items-center gap-2 ml-auto">
              <input
                type="text"
                value={loadoutName}
                onChange={(e) => setLoadoutName(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Enter') handleSaveBuild() }}
                placeholder={t.build.savePlaceholder}
                maxLength={24}
                className="w-40 py-2.5 px-3 rounded-xl text-[10px] font-bold uppercase bg-slate-950/60 border-2 border-slate-800 text-slate-200 placeholder:text-slate-600 focus:border-yellow-500/50 focus:outline-none"
              />
              <button
                onClick={handleSaveBuild}
                disabled={!randomBuild?.stratagems?.some(Boolean)}
                className="py-2.5 px-4 rounded-xl text-[10px] font-black uppercase border-2 bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5 disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:border-slate-800 disabled:hover:text-slate-400 disabled:hover:bg-slate-900"
              >
                {t.build.saveBuild}
              </button>
            </div>
          )}
        </div>
      </div>

      {randomBuild && (
        <>
          <div className="hd-card p-5 border-l-4 border-l-slate-700 hover:border-l-cyan-500/50">
            <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-5">
              <div className="w-2 h-2 rounded-full border bg-cyan-500 shadow-[0_0_12px_rgba(34,211,238,0.8)] border-cyan-400/50"></div>
              {t.build.stratagems}
              <button
                onClick={applyBuildStratagems}
                className="ml-auto py-2 px-4 rounded-xl text-[9px] font-black uppercase tracking-widest border-2 bg-slate-900 border-slate-800 text-slate-300 hover:border-cyan-500/50 hover:text-white hover:bg-cyan-500/5 normal-case"
              >
                {t.build.applyStratagems}
              </button>
            </h2>
            <div className="grid grid-cols-4 gap-3">
              {randomBuild.stratagems.map((s, i) => (
                <BuildItemCard
                  key={i}
                  label={`${t.build.stratagem} ${i + 1}`}
                  name={s?.nome || '—'}
                  image={s?.imagem || null}
                  locked={!!buildLocks[`strat${i}`]}
                  onToggleLock={() => toggleBuildLock(`strat${i}`)}
                  lockTitle={buildLocks[`strat${i}`] ? t.build.unlock : t.build.lock}
                />
              ))}
            </div>
          </div>

          <div className="hd-card p-5 border-l-4 border-l-slate-700 hover:border-l-yellow-500/50">
            <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-5">
              <div className="w-2 h-2 rounded-full border bg-yellow-500 shadow-[0_0_12px_rgba(234,179,8,0.8)] border-yellow-400/50"></div>
              {t.build.equipment}
            </h2>
            <div className="grid grid-cols-4 gap-3">
              {EQUIPMENT_SLOTS.map((slot) => {
                const item = randomBuild[slot]
                if (!item) return null
                let subtitle = null
                let description = null
                let badge = null
                if (slot === 'helmet' && randomBuild.armor && item.nome === randomBuild.armor.nome) {
                  badge = t.build.setBadge
                } else if (slot === 'cape' && randomBuild.armor?.warbond && item.warbond === randomBuild.armor.warbond) {
                  badge = t.build.setBadge
                }
                if (slot === 'primary' || slot === 'secondary' || slot === 'grenade') {
                  subtitle = [item.tipo, item.dano].filter(Boolean).join(' · ')
                } else if (slot === 'armor') {
                  const pesoLabel = item.peso === 'Light' ? t.build.weightLight : item.peso === 'Heavy' ? t.build.weightHeavy : t.build.weightMedium
                  subtitle = `${pesoLabel} · ARM ${item.armor ?? '?'} · VEL ${item.speed ?? '?'} · STA ${item.stamina ?? '?'}`
                  if (item.passive) {
                    const p = passiveByName[item.passive]
                    description = p?.descricao ? `${item.passive}: ${p.descricao}` : item.passive
                  }
                } else if (slot === 'booster') {
                  description = item.descricao || null
                }
                return (
                  <BuildItemCard
                    key={slot}
                    label={t.build[slot]}
                    name={item.nome}
                    image={item.imagem || null}
                    subtitle={subtitle}
                    description={description}
                    badge={badge}
                    locked={!!buildLocks[slot]}
                    onToggleLock={() => toggleBuildLock(slot)}
                    lockTitle={buildLocks[slot] ? t.build.unlock : t.build.lock}
                  />
                )
              })}
            </div>
          </div>
        </>
      )}
    </div>
  )
}

export default BuildTab
