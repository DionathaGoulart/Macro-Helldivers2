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
    <div className="h-screen flex flex-col bg-[#11141a] text-slate-100">
      
      {/* NAVBAR: SPACIOUS & CLEAR */}
      <nav className="hd-navbar shrink-0 flex items-center justify-between">
        <div className="flex items-center gap-6">
          <div className="w-10 h-10 bg-yellow-500 rounded-lg flex items-center justify-center shadow-xl shadow-yellow-500/20">
            <span className="text-slate-950 font-black text-xl italic">H</span>
          </div>
          <div className="flex flex-col">
            <span className="text-sm font-black uppercase tracking-widest text-white">Macro Helldivers 2</span>
            <span className="text-[10px] font-bold text-slate-500 uppercase tracking-widest">Protocolo de Combate</span>
          </div>
        </div>

        <div className="flex h-full ml-auto">
          <button 
            onClick={() => setActiveTab('macro')}
            className={`hd-nav-link mr-2 ${activeTab === 'macro' ? 'hd-nav-active' : 'hd-nav-inactive'}`}
          >
            Mapeamento
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
          <div className="max-w-5xl mx-auto space-y-12">
            {['Offensive', 'Supply', 'Defensive'].map((tag) => stratagemsByTag[tag] && (
              <section key={tag} className="animate-fade-in">
                <div className="hd-card-header">
                  <div className="hd-indicator"></div>
                  Estratagemas: {tag}
                </div>
                
                <div className="grid grid-cols-4 gap-6">
                  {stratagemsByTag[tag].sort((a,b) => a.id - b.id).map((strat) => {
                    const isEquipped = slots.some(s => s && s.id === strat.id)
                    const disabled = isEquipped || (strat.tag?.includes('Mecha') && slots.some((s, i) => i !== activeSlot && s?.tag?.includes('Mecha')))

                    return (
                      <button
                        key={strat.id}
                        onClick={() => !disabled && handleAssignStratagem(strat)}
                        className={`group relative flex flex-col items-center p-5 rounded-xl border-2 transition-all gap-4
                          ${disabled 
                            ? 'bg-transparent border-slate-900 opacity-20 cursor-not-allowed' 
                            : 'bg-[#1c212b] border-[#2d3646] hover:bg-[#232936] hover:border-yellow-500/50 hover:-translate-y-1 shadow-lg'}`}
                      >
                        <img src={strat.imagem} alt={strat.nome} className="w-12 h-12 object-contain drop-shadow-2xl group-hover:scale-110 transition-transform" />
                        <span className="text-[11px] font-black uppercase text-slate-300 leading-tight text-center h-4">{strat.nome}</span>
                        
                        <div className="flex gap-1.5 bg-black/30 p-2 rounded border border-slate-800">
                          {strat.codex.map((dir, i) => <ArrowIcon key={i} direction={dir} size={11} />)}
                        </div>

                        {isEquipped && (
                          <div className="absolute top-3 right-3">
                            <div className="w-2.5 h-2.5 rounded-full bg-yellow-500 shadow-[0_0_8px_rgba(251,191,36,0.5)]"></div>
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
          <div className="max-w-4xl mx-auto space-y-10 animate-fade-in">
            
            <div className="grid grid-cols-2 gap-10">
              {/* ATALHOS */}
              <div className="hd-card p-8">
                <h2 className="hd-card-header">
                  <div className="hd-indicator"></div>
                  Teclas de Atalho
                </h2>
                <div className="grid grid-cols-2 gap-5">
                  {[0, 1, 2, 3].map(i => (
                    <div key={i} className="space-y-3">
                      <label className="text-[10px] font-black text-slate-500 uppercase tracking-widest">Slot {i + 1}</label>
                      <button
                        onClick={async () => {
                          await window.api.invoke('set-recording-mode', true)
                          setCapturingSlot(i)
                        }}
                        className={`w-full py-4 rounded-lg font-black text-xs border-2 transition-all ${
                          capturingSlot === i ? 'hd-btn-recording' : 'bg-[#0d1015] border-slate-800 text-slate-200 hover:border-slate-600'
                        }`}
                      >
                        {capturingSlot === i ? '???' : settings.shortcuts[i]}
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              {/* OPÇÕES */}
              <div className="hd-card p-8">
                <h2 className="hd-card-header">
                  <div className="hd-indicator"></div>
                  Preferências de Jogo
                </h2>
                <div className="space-y-8">
                  <div className="space-y-4">
                    <label className="text-[10px] font-black text-slate-500 uppercase tracking-widest">Tecla para Abrir Menu (Segurar)</label>
                    <div className="grid grid-cols-2 gap-3">
                      {['LeftControl', 'LeftAlt', 'Equal', 'Minus'].map((key) => (
                        <button
                          key={key}
                          onClick={() => handleSettingChange('modifierKey', key)}
                          className={`py-3 rounded-lg text-[11px] font-black uppercase transition-all border-2 ${
                            settings.modifierKey === key ? 'bg-yellow-500 border-yellow-600 text-slate-950' : 'bg-[#0d1015] border-slate-800 text-slate-400 hover:border-slate-600'
                          }`}
                        >
                          {key === 'LeftControl' ? 'CTRL' : key === 'LeftAlt' ? 'ALT' : key}
                        </button>
                      ))}
                    </div>
                  </div>

                  <div className="flex items-center justify-between p-5 bg-[#0d1015] rounded-xl border border-slate-800">
                    <div className="flex flex-col">
                      <span className="text-[11px] font-black uppercase text-slate-200">Digitação Segura (Setas)</span>
                      <span className="text-[10px] text-slate-500 uppercase">{settings.useArrows ? 'Ativado' : 'Desativado (WASD)'}</span>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input type="checkbox" checked={settings.useArrows} onChange={(e) => handleSettingChange('useArrows', e.target.checked)} className="sr-only peer" />
                      <div className="w-11 h-6 bg-slate-800 rounded-full peer peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[4px] after:left-[4px] after:bg-slate-500 after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-yellow-500 peer-checked:after:bg-slate-950"></div>
                    </label>
                  </div>
                </div>
              </div>
            </div>

            {/* APOIO */}
            <div className="hd-card p-8">
              <h2 className="hd-card-header">
                <div className="hd-indicator bg-green-500 shadow-[0_0_12px_rgba(34,197,94,0.4)]"></div>
                Estratagemas de Suporte
              </h2>
              <div className="grid grid-cols-3 gap-8">
                {[
                  { nome: 'Reinforce', imagem: '/Reinforce_Stratagem_Icon.png', codex: ['UP', 'DOWN', 'RIGHT', 'LEFT', 'UP'] },
                  { nome: 'Resupply', imagem: '/Resupply_Stratagem_Icon.png', codex: ['DOWN', 'DOWN', 'UP', 'RIGHT'] },
                  { nome: 'Eagle Rearm', imagem: '/Eagle_Rearm_Stratagem_Icon.png', codex: ['UP', 'UP', 'LEFT', 'UP', 'RIGHT'] }
                ].map((strat, i) => (
                  <div key={i} className="bg-[#0d1015] p-6 rounded-xl border border-slate-800 space-y-4 hover:border-slate-700 transition-colors">
                    <div className="flex items-center justify-between">
                      <img src={strat.imagem} alt={strat.nome} className="w-10 h-10 object-contain" />
                      <div className="flex gap-1.5">
                        {strat.codex.map((dir, idx) => <ArrowIcon key={idx} direction={dir} size={12} />)}
                      </div>
                    </div>
                    <div className="text-[11px] font-black uppercase text-slate-400 tracking-widest">{strat.nome}</div>
                    <button
                      onClick={async () => {
                        await window.api.invoke('set-recording-mode', true)
                        setCapturingSlot(`support-${i}`)
                      }}
                      className={`w-full py-3 rounded-lg font-black text-[10px] border-2 transition-all ${
                        capturingSlot === `support-${i}` ? 'hd-btn-recording' : 'bg-[#1c212b] border-slate-800 text-slate-500 hover:border-slate-600'
                      }`}
                    >
                      {capturingSlot === `support-${i}` ? 'AGUARDANDO...' : (settings.supportShortcuts?.[i] || 'VINCULAR TECLA')}
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
        <footer className="shrink-0 fixed bottom-0 left-0 right-0 bg-[#161a21]/95 border-t border-slate-800 p-8 backdrop-blur-xl z-[100] shadow-[0_-20px_50px_rgba(0,0,0,0.5)]">
          <div className="max-w-5xl mx-auto flex flex-col items-center gap-6">
            <div className="flex justify-center gap-8">
              {slots.map((slot, index) => (
                <Slot 
                  key={index} index={index} selectedStratagem={slot}
                  isActive={activeSlot === index} onSelectSlot={setActiveSlot}
                  shortcut={settings.shortcuts[index]}
                />
              ))}
            </div>
          </div>
        </footer>
      )}

    </div>
  )
}

export default App
