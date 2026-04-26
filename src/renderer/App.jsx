import { useState, useEffect, useCallback } from 'react'
import Slot, { ArrowIcon } from './components/Slot'
import stratagemsData from './data/stratagems.json'

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

function App() {
  const [activeTab, setActiveTab] = useState('macro')
  const [slots, setSlots] = useState([null, null, null, null])
  const [activeSlot, setActiveSlot] = useState(0)
  const [capturingSlot, setCapturingSlot] = useState(null)

  const [settings, setSettings] = useState({
    shortcuts: ['F1', 'F2', 'F3', 'F4'],
    supportShortcuts: [null, null, null],
    modifierKey: 'LeftControl',
    useArrows: false
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

  const stratagemsByTag = {}
  stratagemsData.forEach(strat => {
    const tag = (strat.tag && strat.tag.length > 0) ? strat.tag[0] : 'Outros'
    if (!stratagemsByTag[tag]) stratagemsByTag[tag] = []
    stratagemsByTag[tag].push(strat)
  })

  const sortedTags = Object.keys(stratagemsByTag).sort((a, b) => {
    const order = ['Offensive', 'Supply', 'Defensive']
    const indexA = order.indexOf(a), indexB = order.indexOf(b)
    if (indexA !== -1 && indexB !== -1) return indexA - indexB
    return indexA !== -1 ? -1 : indexB !== -1 ? 1 : a.localeCompare(b)
  })

  return (
    <div className="h-screen flex flex-col bg-slate-950 text-slate-200">

      {/* HEADER: TABS */}
      <header className="shrink-0 bg-slate-950/40 backdrop-blur-2xl border-b border-white/5 z-50 relative">
        {/* HUD Decorations */}
        <div className="absolute top-0 left-0 w-8 h-8 border-t border-l border-yellow-500/20 pointer-events-none"></div>
        <div className="absolute top-0 right-0 w-8 h-8 border-t border-r border-yellow-500/20 pointer-events-none"></div>

        <div className="w-full flex justify-center">
          <nav className="flex items-center">
            <div className="w-[1px] h-6 bg-slate-800 self-center"></div>
            <button
              onClick={() => setActiveTab('macro')}
              className={`hd-tab-button ${activeTab === 'macro' ? 'hd-tab-active' : 'hd-tab-inactive'}`}
            >
              Configurar Macros
            </button>
            <div className="w-[1px] h-6 bg-slate-800 self-center"></div>
            <button
              onClick={() => setActiveTab('settings')}
              className={`hd-tab-button ${activeTab === 'settings' ? 'hd-tab-active' : 'hd-tab-inactive'}`}
            >
              Configurações
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
                Selecionar Estratagemas para o Slot {settings.shortcuts[activeSlot]}
              </span>
              <div className="h-[1px] flex-1 bg-gradient-to-l from-transparent to-slate-800"></div>
            </div>

            {sortedTags.map((tag) => (
              <section key={tag} className="hd-card p-5 border-l-4 border-l-slate-700 hover:border-l-yellow-500/50 transition-all">
                <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-5">
                  <div className="hd-indicator"></div>
                  {tag}
                </h2>

                <div className="grid grid-cols-4 gap-3">
                  {stratagemsByTag[tag].map((strat) => {
                    const isEquipped = slots.some(s => s && s.id === strat.id)
                    const isMecha = strat.tag && strat.tag.includes('Mecha')
                    const hasOtherMecha = slots.some((s, i) => i !== activeSlot && s && s.tag && s.tag.includes('Mecha'))
                    const disabled = isEquipped || (isMecha && hasOtherMecha)

                    return (
                      <button
                        key={strat.id}
                        onClick={() => !disabled && handleAssignStratagem(strat)}
                        className={`group relative aspect-square rounded-2xl border-2 transition-all overflow-hidden
                          ${disabled
                            ? 'bg-slate-950/50 border-slate-900 opacity-20 cursor-not-allowed'
                            : 'bg-slate-900/40 border-slate-800/50 hover:border-yellow-500/50 hover:shadow-[0_0_30px_rgba(251,191,36,0.15)]'}`}
                      >
                        {/* Stratagem Icon - Full bleed */}
                        <img 
                          src={strat.imagem} 
                          alt={strat.nome} 
                          className="w-full h-full object-cover opacity-70 group-hover:scale-110 group-hover:opacity-100 transition-all duration-500" 
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
                                className="text-yellow-500 drop-shadow-lg" 
                              />
                            ))}
                          </div>
                        </div>

                      </button>
                    )
                  })}
                </div>
              </section>
            ))}
          </div>
        )}

        {activeTab === 'settings' && (
          <div className="max-w-5xl mx-auto px-6 space-y-8 pt-6 pb-24">

            {/* ShortCuts Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* ATALHOS PRINCIPAIS */}
              <div className="hd-card p-6 border-t-2 border-t-yellow-500/20">
                <h2 className="hd-card-header text-sm">
                  <div className="hd-indicator"></div>
                  Atalhos de Combate
                </h2>
                <div className="grid grid-cols-2 gap-4">
                  {[0, 1, 2, 3].map(i => (
                    <div key={i} className="bg-slate-950/60 p-5 rounded-2xl border border-slate-800/80 flex flex-col gap-3 transition-all hover:border-yellow-500/20">
                      <div className="text-[10px] font-black text-slate-500 uppercase tracking-[0.2em]">Atalho {i + 1}</div>
                      <button
                        onClick={async () => {
                          await window.api.invoke('set-recording-mode', true)
                          setCapturingSlot(i)
                        }}
                        className={`w-full py-3.5 rounded-xl font-black text-xs tracking-widest border-2 transition-all ${capturingSlot === i
                          ? 'bg-yellow-500/10 border-yellow-500 text-yellow-400 animate-pulse-hd'
                          : 'bg-slate-900 border-slate-800 text-slate-300 hover:border-yellow-500/50 hover:text-white'
                          }`}
                      >
                        {capturingSlot === i ? 'ESCUTANDO...' : settings.shortcuts[i]}
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              {/* JOGO / MODIFICADORES */}
              <div className="hd-card p-6 border-t-2 border-t-slate-700">
                <h2 className="hd-card-header text-sm">
                  <div className="hd-indicator bg-slate-500 shadow-none"></div>
                  Parâmetros de Missão
                </h2>
                <div className="space-y-6">
                  <div className="space-y-3">
                    <label className="text-[10px] font-black text-slate-500 uppercase tracking-widest">Tecla do Menu In-Game</label>
                    <div className="grid grid-cols-2 gap-2">
                      {['LeftControl', 'LeftAlt', 'Equal', 'Minus'].map((key) => (
                        <button
                          key={key}
                          onClick={() => handleSettingChange('modifierKey', key)}
                          className={`py-2 rounded-md text-[10px] font-black uppercase transition-all border ${settings.modifierKey === key ? 'bg-yellow-500 border-yellow-600 text-slate-950' : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-600'
                            }`}
                        >
                          {key === 'LeftControl' ? 'CTRL' : key === 'LeftAlt' ? 'ALT' : key}
                        </button>
                      ))}
                    </div>
                  </div>

                  <div className="flex items-center justify-between p-3 bg-slate-950 rounded-lg border border-slate-800">
                    <div className="flex flex-col">
                      <span className="text-[10px] font-black uppercase text-slate-300">Modo de Digitação</span>
                      <span className="text-[9px] text-slate-500 uppercase">{settings.useArrows ? 'Setas (Seguro)' : 'WASD (Movimento)'}</span>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input type="checkbox" checked={settings.useArrows} onChange={(e) => handleSettingChange('useArrows', e.target.checked)} className="sr-only peer" />
                      <div className="w-9 h-5 bg-slate-800 rounded-full peer peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[4px] after:left-[4px] after:bg-slate-400 after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:bg-yellow-500 peer-checked:after:bg-slate-950"></div>
                    </label>
                  </div>
                </div>
              </div>
            </div>

            {/* ESTRATAGEMAS DE SUPORTE */}
            <div className="hd-card p-6 border-t-2 border-t-green-500/20">
              <h2 className="hd-card-header text-sm">
                <div className="hd-indicator bg-green-500 shadow-[0_0_12px_rgba(34,197,94,0.5)]"></div>
                Estratagemas de Apoio Fixo
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
                        className="w-full h-full object-cover opacity-80 group-hover:scale-110 group-hover:opacity-100 transition-all duration-500" 
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
                              className="text-yellow-500 drop-shadow-lg" 
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
                      className={`w-full py-4 rounded-xl font-black text-[11px] tracking-[0.2em] uppercase border-2 transition-all ${capturingSlot === `support-${i}`
                        ? 'bg-yellow-500/10 border-yellow-500 text-yellow-400 animate-pulse-hd shadow-[0_0_20px_rgba(251,191,36,0.2)]'
                        : 'bg-slate-950 border-slate-800 text-slate-500 hover:border-slate-600 hover:text-slate-300'
                        }`}
                    >
                      {capturingSlot === `support-${i}` ? 'AGUARDANDO TECLA...' : (settings.supportShortcuts?.[i] || 'VINCULAR ATALHO')}
                    </button>
                  </div>
                ))}
              </div>
              <p className="text-[9px] text-slate-500 mt-6 uppercase tracking-[0.2em] text-center opacity-50">
                Protocolo de Emergência // Atribuição de Tecla Rápida
              </p>
            </div>

            {/* INFO FOOTER */}
            <div className="flex items-center justify-between opacity-40 px-2">
              <span className="text-[9px] font-black tracking-widest uppercase">Versão v0.1.0 // Super Earth Command</span>
              <div className="flex items-center gap-2">
                <div className="w-1.5 h-1.5 rounded-full bg-green-500"></div>
                <span className="text-[9px] font-black uppercase">Sistema Operacional</span>
              </div>
            </div>
          </div>
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
