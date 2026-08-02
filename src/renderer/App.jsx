import { useState, useEffect, useCallback, useMemo, memo } from 'react'
import Slot, { ArrowIcon } from './components/Slot'
import stratagemsData from './data/stratagems.json'
import equipmentData from './data/equipment.json'
import statsMap from './data/statsMap.json'
import { translations } from './data/translations'
import pkg from '../../package.json'
import { SUPPORT_STRATS } from '../shared/constants.js'

const SPRINT_MODIFIERS = ['Shift', 'Alt', 'Ctrl', 'None']
const MACRO_SPEEDS = ['normal', 'fast', 'turbo']

// Tags que permitem apenas 1 estratagema equipado por vez (Mecha = exos, Vehicle = FRVs)
const EXCLUSIVE_TAGS = ['Mecha', 'Vehicle']

const hasExclusiveConflict = (strat, slots, activeSlot) =>
  EXCLUSIVE_TAGS.some(tag =>
    strat.tag?.includes(tag) &&
    slots.some((s, i) => i !== activeSlot && s?.tag?.includes(tag))
  )

// Mapa de teclas do browser para o formato do Electron/nut.js
const keyMap = {
  'F1': 'F1', 'F2': 'F2', 'F3': 'F3', 'F4': 'F4',
  'F5': 'F5', 'F6': 'F6', 'F7': 'F7', 'F8': 'F8',
  'F9': 'F9', 'F10': 'F10', 'F11': 'F11', 'F12': 'F12',
  'Control': 'LeftControl', 'Shift': 'LeftShift', 'Alt': 'LeftAlt',
  'ArrowUp': 'Up', 'ArrowDown': 'Down', 'ArrowLeft': 'Left', 'ArrowRight': 'Right',
  'Backspace': 'Backspace', 'Delete': 'Delete', 'Home': 'Home', 'End': 'End',
  'Insert': 'Insert', 'PageUp': 'PageUp', 'PageDown': 'PageDown',
  'Numpad0': 'Numpad0', 'Numpad1': 'Numpad1', 'Numpad2': 'Numpad2',
  'Numpad3': 'Numpad3', 'Numpad4': 'Numpad4', 'Numpad5': 'Numpad5',
  'Numpad6': 'Numpad6', 'Numpad7': 'Numpad7', 'Numpad8': 'Numpad8', 'Numpad9': 'Numpad9',
}

const TAG_HOVER_CLASSES = {
  Offensive: 'hover:border-red-500/50 hover:shadow-[0_0_30px_rgba(239,68,68,0.15)]',
  Defensive: 'hover:border-green-500/50 hover:shadow-[0_0_30px_rgba(34,197,94,0.15)]',
  Supply: 'hover:border-cyan-500/50 hover:shadow-[0_0_30px_rgba(6,182,212,0.15)]'
}

const StratagemCard = memo(function StratagemCard({ strat, tag, isInActiveSlot, disabled, clearLabel, onAssign }) {
  const hoverClasses = TAG_HOVER_CLASSES[tag] || TAG_HOVER_CLASSES.Supply

  return (
    <button
      onClick={() => !disabled && onAssign(strat)}
      title={isInActiveSlot ? clearLabel : strat.nome}
      className={`group relative aspect-square rounded-2xl border-2 overflow-hidden
        ${disabled
          ? 'bg-slate-950/50 border-slate-900 opacity-20 cursor-not-allowed'
          : isInActiveSlot
            ? 'bg-slate-900/40 border-yellow-500/60 shadow-[0_0_20px_rgba(234,179,8,0.15)]'
            : `bg-slate-900/40 border-slate-800/50 ${hoverClasses}`}`}
    >
      {/* Stratagem Icon - Full bleed */}
      <img
        src={strat.imagem}
        alt={strat.nome}
        decoding="async"
        loading="lazy"
        className="w-full h-full object-cover opacity-70 group-hover:scale-110 group-hover:opacity-100 transition-all duration-500 transform-gpu will-change-transform"
        style={{ imageRendering: 'auto' }}
      />

      {/* HUD Overlay: Name (Top) */}
      <div className="absolute inset-x-0 top-0 bg-gradient-to-b from-slate-950 via-slate-950/70 to-transparent p-3 pb-8 flex justify-center z-10">
        <span className="text-[10px] font-black text-slate-100 uppercase tracking-tighter text-center leading-none">
          {strat.nome}
        </span>
      </div>

      {/* HUD Overlay: Codex (Bottom) */}
      <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-slate-950 via-slate-950/90 to-transparent p-4 pt-12 flex justify-center z-10">
        <div className={`flex ${strat.codex.length > 6 ? 'gap-1' : 'gap-1.5'}`}>
          {strat.codex.map((dir, i) => (
            <ArrowIcon
              key={i}
              direction={dir}
              size={strat.codex.length > 6 ? 13 : 16}
              className="text-cyan-500 drop-shadow-lg"
            />
          ))}
        </div>
      </div>
    </button>
  )
})

const EQUIPMENT_SLOTS = ['primary', 'secondary', 'grenade', 'armor', 'helmet', 'cape', 'booster']

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

function App() {
  const [activeTab, setActiveTab] = useState('macro')
  const [slots, setSlots] = useState([null, null, null, null])
  const [activeSlot, setActiveSlot] = useState(0)
  const [capturingSlot, setCapturingSlot] = useState(null)
  const [loadouts, setLoadouts] = useState([])
  const [loadoutName, setLoadoutName] = useState('')
  const [searchQuery, setSearchQuery] = useState('')
  const [backupStatus, setBackupStatus] = useState(null)
  const [randomBuild, setRandomBuild] = useState(null)
  const [buildLocks, setBuildLocks] = useState({})
  const [buildSubTab, setBuildSubTab] = useState('meta')
  const [metaFaction, setMetaFaction] = useState('terminid')
  const [metaDifficulty, setMetaDifficulty] = useState(0)
  const [metaStats, setMetaStats] = useState(null)
  const [metaLoading, setMetaLoading] = useState(false)
  const [metaError, setMetaError] = useState(false)

  const [settings, setSettings] = useState({
    shortcuts: ['F1', 'F2', 'F3', 'F4'],
    supportShortcuts: [null, null, null],
    modifierKey: 'LeftControl',
    sprintModifier: 'Shift',
    macroSpeed: 'normal',
    buildMatchSet: true,
    buildBalanced: false,
    buildMaxOneSentry: false,
    useArrows: false,
    language: 'pt',
    enableOverlay: true,
    alwaysShowSlots: false
  })
  const [updateStatus, setUpdateStatus] = useState({ status: 'idle', percent: 0 })
  const [gameFocused, setGameFocused] = useState(false)
  const [supportActiveVisual, setSupportActiveVisual] = useState(null)
  const [macroBlockedSlot, setMacroBlockedSlot] = useState(null)
  const [isBooting, setIsBooting] = useState(window.location.hash !== '#overlay')
  const isOverlay = window.location.hash === '#overlay'
  // 'hidden' | 'minimal' | 'panel' — controlado pelo main; só relevante na janela de overlay
  const [overlayState, setOverlayState] = useState('hidden')
  const [fullscreenWarning, setFullscreenWarning] = useState(false)
  const isMinimal = isOverlay && overlayState === 'minimal'
  const overlayHidden = isOverlay && overlayState === 'hidden'

  useEffect(() => {
    // Sequência de boot dura 4 segundos
    const timer = setTimeout(() => setIsBooting(false), 4500)
    return () => clearTimeout(timer)
  }, [])

  useEffect(() => {
    const savedSlots = localStorage.getItem('helldivers-macro-slots')
    if (savedSlots) {
      try {
        const parsed = JSON.parse(savedSlots)
        // Re-resolve por id: saves antigos podem ter tags/codex desatualizados
        const normalized = parsed.map(s =>
          s ? (stratagemsData.find(d => d.id === s.id) || null) : null
        )
        // Remove conflitos de exclusividade herdados de saves antigos
        normalized.forEach((s, i) => {
          if (s && hasExclusiveConflict(s, normalized.slice(0, i), i)) normalized[i] = null
        })
        setSlots(normalized)
        localStorage.setItem('helldivers-macro-slots', JSON.stringify(normalized))
        if (window.api) window.api.updateSlots(normalized)
      } catch (e) { console.error(e) }
    }

    const savedLoadouts = localStorage.getItem('helldivers-macro-loadouts')
    if (savedLoadouts) {
      try {
        setLoadouts(JSON.parse(savedLoadouts))
      } catch (e) { console.error(e) }
    }

    const savedSettings = localStorage.getItem('helldivers-macro-settings')
    if (savedSettings) {
      try {
        const parsed = JSON.parse(savedSettings)
        setSettings(prev => ({ ...prev, ...parsed }))
        if (window.api) window.api.saveSettings({ ...settings, ...parsed })
      } catch (e) { console.error(e) }
    } else {
      if (window.api) window.api.saveSettings(settings)
    }
  }, [])

  useEffect(() => {
    const timers = []
    const flash = (setter, value, ms) => {
      setter(value)
      timers.push(setTimeout(() => setter(null), ms))
    }
    const disposers = [
      window.api?.onUpdateStatus?.((info) => setUpdateStatus(info)),
      window.api?.onGameFocusChanged?.((focused) => setGameFocused(focused)),
      window.api?.onOverlayState?.((state) => setOverlayState(state)),
      window.api?.onFullscreenWarning?.((warn) => setFullscreenWarning(warn)),
      window.api?.onSyncSlots?.((newSlots) => setSlots(newSlots)),
      window.api?.onSyncLoadouts?.((newLoadouts) => setLoadouts(newLoadouts)),
      window.api?.onSyncSettings?.((newSettings) => setSettings(prev => ({ ...prev, ...newSettings }))),
      window.api?.onSupportMacroTriggered?.((index) => flash(setSupportActiveVisual, index, 500)),
      window.api?.onMacroBlocked?.((blocked) => flash(setMacroBlockedSlot, blocked, 400))
    ]

    return () => {
      disposers.forEach(dispose => dispose?.())
      timers.forEach(clearTimeout)
    }
  }, [])

  const updateSlots = useCallback((newSlots) => {
    setSlots(newSlots)
    localStorage.setItem('helldivers-macro-slots', JSON.stringify(newSlots))
    if (window.api) window.api.updateSlots(newSlots)
  }, [])

  const handleClearSlot = useCallback((index) => {
    const newSlots = [...slots]
    newSlots[index] = null
    updateSlots(newSlots)
  }, [slots, updateSlots])

  const isCardDisabled = useCallback((strat) => {
    if (slots[activeSlot]?.id === strat.id) return false
    if (slots.some(s => s && s.id === strat.id)) return true
    return hasExclusiveConflict(strat, slots, activeSlot)
  }, [slots, activeSlot])

  // useCallback preserva a identidade da função entre renders — sem isso o memo()
  // dos StratagemCard nunca corta re-render (a prop onAssign mudaria sempre)
  const handleAssignStratagem = useCallback((stratagem) => {
    // Clicar no estratagema já equipado no slot ativo desequipa
    const current = slots[activeSlot]
    if (current && current.id === stratagem.id) {
      handleClearSlot(activeSlot)
      return
    }

    const isEquipped = slots.some(s => s && s.id === stratagem.id)
    if (isEquipped) return

    if (hasExclusiveConflict(stratagem, slots, activeSlot)) return

    const newSlots = [...slots]
    newSlots[activeSlot] = stratagem
    updateSlots(newSlots)
    if (activeSlot < 3) setActiveSlot(activeSlot + 1)
  }, [slots, activeSlot, handleClearSlot, updateSlots])

  const persistLoadouts = useCallback((list) => {
    setLoadouts(list)
    localStorage.setItem('helldivers-macro-loadouts', JSON.stringify(list))
    if (window.api?.updateLoadouts) window.api.updateLoadouts(list)
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
      sid != null ? (stratagemsData.find(d => d.id === sid) || null) : null
    )
    resolved.forEach((s, i) => {
      if (s && hasExclusiveConflict(s, resolved.slice(0, i), i)) resolved[i] = null
    })
    updateSlots(resolved)
    const build = { stratagems: resolved }
    EQUIPMENT_SLOTS.forEach(slot => {
      build[slot] = loadout.equip?.[slot] ? (equipById[loadout.equip[slot]] || null) : null
    })
    // Builds antigas (salvas antes do equipamento existir) mostram só os estratagemas
    if (EQUIPMENT_SLOTS.some(slot => build[slot])) setRandomBuild(build)
    else if (randomBuild) setRandomBuild({ ...randomBuild, stratagems: resolved })
    else setRandomBuild(build)
  }

  const handleDeleteLoadout = (id) => {
    persistLoadouts(loadouts.filter(l => l.id !== id))
  }

  // Metadados da wiki (tipo do estratagema + se ocupa mochila) casados por nome normalizado
  const stratMeta = useMemo(() => {
    const infos = equipmentData.stratagemInfo || []
    const norm = (s) => s.toLowerCase().normalize('NFD').replace(/[^a-z0-9]/g, '')
    const map = {}
    stratagemsData.forEach(s => {
      const n = norm(s.nome)
      let info = infos.find(i => {
        const w = norm(i.nome)
        return w === n || n.includes(w) || w.includes(n)
      })
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

  const toggleBuildLock = (key) => setBuildLocks(prev => ({ ...prev, [key]: !prev[key] }))

  const applyBuildStratagems = () => {
    if (randomBuild?.stratagems) updateSlots([...randomBuild.stratagems])
  }

  const passiveByName = useMemo(() => {
    const map = {}
    equipmentData.passives.forEach(p => { map[p.nome] = p })
    return map
  }, [])

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
    if (activeTab === 'build') loadMetaStats(metaFaction, metaDifficulty)
  }, [activeTab, metaFaction, metaDifficulty, loadMetaStats])

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

  const flashBackupStatus = (status) => {
    setBackupStatus(status)
    setTimeout(() => setBackupStatus(null), 2500)
  }

  const handleExportData = async () => {
    const data = {
      app: 'macro-helldivers2',
      exportedAt: new Date().toISOString(),
      settings,
      loadouts,
      slotIds: slots.map(s => s?.id ?? null)
    }
    const res = await window.api?.exportData?.(data)
    if (res?.ok) flashBackupStatus('exported')
    else if (!res?.canceled) flashBackupStatus('error')
  }

  const handleImportData = async () => {
    const res = await window.api?.importData?.()
    if (!res?.ok) {
      if (!res?.canceled) flashBackupStatus('error')
      return
    }
    const d = res.data
    if (!d || d.app !== 'macro-helldivers2') {
      flashBackupStatus('error')
      return
    }
    if (d.settings && typeof d.settings === 'object') {
      const merged = { ...settings, ...d.settings }
      setSettings(merged)
      localStorage.setItem('helldivers-macro-settings', JSON.stringify(merged))
      if (window.api) window.api.saveSettings(merged)
    }
    if (Array.isArray(d.loadouts)) persistLoadouts(d.loadouts)
    if (Array.isArray(d.slotIds)) {
      const resolved = d.slotIds.slice(0, 4).map(sid =>
        sid != null ? (stratagemsData.find(x => x.id === sid) || null) : null
      )
      while (resolved.length < 4) resolved.push(null)
      resolved.forEach((s, i) => {
        if (s && hasExclusiveConflict(s, resolved.slice(0, i), i)) resolved[i] = null
      })
      updateSlots(resolved)
    }
    flashBackupStatus('imported')
  }

  // Loadout cujos ids batem com os slots atuais (para destacar o chip ativo)
  const activeLoadoutId = useMemo(() => {
    const current = slots.map(s => s?.id ?? null)
    return loadouts.find(l =>
      l.slotIds.length === current.length && l.slotIds.every((sid, i) => sid === current[i])
    )?.id ?? null
  }, [loadouts, slots])

  useEffect(() => {
    if (isOverlay) {
      document.title = "HD2_OVERLAY"
      document.body.classList.add('is-overlay')
    } else {
      document.body.classList.remove('is-overlay')
    }
  }, [isOverlay])

  const handleSettingChange = (key, value) => {
    const newSettings = { ...settings, [key]: value }
    setSettings(newSettings)
    localStorage.setItem('helldivers-macro-settings', JSON.stringify(newSettings))
    if (window.api) window.api.saveSettings(newSettings)
  }

  const handleShortcutChange = (index, value) => {
    const newShortcuts = [...settings.shortcuts]
    newShortcuts[index] = value
    handleSettingChange('shortcuts', newShortcuts)
  }

  const handleSupportShortcutChange = (index, value) => {
    const newShortcuts = [...settings.supportShortcuts]
    newShortcuts[index] = value
    handleSettingChange('supportShortcuts', newShortcuts)
  }

  const handleKeyCapture = useCallback((e) => {
    if (capturingSlot === null) return
    e.preventDefault()
    if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return
    if (e.key === 'Escape') { setCapturingSlot(null); return }

    let key = e.code
    if (key.startsWith('Key')) key = key.replace('Key', '')
    else if (key.startsWith('Digit')) key = key.replace('Digit', '')
    const mapped = keyMap[e.key] || keyMap[key] || key

    if (typeof capturingSlot === 'number') handleShortcutChange(capturingSlot, mapped)
    else if (typeof capturingSlot === 'string' && capturingSlot.startsWith('support-')) {
      const index = parseInt(capturingSlot.split('-')[1])
      handleSupportShortcutChange(index, mapped)
    }
    setCapturingSlot(null)
    window.api?.setRecordingMode?.(false)
  }, [capturingSlot, settings])

  useEffect(() => {
    if (capturingSlot !== null) {
      const keyHandler = (e) => {
        if (e.key === 'Escape') {
          e.preventDefault()
          setCapturingSlot(null)
          window.api?.setRecordingMode?.(false)
          return
        }
        handleKeyCapture(e)
      }
      window.addEventListener('keydown', keyHandler, true)
      return () => window.removeEventListener('keydown', keyHandler, true)
    }
  }, [capturingSlot, handleKeyCapture])

  const t = translations[settings.language] || translations.pt

  const stratagemsByTag = useMemo(() => {
    const grouped = {}
    stratagemsData.forEach(strat => {
      const tag = (strat.tag && strat.tag.length > 0) ? strat.tag[0] : t.macro.others
      if (!grouped[tag]) grouped[tag] = []
      grouped[tag].push(strat)
    })
    return grouped
  }, [t.macro.others])

  const sortedTags = useMemo(() => {
    return Object.keys(stratagemsByTag).sort((a, b) => {
      const order = ['Offensive', 'Supply', 'Defensive']
      const indexA = order.indexOf(a), indexB = order.indexOf(b)
      if (indexA !== -1 && indexB !== -1) return indexA - indexB
      return indexA !== -1 ? -1 : indexB !== -1 ? 1 : a.localeCompare(b)
    })
  }, [stratagemsByTag])

  // Busca sem acento e sem case: "orbital" acha "Orbital", "gatling" acha "A/G-16 Gatling"
  const normalizeText = (s) => s.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '')
  const filteredStratagemsByTag = useMemo(() => {
    const q = normalizeText(searchQuery.trim())
    if (!q) return stratagemsByTag
    const filtered = {}
    Object.entries(stratagemsByTag).forEach(([tag, strats]) => {
      const matches = strats.filter(s => normalizeText(s.nome).includes(q))
      if (matches.length > 0) filtered[tag] = matches
    })
    return filtered
  }, [stratagemsByTag, searchQuery])

  // A janela do overlay nunca é escondida pelo main (ver index.js); quando o
  // estado é 'hidden' simplesmente não renderizamos nada na janela transparente
  if (overlayHidden) return null

  return (
    <>
    {fullscreenWarning && (isOverlay || gameFocused) && (
      <div className="fixed top-4 left-1/2 -translate-x-1/2 z-[10001] max-w-xl bg-yellow-950/95 border-2 border-yellow-500/70 text-yellow-200 text-xs font-bold px-5 py-3 rounded-2xl shadow-[0_0_30px_rgba(234,179,8,0.25)] text-center leading-relaxed">
        ⚠ {t.overlay?.fullscreenWarning}
      </div>
    )}
    <div className={`h-screen flex flex-col ${
      isMinimal 
        ? 'bg-transparent text-slate-200 overflow-hidden' 
        : isOverlay
          ? 'bg-slate-950/20 backdrop-blur-[2px] text-slate-200 relative selection:bg-cyan-500/30 overflow-hidden items-center justify-center'
          : 'bg-slate-950 text-slate-200 relative selection:bg-cyan-500/30 overflow-hidden'
    }`}>
      
      {/* BOOT INTRO SEQUENCE */}
      {!isMinimal && isBooting && (
        <div className="absolute inset-0 z-[5000] bg-slate-950/95 flex flex-col items-center justify-center font-mono overflow-hidden rounded-2xl">
          {/* CRT Scanline Effect */}
          <div className="absolute inset-0 pointer-events-none bg-[linear-gradient(rgba(18,16,16,0)_50%,rgba(0,0,0,0.25)_50%),linear-gradient(90deg,rgba(255,0,0,0.06),rgba(0,255,0,0.02),rgba(0,0,255,0.06))] z-50 bg-[length:100%_4px,3px_100%] opacity-30"></div>
          
          <div className="relative z-10 w-full max-w-lg px-12 space-y-8" style={{ animation: 'fadeIn 1s ease-out 3.5s reverse forwards' }}>
            {/* Logo Part */}
            <div className="flex flex-col items-center space-y-4 opacity-0 intro-zoom-in" style={{ animationDelay: '0.3s' }}>
              <div className="relative">
                <div className="w-20 h-20 border-4 border-yellow-500 rounded-full flex items-center justify-center">
                  <span className="text-3xl font-black text-yellow-500 tracking-tighter">HD</span>
                </div>
                <div className="absolute inset-0 border-4 border-cyan-500 rounded-full animate-ping opacity-20"></div>
              </div>
              <h1 className="text-yellow-500 text-xs font-black tracking-[0.5em] uppercase">
                Macro Helldivers 2
              </h1>
            </div>

            {/* Terminal Lines */}
            <div className="space-y-2 text-[10px] uppercase tracking-widest text-cyan-500/70">
              <p className="opacity-0 intro-slide-left" style={{ animationDelay: '1s' }}>
                &gt; {t.settings.bootLink} [OK]
              </p>
              <p className="opacity-0 intro-slide-left" style={{ animationDelay: '1.5s' }}>
                <span className="text-yellow-500/70">&gt; {t.settings.bootProtocols} [OK]</span>
              </p>
              <p className="opacity-0 intro-slide-left" style={{ animationDelay: '2s' }}>
                &gt; {t.settings.bootIdentity} [VERIFIED]
              </p>
              <p className="opacity-0 intro-slide-left" style={{ animationDelay: '2.5s' }}>
                <span className="text-white">&gt; {t.settings.bootReady}</span>
              </p>
            </div>

            {/* Progress Bar (Fake) */}
            <div className="w-full h-1 bg-slate-900 rounded-full overflow-hidden opacity-0 intro-fade-in" style={{ animationDelay: '0.8s' }}>
              <div className="h-full bg-yellow-500 shadow-[0_0_15px_rgba(234,179,8,0.5)] animate-boot-progress"></div>
            </div>
          </div>

          {/* Vignette */}
          <div className="absolute inset-0 pointer-events-none shadow-[inset_0_0_150px_rgba(0,0,0,0.8)]"></div>
        </div>
      )}

      <div className={`flex flex-col ${
        !isMinimal && isBooting ? 'scale-110 blur-xl opacity-0' : 'scale-100 blur-0 opacity-100'
      } ${
        isOverlay && !isMinimal ? 'w-[820px] h-[640px] bg-slate-950/70 rounded-3xl border border-white/10 shadow-[0_0_50px_rgba(0,0,0,0.4)] overflow-hidden relative m-auto backdrop-blur-md' : 'h-full'
      }`}>
      
      {/* HEADER: TABS */}
      {!isMinimal && (
      <header className="shrink-0 bg-slate-950/40 backdrop-blur-2xl border-b border-white/5 z-50 relative">
        {/* HUD Decorations */}
        <div className="absolute top-0 left-0 w-8 h-8 border-t border-l border-cyan-500/20 pointer-events-none rounded-tl-2xl"></div>
        <div className="absolute top-0 right-0 w-8 h-8 border-t border-r border-cyan-500/20 pointer-events-none"></div>

        {isOverlay && (
          <button 
            onClick={() => window.api?.hideOverlay?.()} 
            className="absolute top-2 right-3 text-slate-500 hover:text-red-500 transition-colors p-2 z-[60] rounded-lg hover:bg-white/5"
            title="Fechar Overlay (Ctrl + H)"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
          </button>
        )}

        <div className="w-full flex justify-center py-2">
          <nav className="flex items-center">
            <div className="w-[1px] h-6 bg-slate-800 self-center"></div>
            <button
              onClick={() => setActiveTab('macro')}
              className={`hd-tab-button ${activeTab === 'macro' ? 'hd-tab-active' : 'hd-tab-inactive'}`}
            >
              {t.tabs.macro}
            </button>
            <div className="w-[1px] h-6 bg-slate-800 self-center"></div>
            <button
              onClick={() => setActiveTab('build')}
              className={`hd-tab-button ${activeTab === 'build' ? 'hd-tab-active' : 'hd-tab-inactive'}`}
            >
              {t.tabs.build}
            </button>
            <div className="w-[1px] h-6 bg-slate-800 self-center"></div>
            <button
              onClick={() => setActiveTab('settings')}
              className={`hd-tab-button ${activeTab === 'settings' ? 'hd-tab-active-yellow' : 'hd-tab-inactive'}`}
            >
              {t.tabs.settings}
            </button>
            <div className="w-[1px] h-6 bg-slate-800 self-center"></div>
          </nav>
        </div>
      </header>
      )}

      {/* MAIN CONTENT AREA */}
      {!isMinimal && (
      <main className="flex-1 overflow-y-auto scrollbar-hd relative flex flex-col">

        {activeTab === 'macro' && (
          <div className="max-w-none mx-auto px-6 space-y-6 pb-32">
            <div className="flex items-center justify-center gap-4 py-2 opacity-60">
              <div className="h-[1px] flex-1 bg-gradient-to-r from-transparent to-slate-800"></div>
              <span className="text-[10px] font-black uppercase tracking-[0.2em] text-slate-400">
                {t.macro.selectTitle} {settings.shortcuts[activeSlot]}
              </span>
              <div className="h-[1px] flex-1 bg-gradient-to-l from-transparent to-slate-800"></div>
            </div>

            {!isOverlay && (
              <div className="relative">
                <span className="absolute left-4 top-1/2 -translate-y-1/2 text-slate-600 text-sm pointer-events-none">⌕</span>
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder={t.macro.searchPlaceholder}
                  className="w-full py-3 pl-10 pr-10 rounded-xl text-[11px] font-bold uppercase tracking-wider bg-slate-950/60 border-2 border-slate-800 text-slate-200 placeholder:text-slate-600 focus:border-cyan-500/50 focus:outline-none"
                />
                {searchQuery && (
                  <button
                    onClick={() => setSearchQuery('')}
                    className="absolute right-3 top-1/2 -translate-y-1/2 w-5 h-5 flex items-center justify-center text-slate-500 hover:text-white text-xs font-bold"
                  >
                    ×
                  </button>
                )}
              </div>
            )}

            {Object.keys(filteredStratagemsByTag).length === 0 && (
              <p className="text-center text-[10px] text-slate-600 uppercase tracking-widest py-8">
                {t.macro.searchNoResults} “{searchQuery}”
              </p>
            )}

            {sortedTags.filter(tag => filteredStratagemsByTag[tag]).map((tag) => {
              const sectionColor = tag === 'Offensive' ? 'red' : tag === 'Defensive' ? 'green' : 'cyan'
              const borderClass = sectionColor === 'red' ? 'hover:border-l-red-500/50' : sectionColor === 'green' ? 'hover:border-l-green-500/50' : 'hover:border-l-cyan-500/50'
              const indicatorClass = sectionColor === 'red' ? 'bg-red-500 shadow-[0_0_12px_rgba(239,68,68,0.8)] border-red-400/50' : sectionColor === 'green' ? 'bg-green-500 shadow-[0_0_12px_rgba(34,197,94,0.8)] border-green-400/50' : 'bg-cyan-500 shadow-[0_0_12px_rgba(34,211,238,0.8)] border-cyan-400/50'

              return (
                <section key={tag} className={`hd-card p-5 border-l-4 border-l-slate-700 ${borderClass} [content-visibility:auto]`}>
                  <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-5">
                    <div className={`w-2 h-2 rounded-full border ${indicatorClass}`}></div>
                    {tag === 'Offensive' ? t.settings.tagOffensive : 
                     tag === 'Supply' ? t.settings.tagSupply : 
                     tag === 'Defensive' ? t.settings.tagDefensive : tag}
                  </h2>

                <div className="grid grid-cols-4 gap-3">
                  {filteredStratagemsByTag[tag].map((strat) => (
                    <StratagemCard
                      key={strat.id}
                      strat={strat}
                      tag={tag}
                      isInActiveSlot={slots[activeSlot]?.id === strat.id}
                      disabled={isCardDisabled(strat)}
                      clearLabel={t.macro.clearSlot}
                      onAssign={handleAssignStratagem}
                    />
                  ))}
                </div>
              </section>
            )})}
          </div>
        )}

        {activeTab === 'build' && (
          <div className="max-w-5xl w-full mx-auto px-6 space-y-6 pt-6 pb-24">
            {/* SUB-ABAS: META | ALEATÓRIA */}
            <div className="flex justify-center">
              <div className="flex bg-slate-950/60 border border-white/5 rounded-2xl p-1 gap-1">
                {['meta', 'random'].map((sub) => (
                  <button
                    key={sub}
                    onClick={() => setBuildSubTab(sub)}
                    className={`py-2.5 px-8 rounded-xl text-[11px] font-black uppercase tracking-widest ${buildSubTab === sub
                      ? 'bg-yellow-500 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]'
                      : 'text-slate-500 hover:text-white'
                      }`}
                  >
                    {sub === 'meta' ? t.build.subMeta : t.build.subRandom}
                  </button>
                ))}
              </div>
            </div>

            {/* OPÇÕES DA BUILD */}
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
                    onClick={() => handleSettingChange(opt.key, !enabled)}
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
        )}

        {activeTab === 'settings' && (
          <div className="max-w-5xl mx-auto px-6 space-y-8 pt-6 pb-24">

            {/* ShortCuts Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* ATALHOS PRINCIPAIS */}
              <div className="hd-card p-6 border-t-2 border-t-yellow-500/20">
                <h2 className="hd-card-header text-sm !text-yellow-500">
                  <div className="hd-indicator bg-yellow-500 shadow-[0_0_12px_rgba(251,191,36,0.5)] border-yellow-400/50"></div>
                  {t.settings.keybinding}
                </h2>
                <div className="grid grid-cols-2 gap-4">
                  {[0, 1, 2, 3].map(i => (
                    <div key={i} className="bg-slate-950/60 p-5 rounded-2xl border border-slate-800/80 flex flex-col gap-3 hover:border-yellow-500/20">
                      <div className="text-[10px] font-black text-slate-500 uppercase tracking-[0.2em]">{t.settings.shortcutLabel} {i + 1}</div>
                      <button
                        onClick={async () => {
                          await window.api?.setRecordingMode?.(true)
                          setCapturingSlot(i)
                        }}
                        className={`w-full py-3.5 rounded-xl font-black text-xs tracking-widest border-2 ${capturingSlot === i
                          ? 'bg-yellow-500/10 border-yellow-500 text-yellow-400 animate-pulse-hd'
                          : 'bg-slate-900 border-slate-800 text-slate-300 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5'
                          }`}
                      >
                        {capturingSlot === i ? t.macro.listening : settings.shortcuts[i]}
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              {/* JOGO / MODIFICADORES */}
              <div className="hd-card p-6 border-t-2 border-t-slate-700">
                <h2 className="hd-card-header text-sm !text-yellow-500">
                  <div className="hd-indicator bg-yellow-500 shadow-none"></div>
                  {t.settings.controller}
                </h2>
                <div className="space-y-6">
                  <div className="space-y-3">
                    <label className="text-[10px] font-black text-slate-500 uppercase tracking-widest">{t.settings.ingameKey}</label>
                    <div className="grid grid-cols-2 gap-2">
                      {['LeftControl', 'LeftAlt', 'Equal', 'Minus'].map((key) => (
                        <button
                          key={key}
                          onClick={() => handleSettingChange('modifierKey', key)}
                          className={`py-3 rounded-xl text-[10px] font-black uppercase border-2 ${settings.modifierKey === key 
                            ? 'bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]' 
                            : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5'
                            }`}
                        >
                          {key === 'LeftControl' ? 'CTRL' : key === 'LeftAlt' ? 'ALT' : key === 'Equal' ? '=' : key === 'Minus' ? '-' : key}
                        </button>
                      ))}
                    </div>
                  </div>

                  <div className="space-y-3">
                    <label className="text-[10px] font-black text-slate-500 uppercase tracking-widest">{t.settings.sprint}</label>
                    <p className="text-[9px] text-slate-600 leading-tight">{t.settings.sprintDesc}</p>
                    <div className="grid grid-cols-4 gap-2">
                      {SPRINT_MODIFIERS.map((mod) => (
                        <button
                          key={mod}
                          onClick={() => handleSettingChange('sprintModifier', mod)}
                          className={`py-3 rounded-xl text-[10px] font-black uppercase border-2 ${settings.sprintModifier === mod
                            ? 'bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]'
                            : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5'
                            }`}
                        >
                          {mod === 'None' ? t.settings.sprintNone : mod}
                        </button>
                      ))}
                    </div>
                  </div>

                  <div className="space-y-3">
                    <label className="text-[10px] font-black text-slate-500 uppercase tracking-widest">{t.settings.macroSpeed}</label>
                    <p className="text-[9px] text-slate-600 leading-tight">{t.settings.macroSpeedDesc}</p>
                    <div className="grid grid-cols-3 gap-2">
                      {MACRO_SPEEDS.map((speed) => (
                        <button
                          key={speed}
                          onClick={() => handleSettingChange('macroSpeed', speed)}
                          className={`py-3 rounded-xl text-[10px] font-black uppercase border-2 ${(settings.macroSpeed || 'normal') === speed
                            ? 'bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]'
                            : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5'
                            }`}
                        >
                          {t.settings[`macroSpeed_${speed}`]}
                        </button>
                      ))}
                    </div>
                  </div>

                  <button
                    onClick={() => handleSettingChange('useArrows', !settings.useArrows)}
                    className={`w-full p-4 rounded-xl border-2 flex items-center justify-between group ${settings.useArrows 
                      ? 'bg-yellow-500/5 border-yellow-500/50' 
                      : 'bg-slate-950/40 border-slate-800 hover:border-yellow-500/30'
                    }`}
                  >
                    <div className="flex flex-col items-start gap-1">
                      <span className={`text-[10px] font-black uppercase tracking-widest ${settings.useArrows ? 'text-yellow-500' : 'text-slate-400'}`}>{t.settings.arrowMode}</span>
                      <span className="text-[9px] text-slate-500 uppercase text-left leading-tight">
                        {settings.useArrows 
                          ? t.settings.arrowActive 
                          : t.settings.wasdActive}
                      </span>
                    </div>
                    <div className={`w-10 h-5 rounded-full border-2 flex items-center px-1 ${settings.useArrows ? 'border-yellow-500 bg-yellow-500/20' : 'border-slate-700 bg-slate-900'}`}>
                      <div className={`w-2 h-2 rounded-full ${settings.useArrows ? 'translate-x-5 bg-yellow-500 shadow-[0_0_8px_rgba(234,179,8,0.8)]' : 'bg-slate-600'}`}></div>
                    </div>
                  </button>

                  <button 
                    onClick={() => {
                      const newValue = settings.enableOverlay === false ? true : false;
                      handleSettingChange('enableOverlay', newValue);
                      window.api?.setRecordingMode?.(false); // Trigger re-register
                    }}
                    className={`w-full p-4 rounded-xl border-2 flex items-center justify-between group ${settings.enableOverlay !== false 
                      ? 'bg-yellow-500/5 border-yellow-500/50' 
                      : 'bg-slate-950/40 border-slate-800 hover:border-yellow-500/30'
                    }`}
                  >
                    <div className="flex flex-col items-start gap-1">
                      <span className={`text-[10px] font-black uppercase tracking-widest ${settings.enableOverlay !== false ? 'text-yellow-500' : 'text-slate-400'}`}>
                        {settings.language === 'pt' ? 'Atalho do Overlay' : 'Overlay Shortcut'}
                      </span>
                      <span className="text-[9px] text-slate-500 uppercase text-left leading-tight">
                        {settings.enableOverlay !== false 
                          ? (settings.language === 'pt' ? 'Overlay Ativado (Ctrl + H)' : 'Overlay Active (Ctrl + H)')
                          : (settings.language === 'pt' ? 'Overlay Desativado' : 'Overlay Disabled')}
                      </span>
                    </div>
                    <div className={`w-10 h-5 rounded-full border-2 flex items-center px-1 ${settings.enableOverlay !== false ? 'border-yellow-500 bg-yellow-500/20' : 'border-slate-700 bg-slate-900'}`}>
                      <div className={`w-2 h-2 rounded-full ${settings.enableOverlay !== false ? 'translate-x-5 bg-yellow-500 shadow-[0_0_8px_rgba(234,179,8,0.8)]' : 'bg-slate-600'}`}></div>
                    </div>
                  </button>

                  <button 
                    onClick={() => handleSettingChange('alwaysShowSlots', !settings.alwaysShowSlots)}
                    className={`w-full p-4 rounded-xl border-2 flex items-center justify-between group ${settings.alwaysShowSlots 
                      ? 'bg-yellow-500/5 border-yellow-500/50' 
                      : 'bg-slate-950/40 border-slate-800 hover:border-yellow-500/30'
                    }`}
                  >
                    <div className="flex flex-col items-start gap-1">
                      <span className={`text-[10px] font-black uppercase tracking-widest ${settings.alwaysShowSlots ? 'text-yellow-500' : 'text-slate-400'}`}>
                        {settings.language === 'pt' ? 'HUD Persistente' : 'Persistent HUD'}
                      </span>
                      <span className="text-[9px] text-slate-500 uppercase text-left leading-tight">
                        {settings.alwaysShowSlots 
                          ? (settings.language === 'pt' ? 'Slots sempre visíveis no jogo' : 'Slots always visible in-game')
                          : (settings.language === 'pt' ? 'Esconder slots ao fechar' : 'Hide slots on close')}
                      </span>
                    </div>
                    <div className={`w-10 h-5 rounded-full border-2 flex items-center px-1 ${settings.alwaysShowSlots ? 'border-yellow-500 bg-yellow-500/20' : 'border-slate-700 bg-slate-900'}`}>
                      <div className={`w-2 h-2 rounded-full ${settings.alwaysShowSlots ? 'translate-x-5 bg-yellow-500 shadow-[0_0_8px_rgba(234,179,8,0.8)]' : 'bg-slate-600'}`}></div>
                    </div>
                  </button>
                </div>
              </div>

              {/* IDIOMA */}
              <div className="hd-card p-6 border-t-2 border-t-slate-700">
                <h2 className="hd-card-header text-sm !text-yellow-500">
                  <div className="hd-indicator bg-yellow-500 shadow-none"></div>
                  {t.settings.language}
                </h2>
                <div className="grid grid-cols-2 gap-2">
                  {['pt', 'en'].map((lang) => (
                    <button
                      key={lang}
                      onClick={() => handleSettingChange('language', lang)}
                      className={`py-3 rounded-xl text-[10px] font-black uppercase border-2 ${settings.language === lang 
                        ? 'bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]' 
                        : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5'
                        }`}
                    >
                      {lang === 'pt' ? 'Português' : 'English'}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            {/* ESTRATAGEMAS DE SUPORTE */}
            <div className="hd-card p-6 border-t-2 border-t-yellow-500/20">
              <h2 className="hd-card-header text-sm !text-yellow-500">
                <div className="hd-indicator bg-yellow-500 shadow-[0_0_12px_rgba(251,191,36,0.5)] border-yellow-400/50"></div>
                {t.settings.support}
              </h2>
              <div className="grid grid-cols-3 gap-5">
                {SUPPORT_STRATS.map((strat, i) => (
                  <div key={i} className="flex flex-col gap-4">
                    {/* The 1:1 Card Visual */}
                    <div className={`group relative aspect-square rounded-2xl border-2 bg-slate-900/40 overflow-hidden ${
                      supportActiveVisual === i
                        ? 'border-yellow-400 shadow-[0_0_20px_rgba(251,191,36,0.3)]'
                        : macroBlockedSlot?.isSupport && macroBlockedSlot.slot === i
                          ? 'border-red-500/70 shadow-[0_0_20px_rgba(239,68,68,0.25)]'
                          : 'border-slate-800/50 hover:border-yellow-500/50'
                    }`}>
                      {/* Stratagem Icon */}
                      <img 
                        src={strat.imagem} 
                        alt={strat.nome} 
                        decoding="async"
                        loading="lazy"
                        className="w-full h-full object-cover opacity-80 group-hover:scale-110 group-hover:opacity-100 transition-all duration-500 transform-gpu will-change-transform" 
                        style={{ imageRendering: 'auto' }}
                      />

                      {/* HUD Overlay: Name (Top) */}
                      <div className="absolute inset-x-0 top-0 bg-gradient-to-b from-slate-950 via-slate-950/70 to-transparent p-4 pb-10 flex justify-center z-10">
                        <span className="text-[11px] font-black text-slate-100 uppercase tracking-widest text-center leading-none">
                          {strat.nome}
                        </span>
                      </div>

                      {/* HUD Overlay: Codex (Bottom) */}
                      <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-slate-950 via-slate-950/90 to-transparent p-5 pt-12 flex justify-center z-10">
                        <div className={`flex justify-center ${strat.codex.length > 6 ? 'gap-1' : 'gap-2'}`}>
                          {strat.codex.map((dir, idx) => (
                            <ArrowIcon 
                              key={idx} 
                              direction={dir} 
                              size={strat.codex.length > 6 ? 13 : 16} 
                              className="text-cyan-500 drop-shadow-lg" 
                            />
                          ))}
                        </div>
                      </div>
                    </div>

                    {/* Shortcut Button */}
                    <button
                      onClick={async () => {
                        await window.api?.setRecordingMode?.(true)
                        setCapturingSlot(`support-${i}`)
                      }}
                      className={`w-full py-3.5 rounded-xl font-black text-xs tracking-widest border-2 ${capturingSlot === `support-${i}`
                        ? 'bg-yellow-500/10 border-yellow-500 text-yellow-400 animate-pulse-hd'
                        : 'bg-slate-900 border-slate-800 text-slate-300 hover:border-yellow-500/50 hover:text-white'
                        }`}
                    >
                      {capturingSlot === `support-${i}` ? t.macro.listening : (settings.supportShortcuts?.[i] || t.macro.bind)}
                    </button>
                  </div>
                ))}
              </div>
            </div>

            {/* BACKUP */}
            <div className="hd-card p-6 border-t-2 border-t-slate-700">
              <h2 className="hd-card-header text-sm !text-yellow-500">
                <div className="hd-indicator bg-yellow-500 shadow-none"></div>
                {t.settings.backup}
              </h2>
              <p className="text-[9px] text-slate-600 uppercase tracking-wider leading-tight mb-4">{t.settings.backupDesc}</p>
              <div className="grid grid-cols-2 gap-2">
                <button
                  onClick={handleExportData}
                  className="py-3 rounded-xl text-[10px] font-black uppercase border-2 bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5"
                >
                  {t.settings.backupExport}
                </button>
                <button
                  onClick={handleImportData}
                  className="py-3 rounded-xl text-[10px] font-black uppercase border-2 bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5"
                >
                  {t.settings.backupImport}
                </button>
              </div>
              {backupStatus && (
                <p className={`mt-3 text-[10px] font-black uppercase tracking-widest text-center ${backupStatus === 'error' ? 'text-red-400' : 'text-green-400'}`}>
                  {backupStatus === 'exported' ? t.settings.backupExported
                    : backupStatus === 'imported' ? t.settings.backupImported
                    : t.settings.backupError}
                </p>
              )}
            </div>

          </div>
        )}

        {/* SETTINGS FOOTER: VERSION INFO (FIXED) */}
        {activeTab === 'settings' && (
          <footer className="shrink-0 fixed bottom-0 left-1/2 -translate-x-1/2 w-full max-w-5xl z-50">
            {/* Styled Divider Line */}
            <div className="w-full h-[1px] bg-gradient-to-r from-transparent via-slate-800 to-transparent opacity-50"></div>
            
            <div className="py-6 px-6 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="text-[9px] font-black tracking-[0.4em] uppercase text-slate-600">{t.settings.version} v{pkg.version}</span>
                <div className="w-[1px] h-3 bg-slate-800"></div>
                <div className="flex items-center gap-1.5">
                  <div className={`w-1.5 h-1.5 rounded-full ${gameFocused ? 'bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.8)]' : 'bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.8)]'}`}></div>
                  <span className="text-[9px] font-black uppercase text-slate-500 tracking-[0.2em]">
                    {gameFocused ? t.settings.gameActive : t.settings.gameInactive}
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-2.5">
                {updateStatus.status === 'ready' ? (
                  <button 
                    onClick={() => window.api?.installUpdate?.()}
                    className="flex items-center gap-2 px-3 py-1 bg-yellow-500 rounded-lg text-[9px] font-black uppercase text-slate-950 animate-pulse-hd shadow-[0_0_15px_rgba(234,179,8,0.4)]"
                  >
                    <div className="w-1.5 h-1.5 rounded-full bg-slate-950"></div>
                    {t.settings.updateReady}
                  </button>
                ) : (
                  <>
                    <div className={`w-1 h-1 rounded-full ${updateStatus.status === 'error' ? 'bg-red-500' : 'bg-yellow-500'} shadow-[0_0_8px_rgba(251,191,36,0.8)] animate-pulse`}></div>
                    <span className="text-[9px] font-black uppercase text-slate-500 tracking-[0.2em]">
                      {updateStatus.status === 'checking' && t.settings.updateChecking}
                      {updateStatus.status === 'available' && t.settings.updateAvailable}
                      {updateStatus.status === 'downloading' && `${t.settings.updateDownloading} ${Math.round(updateStatus.percent)}%`}
                      {updateStatus.status === 'up-to-date' && t.settings.updateUpToDate}
                      {updateStatus.status === 'error' && t.settings.updateError}
                      {(!updateStatus.status || updateStatus.status === 'idle') && t.settings.updated}
                    </span>
                  </>
                )}
              </div>
            </div>
          </footer>
        )}

      </main>
      )}

      {/* FOOTER: SLOTS BAR (FIXED) */}
      {((activeTab === 'macro' && !isOverlay) || (isMinimal && settings.alwaysShowSlots) || (!isMinimal && isOverlay && activeTab === 'macro')) && (
        <footer className={`shrink-0 fixed transition-none ${isMinimal ? 'bottom-2 scale-[0.70] origin-bottom bg-slate-950/20 backdrop-blur-sm shadow-[0_0_15px_rgba(0,0,0,0.5)]' : 'bottom-6 bg-slate-950/60 backdrop-blur-2xl shadow-[0_20px_50px_-10px_rgba(0,0,0,0.8)]'} left-1/2 -translate-x-1/2 w-fit border border-white/5 p-4 rounded-3xl ${isMinimal ? 'z-[9999]' : 'z-[100]'}`}>
          <div className="flex flex-col items-center gap-3">
            <div className="flex justify-center gap-4">
              {slots.map((slot, index) => (
                <div key={index} className="flex-shrink-0 relative group">
                  {macroBlockedSlot && !macroBlockedSlot.isSupport && macroBlockedSlot.slot === index && (
                    <div className="absolute -inset-1 rounded-2xl bg-red-500/30 animate-pulse z-40 pointer-events-none" />
                  )}
                  <Slot
                    index={index}
                    selectedStratagem={slot}
                    isActive={activeSlot === index}
                    onSelectSlot={setActiveSlot}
                    shortcut={settings.shortcuts[index]}
                  />
                  {slot && !isMinimal && (
                    <button
                      onClick={() => handleClearSlot(index)}
                      title={t.macro.clearSlot}
                      className="absolute -top-2 -right-2 w-5 h-5 bg-red-500/80 hover:bg-red-500 rounded-full flex items-center justify-center text-white text-xs font-black opacity-0 group-hover:opacity-100 transition-opacity z-50 shadow-lg cursor-pointer"
                    >
                      ×
                    </button>
                  )}
                </div>
              ))}
            </div>
          </div>
        </footer>
      )}

      {/* UPDATE NOTIFICATION MODAL */}
      {updateStatus.status === 'ready' && (
        <div className="fixed inset-0 z-[2000] flex items-center justify-center p-6 bg-slate-950/80 backdrop-blur-md">
          <div className="w-full max-w-md bg-slate-900 border-2 border-yellow-500/50 rounded-3xl p-8 shadow-[0_0_50px_rgba(0,0,0,0.5)] relative overflow-hidden">
            {/* Background Glow */}
            <div className="absolute -top-24 -right-24 w-48 h-48 bg-yellow-500/10 blur-[60px] rounded-full"></div>
            
            <div className="relative space-y-6">
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 bg-yellow-500/20 rounded-2xl flex items-center justify-center border border-yellow-500/30">
                  <div className="w-3 h-3 bg-yellow-500 rounded-full animate-pulse"></div>
                </div>
                <div>
                  <h3 className="text-lg font-black uppercase tracking-widest text-yellow-500">
                    {settings.language === 'pt' ? 'Atualização Disponível' : 'Update Available'}
                  </h3>
                  <p className="text-[10px] text-slate-500 font-bold uppercase tracking-widest">
                    v{updateStatus.version}
                  </p>
                </div>
              </div>

              <p className="text-sm text-slate-300 leading-relaxed font-medium">
                {settings.language === 'pt' 
                  ? 'Uma nova versão foi baixada e está pronta para ser instalada. Deseja reiniciar o programa agora para aplicar as mudanças?' 
                  : 'A new version has been downloaded and is ready to install. Would you like to restart the program now to apply changes?'}
              </p>

              <div className="flex gap-3 pt-2">
                <button
                  onClick={() => window.api?.installUpdate?.()}
                  className="flex-1 px-6 py-4 bg-yellow-500 hover:bg-yellow-400 text-slate-950 rounded-2xl font-black uppercase tracking-widest text-xs transition-all shadow-[0_10px_20px_rgba(234,179,8,0.2)] hover:scale-[1.02] active:scale-95"
                >
                  {settings.language === 'pt' ? 'Reiniciar Agora' : 'Restart Now'}
                </button>
                <button
                  onClick={() => setUpdateStatus(prev => ({ ...prev, status: 'idle' }))}
                  className="px-6 py-4 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-2xl font-black uppercase tracking-widest text-xs transition-all border border-white/5"
                >
                  {settings.language === 'pt' ? 'Depois' : 'Later'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      </div>
    </div>
    </>
  )
}

export default App
