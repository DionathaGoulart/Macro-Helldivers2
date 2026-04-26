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
  { nome: 'Reinforce', imagem: '/Reinforce_Stratagem_Icon.png', codex: ['UP', 'DOWN', 'RIGHT', 'LEFT', 'UP'] },
  { nome: 'Resupply', imagem: '/Resupply_Stratagem_Icon.png', codex: ['DOWN', 'DOWN', 'UP', 'RIGHT'] },
  { nome: 'Eagle Rearm', imagem: '/Eagle_Rearm_Stratagem_Icon.png', codex: ['UP', 'UP', 'LEFT', 'UP', 'RIGHT'] }
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
    <div className="h-screen flex flex-col bg-slate-950 text-slate-200 relative selection:bg-cyan-500/30">

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
                    {tag}
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
              <span className="text-[9px] font-black tracking-[0.4em] uppercase text-slate-600">{t.settings.version} v0.2.0</span>
              <div className="flex items-center gap-2.5">
                <div className="w-1 h-1 rounded-full bg-yellow-500 shadow-[0_0_8px_rgba(251,191,36,0.8)] animate-pulse"></div>
                <span className="text-[9px] font-black uppercase text-slate-500 tracking-[0.2em]">{t.settings.updated}</span>
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

    </div>
  )
}

export default App
