import { useState, useEffect, useCallback, useMemo } from 'react'
import Slot, { ArrowIcon } from './components/Slot'
import stratagemsData from './data/stratagems.json'
import { translations } from './data/translations'

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

// Estratagemas de Apoio Fixo que não mudam
const fixedSupportStrats = [
  { nome: 'Reinforce', imagem: 'Reinforce_Stratagem_Icon.png', codex: ['UP', 'DOWN', 'RIGHT', 'LEFT', 'UP'] },
  { nome: 'Resupply', imagem: 'Resupply_Stratagem_Icon.png', codex: ['DOWN', 'DOWN', 'UP', 'RIGHT'] },
  { nome: 'Eagle Rearm', imagem: 'Eagle_Rearm_Stratagem_Icon.png', codex: ['UP', 'UP', 'LEFT', 'UP', 'RIGHT'] }
]

function App() {
  const [activeTab, setActiveTab] = useState('macro')
  const [slots, setSlots] = useState([null, null, null, null])
  const [activeSlot, setActiveSlot] = useState(0)
  const [capturingSlot, setCapturingSlot] = useState(null)

  const [settings, setSettings] = useState({
    shortcuts: ['F1', 'F2', 'F3', 'F4'],
    supportShortcuts: [null, null, null],
    modifierKey: 'LeftControl',
    useArrows: false,
    language: 'pt'
  })
  const [updateStatus, setUpdateStatus] = useState({ status: 'idle', percent: 0 })
  const [isBooting, setIsBooting] = useState(true)

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
        setSlots(parsed)
        if (window.api) window.api.updateSlots(parsed)
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
    if (window.api && window.api.onUpdateStatus) {
      window.api.onUpdateStatus((info) => {
        setUpdateStatus(info)
      })
    }
    if (window.api && window.api.onGameFocusChanged) {
      window.api.onGameFocusChanged((focused) => {
        // Você pode adicionar um estado de foco aqui se quiser mostrar um aviso
        console.log('Game focus:', focused)
      })
    }
  }, [])

  const handleAssignStratagem = (stratagem) => {
    const isEquipped = slots.some(s => s && s.id === stratagem.id)
    if (isEquipped) return

    const isMecha = stratagem.tag && stratagem.tag.includes('Mecha')
    if (isMecha) {
      const hasOtherMecha = slots.some((s, i) => i !== activeSlot && s && s.tag && s.tag.includes('Mecha'))
      if (hasOtherMecha) return
    }

    const newSlots = [...slots]
    newSlots[activeSlot] = stratagem
    setSlots(newSlots)
    localStorage.setItem('helldivers-macro-slots', JSON.stringify(newSlots))
    if (window.api) window.api.updateSlots(newSlots)
    if (activeSlot < 3) setActiveSlot(activeSlot + 1)
  }

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
    window.api.invoke('set-recording-mode', false)
  }, [capturingSlot, settings])

  useEffect(() => {
    if (capturingSlot !== null) {
      const keyHandler = (e) => {
        if (e.key === 'Escape') {
          e.preventDefault()
          setCapturingSlot(null)
          window.api.invoke('set-recording-mode', false)
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

  return (
    <div className="h-screen flex flex-col bg-slate-950 text-slate-200 relative selection:bg-cyan-500/30 overflow-hidden">
      
      {/* BOOT INTRO SEQUENCE */}
      {isBooting && (
        <div className="fixed inset-0 z-[5000] bg-slate-950 flex flex-col items-center justify-center font-mono overflow-hidden">
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

      <div className={`h-full flex flex-col transition-all duration-1000 ${isBooting ? 'scale-110 blur-xl opacity-0' : 'scale-100 blur-0 opacity-100 animate-entrance'}`}>
      
      {/* HEADER: TABS */}
      <header className="shrink-0 bg-slate-950/40 backdrop-blur-2xl border-b border-white/5 z-50 relative">
        {/* HUD Decorations */}
        <div className="absolute top-0 left-0 w-8 h-8 border-t border-l border-cyan-500/20 pointer-events-none"></div>
        <div className="absolute top-0 right-0 w-8 h-8 border-t border-r border-cyan-500/20 pointer-events-none"></div>

        <div className="w-full flex justify-center">
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
              onClick={() => setActiveTab('settings')}
              className={`hd-tab-button ${activeTab === 'settings' ? 'hd-tab-active-yellow' : 'hd-tab-inactive'}`}
            >
              {t.tabs.settings}
            </button>
            <div className="w-[1px] h-6 bg-slate-800 self-center"></div>
          </nav>
        </div>
      </header>

      {/* MAIN CONTENT AREA */}
      <main className="flex-1 overflow-y-auto scrollbar-hd pt-4 pb-24">

        {activeTab === 'macro' && (
          <div className="max-w-none mx-auto px-6 space-y-6">
            <div className="flex items-center justify-center gap-4 py-2 opacity-60">
              <div className="h-[1px] flex-1 bg-gradient-to-r from-transparent to-slate-800"></div>
              <span className="text-[10px] font-black uppercase tracking-[0.2em] text-slate-400">
                {t.macro.selectTitle} {settings.shortcuts[activeSlot]}
              </span>
              <div className="h-[1px] flex-1 bg-gradient-to-l from-transparent to-slate-800"></div>
            </div>

            {sortedTags.map((tag) => {
              const sectionColor = tag === 'Offensive' ? 'red' : tag === 'Defensive' ? 'green' : 'cyan'
              const borderClass = sectionColor === 'red' ? 'hover:border-l-red-500/50' : sectionColor === 'green' ? 'hover:border-l-green-500/50' : 'hover:border-l-cyan-500/50'
              const indicatorClass = sectionColor === 'red' ? 'bg-red-500 shadow-[0_0_12px_rgba(239,68,68,0.8)] border-red-400/50' : sectionColor === 'green' ? 'bg-green-500 shadow-[0_0_12px_rgba(34,197,94,0.8)] border-green-400/50' : 'bg-cyan-500 shadow-[0_0_12px_rgba(34,211,238,0.8)] border-cyan-400/50'

              return (
                <section key={tag} className={`hd-card p-5 border-l-4 border-l-slate-700 ${borderClass} transition-all [content-visibility:auto]`}>
                  <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-5">
                    <div className={`w-2 h-2 rounded-full border ${indicatorClass}`}></div>
                    {tag === 'Offensive' ? t.settings.tagOffensive : 
                     tag === 'Supply' ? t.settings.tagSupply : 
                     tag === 'Defensive' ? t.settings.tagDefensive : tag}
                  </h2>

                <div className="grid grid-cols-4 gap-3">
                  {stratagemsByTag[tag].map((strat) => {
                    const isEquipped = slots.some(s => s && s.id === strat.id)
                    const isMecha = strat.tag && strat.tag.includes('Mecha')
                    const hasOtherMecha = slots.some((s, i) => i !== activeSlot && s && s.tag && s.tag.includes('Mecha'))
                    const disabled = isEquipped || (isMecha && hasOtherMecha)
                    
                    const tagColor = tag === 'Offensive' ? 'red' : tag === 'Defensive' ? 'green' : 'cyan'
                    const hoverClasses = {
                      red: 'hover:border-red-500/50 hover:shadow-[0_0_30px_rgba(239,68,68,0.15)]',
                      green: 'hover:border-green-500/50 hover:shadow-[0_0_30px_rgba(34,197,94,0.15)]',
                      cyan: 'hover:border-cyan-500/50 hover:shadow-[0_0_30px_rgba(6,182,212,0.15)]'
                    }[tagColor]

                    return (
                      <button
                        key={strat.id}
                        onClick={() => !disabled && handleAssignStratagem(strat)}
                        className={`group relative aspect-square rounded-2xl border-2 transition-all overflow-hidden
                          ${disabled
                            ? 'bg-slate-950/50 border-slate-900 opacity-20 cursor-not-allowed'
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
                  })}
                </div>
              </section>
            )})}
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
                    <div key={i} className="bg-slate-950/60 p-5 rounded-2xl border border-slate-800/80 flex flex-col gap-3 transition-all hover:border-yellow-500/20">
                      <div className="text-[10px] font-black text-slate-500 uppercase tracking-[0.2em]">{t.settings.shortcutLabel} {i + 1}</div>
                      <button
                        onClick={async () => {
                          await window.api.invoke('set-recording-mode', true)
                          setCapturingSlot(i)
                        }}
                        className={`w-full py-3.5 rounded-xl font-black text-xs tracking-widest border-2 transition-all ${capturingSlot === i
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
                          className={`py-3 rounded-xl text-[10px] font-black uppercase transition-all border-2 ${settings.modifierKey === key 
                            ? 'bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_0_15px_rgba(234,179,8,0.3)]' 
                            : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-white hover:bg-yellow-500/5'
                            }`}
                        >
                          {key === 'LeftControl' ? 'CTRL' : key === 'LeftAlt' ? 'ALT' : key === 'Equal' ? '=' : key === 'Minus' ? '-' : key}
                        </button>
                      ))}
                    </div>
                  </div>

                  <button 
                    onClick={() => handleSettingChange('useArrows', !settings.useArrows)}
                    className={`w-full p-4 rounded-xl border-2 transition-all flex items-center justify-between group ${settings.useArrows 
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
                    <div className={`w-10 h-5 rounded-full border-2 transition-all flex items-center px-1 ${settings.useArrows ? 'border-yellow-500 bg-yellow-500/20' : 'border-slate-700 bg-slate-900'}`}>
                      <div className={`w-2 h-2 rounded-full transition-all ${settings.useArrows ? 'translate-x-5 bg-yellow-500 shadow-[0_0_8px_rgba(234,179,8,0.8)]' : 'bg-slate-600'}`}></div>
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
                      className={`py-3 rounded-xl text-[10px] font-black uppercase transition-all border-2 ${settings.language === lang 
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
                {fixedSupportStrats.map((strat, i) => (
                  <div key={i} className="flex flex-col gap-4">
                    {/* The 1:1 Card Visual */}
                    <div className="group relative aspect-square rounded-2xl border-2 border-slate-800/50 bg-slate-900/40 overflow-hidden transition-all hover:border-yellow-500/50">
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
                        await window.api.invoke('set-recording-mode', true)
                        setCapturingSlot(`support-${i}`)
                      }}
                      className={`w-full py-3.5 rounded-xl font-black text-xs tracking-widest border-2 transition-all ${capturingSlot === `support-${i}`
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

          </div>
        )}

        {/* SETTINGS FOOTER: VERSION INFO (FIXED) */}
        {activeTab === 'settings' && (
          <footer className="shrink-0 fixed bottom-0 left-1/2 -translate-x-1/2 w-full max-w-5xl z-50">
            {/* Styled Divider Line */}
            <div className="w-full h-[1px] bg-gradient-to-r from-transparent via-slate-800 to-transparent opacity-50"></div>
            
            <div className="py-6 px-6 flex items-center justify-between">
              <span className="text-[9px] font-black tracking-[0.4em] uppercase text-slate-600">{t.settings.version} v0.2.1</span>
              <div className="flex items-center gap-2.5">
                {updateStatus.status === 'ready' ? (
                  <button 
                    onClick={() => window.api.invoke('install-update')}
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

      {/* FOOTER: SLOTS BAR (FIXED) */}
      {activeTab === 'macro' && (
        <footer className="shrink-0 fixed bottom-6 left-1/2 -translate-x-1/2 w-fit bg-slate-950/60 backdrop-blur-2xl border border-white/5 p-4 rounded-3xl z-[100] shadow-[0_20px_50px_-10px_rgba(0,0,0,0.8)]">
          <div className="flex flex-col items-center gap-3">
            <div className="flex justify-center gap-4">
              {slots.map((slot, index) => (
                <div key={index} className="flex-shrink-0">
                  <Slot
                    index={index}
                    selectedStratagem={slot}
                    isActive={activeSlot === index}
                    onSelectSlot={setActiveSlot}
                    shortcut={settings.shortcuts[index]}
                  />
                </div>
              ))}
            </div>
          </div>
        </footer>
      )}

      {/* UPDATE NOTIFICATION MODAL */}
      {updateStatus.status === 'ready' && (
        <div className="fixed inset-0 z-[2000] flex items-center justify-center p-6 bg-slate-950/80 backdrop-blur-md animate-in fade-in duration-300">
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
                  onClick={() => window.api.invoke('install-update')}
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
  )
}

export default App
