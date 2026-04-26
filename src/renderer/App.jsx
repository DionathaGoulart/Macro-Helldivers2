import { useState, useEffect, useCallback } from 'react'
import Slot, { ArrowIcon } from './components/Slot'
import stratagemsData from './data/stratagems.json'

const keyMap = {
  'F1': 'F1', 'F2': 'F2', 'F3': 'F3', 'F4': 'F4',
  'F5': 'F5', 'F6': 'F6', 'F7': 'F7', 'F8': 'F8',
  'F9': 'F9', 'F10': 'F10', 'F11': 'F11', 'F12': 'F12',
  'Control': 'LeftControl', 'Shift': 'LeftShift', 'Alt': 'LeftAlt',
  'ArrowUp': 'Up', 'ArrowDown': 'Down', 'ArrowLeft': 'Left', 'ArrowRight': 'Right',
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
    }
  }, [])

  const handleAssignStratagem = (stratagem) => {
    if (slots.some(s => s && s.id === stratagem.id)) return
    const isMecha = stratagem.tag?.includes('Mecha')
    if (isMecha && slots.some((s, i) => i !== activeSlot && s?.tag?.includes('Mecha'))) return
    
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

  const handleKeyCapture = useCallback((e) => {
    if (capturingSlot === null) return
    e.preventDefault()
    if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return
    if (e.key === 'Escape') { setCapturingSlot(null); return }

    let key = e.code.replace('Key', '').replace('Digit', '')
    const mapped = keyMap[e.key] || keyMap[key] || key

    if (typeof capturingSlot === 'number') {
      const newShortcuts = [...settings.shortcuts]
      newShortcuts[capturingSlot] = mapped
      handleSettingChange('shortcuts', newShortcuts)
    } else if (typeof capturingSlot === 'string') {
      const idx = parseInt(capturingSlot.split('-')[1])
      const newShortcuts = [...settings.supportShortcuts]
      newShortcuts[idx] = mapped
      handleSettingChange('supportShortcuts', newShortcuts)
    }
    setCapturingSlot(null)
    window.api.invoke('set-recording-mode', false)
  }, [capturingSlot, settings])

  useEffect(() => {
    if (capturingSlot === null) return
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
  }, [capturingSlot, handleKeyCapture])

  const stratagemsByTag = {}
  stratagemsData.forEach(strat => {
    const tag = strat.tag?.[0] || 'Outros'
    if (!stratagemsByTag[tag]) stratagemsByTag[tag] = []
    stratagemsByTag[tag].push(strat)
  })

  return (
    <div className="h-screen flex flex-col bg-[#0f172a] text-slate-100">
      
      <nav className="hd-navbar shrink-0 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="w-8 h-8 bg-yellow-500 rounded flex items-center justify-center">
            <span className="text-slate-950 font-black text-lg italic">H</span>
          </div>
          <span className="text-xs font-black uppercase tracking-[0.3em] text-white">Super Earth Macro</span>
        </div>

        <div className="flex h-full">
          <button 
            onClick={() => setActiveTab('macro')}
            className={`hd-nav-link ${activeTab === 'macro' ? 'hd-nav-active' : 'hd-nav-inactive'}`}
          >
            Configurar Macros
          </button>
          <button 
            onClick={() => setActiveTab('settings')}
            className={`hd-nav-link ${activeTab === 'settings' ? 'hd-nav-active' : 'hd-nav-inactive'}`}
          >
            Configurações
          </button>
        </div>
      </nav>

      <main className="flex-1 overflow-y-auto scrollbar-hd pt-8 pb-32 px-10">
        
        {activeTab === 'macro' && (
          <div className="max-w-5xl mx-auto space-y-8">
            {['Offensive', 'Supply', 'Defensive'].map((tag) => stratagemsByTag[tag] && (
              <section key={tag} className="hd-section p-6">
                <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-6">
                  <div className="hd-indicator"></div>
                  {tag}
                </h2>
                
                <div className="grid grid-cols-4 gap-4">
                  {stratagemsByTag[tag].sort((a,b) => a.id - b.id).map((strat) => {
                    const isEquipped = slots.some(s => s && s.id === strat.id)
                    const disabled = isEquipped || (strat.tag?.includes('Mecha') && slots.some((s, i) => i !== activeSlot && s?.tag?.includes('Mecha')))

                    return (
                      <button
                        key={strat.id}
                        onClick={() => !disabled && handleAssignStratagem(strat)}
                        className={`group relative flex flex-col items-center p-4 rounded-xl border-2 transition-all gap-3
                          ${disabled 
                            ? 'bg-slate-950/20 border-slate-900 opacity-20 cursor-not-allowed' 
                            : 'bg-slate-900/40 border-slate-800/60 hover:bg-slate-800/60 hover:border-yellow-500/40 hover:-translate-y-1'}`}
                      >
                        <img src={strat.imagem} alt={strat.nome} className="w-10 h-10 object-contain drop-shadow-xl group-hover:scale-110 transition-transform" />
                        <span className="text-[10px] font-bold text-slate-300 leading-tight text-center h-4 flex items-center">{strat.nome}</span>
                        <div className="flex gap-1 bg-black/40 p-1.5 rounded-sm border border-slate-800/50">
                          {strat.codex.map((dir, i) => <ArrowIcon key={i} direction={dir} size={10} />)}
                        </div>
                        {isEquipped && (
                          <div className="absolute inset-0 bg-yellow-500/5 rounded-xl flex items-center justify-center">
                            <div className="w-1.5 h-1.5 rounded-full bg-yellow-500 shadow-[0_0_8px_rgba(251,191,36,0.5)]"></div>
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
          <div className="max-w-4xl mx-auto space-y-8">
            <div className="grid grid-cols-2 gap-8">
              <div className="hd-section p-8">
                <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-6">
                  <div className="hd-indicator"></div>
                  Atalhos de Ativação
                </h2>
                <div className="grid grid-cols-2 gap-4">
                  {[0, 1, 2, 3].map(i => (
                    <div key={i} className="space-y-2">
                      <label className="text-[9px] font-black text-slate-500 uppercase">Slot {i + 1}</label>
                      <button
                        onClick={async () => {
                          await window.api.invoke('set-recording-mode', true)
                          setCapturingSlot(i)
                        }}
                        className={`w-full py-4 rounded-lg font-black text-xs border-2 transition-all ${
                          capturingSlot === i ? 'hd-btn-recording' : 'bg-slate-950 border-slate-800 text-slate-300 hover:border-slate-700'
                        }`}
                      >
                        {capturingSlot === i ? '...' : settings.shortcuts[i]}
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              <div className="hd-section p-8">
                <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-6">
                  <div className="hd-indicator"></div>
                  Preferências
                </h2>
                <div className="space-y-6">
                  <div className="space-y-3">
                    <label className="text-[9px] font-black text-slate-500 uppercase">Tecla Modificadora</label>
                    <div className="grid grid-cols-2 gap-2">
                      {['LeftControl', 'LeftAlt', 'Equal', 'Minus'].map((key) => (
                        <button
                          key={key}
                          onClick={() => handleSettingChange('modifierKey', key)}
                          className={`py-3 rounded-lg text-[10px] font-black uppercase transition-all border ${
                            settings.modifierKey === key ? 'bg-yellow-500 border-yellow-600 text-slate-950' : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                          }`}
                        >
                          {key === 'LeftControl' ? 'CTRL' : key === 'LeftAlt' ? 'ALT' : key}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="flex items-center justify-between p-4 bg-slate-950 rounded-xl border border-slate-800">
                    <span className="text-[10px] font-black uppercase text-slate-400">Digitação Segura (Setas)</span>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input type="checkbox" checked={settings.useArrows} onChange={(e) => handleSettingChange('useArrows', e.target.checked)} className="sr-only peer" />
                      <div className="w-10 h-5 bg-slate-800 rounded-full peer peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[4px] after:left-[4px] after:bg-slate-500 after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:bg-yellow-500 peer-checked:after:bg-slate-950"></div>
                    </label>
                  </div>
                </div>
              </div>
            </div>

            <div className="hd-section p-8">
              <h2 className="flex items-center gap-3 text-xs font-black uppercase tracking-widest text-slate-400 mb-6">
                <div className="hd-indicator"></div>
                Estratagemas de Suporte
              </h2>
              <div className="grid grid-cols-3 gap-6">
                {[
                  { nome: 'Reinforce', imagem: '/Reinforce_Stratagem_Icon.png', codex: ['UP', 'DOWN', 'RIGHT', 'LEFT', 'UP'] },
                  { nome: 'Resupply', imagem: '/Resupply_Stratagem_Icon.png', codex: ['DOWN', 'DOWN', 'UP', 'RIGHT'] },
                  { nome: 'Eagle Rearm', imagem: '/Eagle_Rearm_Stratagem_Icon.png', codex: ['UP', 'UP', 'LEFT', 'UP', 'RIGHT'] }
                ].map((strat, i) => (
                  <div key={i} className="bg-slate-950/40 p-5 rounded-xl border border-slate-800 space-y-4">
                    <div className="flex items-center justify-between">
                      <img src={strat.imagem} alt={strat.nome} className="w-9 h-9 object-contain" />
                      <div className="flex gap-1">
                        {strat.codex.map((dir, idx) => <ArrowIcon key={idx} direction={dir} size={11} />)}
                      </div>
                    </div>
                    <div className="text-[10px] font-black uppercase text-slate-400">{strat.nome}</div>
                    <button
                      onClick={async () => {
                        await window.api.invoke('set-recording-mode', true)
                        setCapturingSlot(`support-${i}`)
                      }}
                      className={`w-full py-3 rounded-lg font-black text-[10px] border-2 transition-all ${
                        capturingSlot === `support-${i}` ? 'hd-btn-recording' : 'bg-slate-900 border-slate-800 text-slate-500 hover:border-slate-700'
                      }`}
                    >
                      {capturingSlot === `support-${i}` ? '...' : (settings.supportShortcuts?.[i] || 'VINCULAR')}
                    </button>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

      </main>

      {/* FOOTER */}
      {activeTab === 'macro' && (
        <footer className="shrink-0 fixed bottom-0 left-0 right-0 bg-slate-950/90 border-t border-slate-900 p-8 backdrop-blur-xl z-[100] shadow-[0_-20px_50px_rgba(0,0,0,0.5)]">
          <div className="max-w-5xl mx-auto flex justify-center gap-8">
            {slots.map((slot, index) => (
              <Slot 
                key={index} index={index} selectedStratagem={slot}
                isActive={activeSlot === index} onSelectSlot={setActiveSlot}
                shortcut={settings.shortcuts[index]}
              />
            ))}
          </div>
        </footer>
      )}

    </div>
  )
}

export default App
