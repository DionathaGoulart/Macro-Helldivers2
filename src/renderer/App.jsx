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
                        className={`group relative flex flex-col items-center p-4 rounded-2xl border-2 transition-all gap-3
                          ${disabled
                            ? 'bg-slate-950/50 border-slate-900 opacity-20 cursor-not-allowed'
                            : 'bg-slate-900/40 border-slate-800/50 hover:bg-slate-800/60 hover:border-yellow-500/50 hover:-translate-y-2 hover:shadow-[0_10px_30px_-10px_rgba(251,191,36,0.15)]'}`}
                      >
                        <img src={strat.imagem} alt={strat.nome} className="w-16 h-16 object-contain drop-shadow-2xl group-hover:scale-110 transition-transform duration-300" />

                        <div className="flex flex-col items-center gap-2 w-full">
                          <span className="text-[11px] font-black text-slate-200 uppercase tracking-tight text-center leading-tight h-7 flex items-center">
                            {strat.nome}
                          </span>

                          <div className="flex gap-1 bg-slate-950/80 px-2 py-1.5 rounded-lg border border-slate-800/60 shadow-inner group-hover:border-yellow-500/30 transition-colors">
                            {strat.codex.map((dir, i) => (
                              <ArrowIcon key={i} direction={dir} size={12} className="group-hover:text-yellow-400" />
                            ))}
                          </div>
                        </div>

                        {isEquipped && (
                          <div className="absolute inset-0 bg-yellow-500/5 rounded-2xl border border-yellow-500/30 flex items-start justify-end p-2 pointer-events-none">
                            <span className="bg-yellow-500 text-slate-950 text-[7px] font-black px-1.5 py-0.5 rounded-sm uppercase tracking-tighter shadow-sm">ATIVO</span>
                          </div>
                        )}
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
                  {[0, 1, 2, 3].map(i => {
                    const equipped = slots[i]
                    return (
                      <div key={i} className="bg-slate-950/60 p-4 rounded-2xl border border-slate-800/80 flex flex-col items-center gap-4 transition-all hover:border-yellow-500/30 group">
                        <div className="flex flex-col items-center gap-3">
                          <div className="text-[9px] font-black text-slate-500 uppercase tracking-widest">Slot {i + 1}</div>
                          {equipped ? (
                            <div className="flex flex-col items-center gap-2">
                              <img src={equipped.imagem} alt={equipped.nome} className="w-10 h-10 object-contain drop-shadow-xl group-hover:scale-110 transition-transform duration-300" />
                              <div className="text-[9px] font-bold text-slate-400 uppercase tracking-tight text-center">{equipped.nome}</div>
                            </div>
                          ) : (
                            <div className="w-10 h-10 border-2 border-dashed border-slate-800 rounded-lg flex items-center justify-center">
                              <span className="text-[8px] font-black text-slate-700 uppercase">Vazio</span>
                            </div>
                          )}
                        </div>

                        <button
                          onClick={async () => {
                            await window.api.invoke('set-recording-mode', true)
                            setCapturingSlot(i)
                          }}
                          className={`w-full py-2.5 rounded-xl font-black text-[10px] tracking-widest border-2 transition-all ${
                            capturingSlot === i 
                              ? 'bg-yellow-500/10 border-yellow-500 text-yellow-400 animate-pulse-hd' 
                              : 'bg-slate-900 border-slate-800 text-slate-300 hover:border-yellow-500/50 hover:text-yellow-400'
                          }`}
                        >
                          {capturingSlot === i ? 'AGUARDANDO...' : settings.shortcuts[i]}
                        </button>
                      </div>
                    )
                  })}
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
                {[
                  { nome: 'Reinforce', imagem: '/Reinforce_Stratagem_Icon.png', codex: ['UP', 'DOWN', 'RIGHT', 'LEFT', 'UP'] },
                  { nome: 'Resupply', imagem: '/Resupply_Stratagem_Icon.png', codex: ['DOWN', 'DOWN', 'UP', 'RIGHT'] },
                  { nome: 'Eagle Rearm', imagem: '/Eagle_Rearm_Stratagem_Icon.png', codex: ['UP', 'UP', 'LEFT', 'UP', 'RIGHT'] }
                ].map((strat, i) => (
                  <div key={i} className="bg-slate-950/60 p-6 rounded-2xl border border-slate-800/80 flex flex-col items-center gap-5 transition-all hover:border-green-500/30 group">
                    <img src={strat.imagem} alt={strat.nome} className="w-16 h-16 object-contain drop-shadow-2xl group-hover:scale-110 transition-transform duration-300" />
                    
                    <div className="text-center w-full space-y-3">
                      <div className="text-[12px] font-black uppercase tracking-widest text-slate-200">{strat.nome}</div>
                      
                      <div className="flex justify-center gap-1.5 bg-slate-900/80 py-2 rounded-lg border border-slate-800 shadow-inner">
                        {strat.codex.map((dir, idx) => <ArrowIcon key={idx} direction={dir} size={12} />)}
                      </div>
                    </div>

                    <button
                      onClick={async () => {
                        await window.api.invoke('set-recording-mode', true)
                        setCapturingSlot(`support-${i}`)
                      }}
                      className={`w-full py-3 rounded-xl font-black text-[10px] tracking-widest border-2 transition-all ${
                        capturingSlot === `support-${i}` 
                          ? 'bg-yellow-500/10 border-yellow-500 text-yellow-400 animate-pulse-hd' 
                          : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-yellow-500/50 hover:text-yellow-500'
                      }`}
                    >
                      {capturingSlot === `support-${i}` ? 'AGUARDANDO...' : (settings.supportShortcuts?.[i] || 'VINCULAR')}
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
        <footer className="shrink-0 fixed bottom-0 left-0 right-0 bg-slate-950/80 backdrop-blur-xl border-t border-slate-900 p-4 pb-6 z-[100]">
          <div className="max-w-5xl mx-auto flex flex-col items-center gap-4">
            <div className="flex justify-center gap-4 flex-wrap">
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
