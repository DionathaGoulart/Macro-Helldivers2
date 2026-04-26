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
  const [activeTab, setActiveTab] = useState('macro') // 'macro' ou 'settings'
  const [slots, setSlots] = useState([null, null, null, null])
  const [activeSlot, setActiveSlot] = useState(0)
  const [capturingSlot, setCapturingSlot] = useState(null) // null | 0-3 | 'modifier'

  const [settings, setSettings] = useState({
    shortcuts: ['F1', 'F2', 'F3', 'F4'],
    supportShortcuts: [null, null, null],
    modifierKey: 'LeftControl',
    useArrows: false
  })

  // Load from localStorage on mount
  useEffect(() => {
    const savedSlots = localStorage.getItem('helldivers-macro-slots')
    if (savedSlots) {
      try {
        const parsed = JSON.parse(savedSlots)
        setSlots(parsed)
        if (window.api) window.api.updateSlots(parsed)
      } catch (e) {
        console.error('Failed to parse saved slots', e)
      }
    }

    const savedSettings = localStorage.getItem('helldivers-macro-settings')
    if (savedSettings) {
      try {
        const parsed = JSON.parse(savedSettings)
        setSettings(prev => ({ ...prev, ...parsed }))
        if (window.api) window.api.saveSettings({ ...settings, ...parsed })
      } catch (e) {
        console.error('Failed to parse saved settings', e)
      }
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

    if (activeSlot < 3) {
      setActiveSlot(activeSlot + 1)
    }
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

  // Key capture logic
  const startCapture = (target) => {
    setCapturingSlot(target)
  }

  const handleKeyCapture = useCallback((e) => {
    if (capturingSlot === null) return
    e.preventDefault()
    e.stopPropagation()

    // Ignorar teclas de modificação puras (capturamos na tecla real)
    if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return
    if (e.key === 'Escape') { setCapturingSlot(null); return }

    let key = e.code
    if (key.startsWith('Key')) key = key.replace('Key', '')
    else if (key.startsWith('Digit')) key = key.replace('Digit', '')
    const mapped = keyMap[e.key] || keyMap[key] || key

    if (typeof capturingSlot === 'number') {
      handleShortcutChange(capturingSlot, mapped)
    } else if (typeof capturingSlot === 'string' && capturingSlot.startsWith('support-')) {
      const index = parseInt(capturingSlot.split('-')[1])
      handleSupportShortcutChange(index, mapped)
    }
    setCapturingSlot(null)
    window.api.invoke('set-recording-mode', false)
  }, [capturingSlot])

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
        window.api.invoke('set-recording-mode', false)
      }

      window.addEventListener('keydown', keyHandler, true)
      return () => window.removeEventListener('keydown', keyHandler, true)
    }
  }, [capturingSlot, handleKeyCapture])

  // Agrupar Stratagems por Tag
  const stratagemsByTag = {}
  stratagemsData.forEach(strat => {
    const tag = (strat.tag && strat.tag.length > 0) ? strat.tag[0] : 'Outros'
    if (!stratagemsByTag[tag]) stratagemsByTag[tag] = []
    stratagemsByTag[tag].push(strat)
  })

  const orderedTags = ['Offensive', 'Supply', 'Defensive']
  const sortedTags = Object.keys(stratagemsByTag).sort((a, b) => {
    const indexA = orderedTags.indexOf(a)
    const indexB = orderedTags.indexOf(b)
    if (indexA !== -1 && indexB !== -1) return indexA - indexB
    if (indexA !== -1) return -1
    if (indexB !== -1) return 1
    return a.localeCompare(b)
  })

  Object.values(stratagemsByTag).forEach(strats => {
    strats.sort((a, b) => a.id - b.id)
  })

  return (
    <div className="h-screen flex flex-col bg-slate-900 overflow-hidden">

      {/* HEADER */}
      <header className="pt-2 pb-0 text-center shrink-0 border-b border-slate-800">
        {/* TABS */}
        <div className="flex justify-center gap-4">
          <button
            onClick={() => setActiveTab('macro')}
            className={`px-6 py-3 rounded-t-lg font-bold text-sm transition-colors border-b-2 ${activeTab === 'macro' ? 'bg-slate-800/80 text-yellow-400 border-yellow-500' : 'text-slate-500 border-transparent hover:bg-slate-800/50 hover:text-slate-300'}`}
          >
            Configurar Macros
          </button>
          <button
            onClick={() => setActiveTab('settings')}
            className={`px-6 py-3 rounded-t-lg font-bold text-sm transition-colors border-b-2 ${activeTab === 'settings' ? 'bg-slate-800/80 text-yellow-400 border-yellow-500' : 'text-slate-500 border-transparent hover:bg-slate-800/50 hover:text-slate-300'}`}
          >
            Configurações
          </button>
        </div>
      </header>

      {/* CENTER: CONTEÚDO DA ABA */}
      <main className="flex-1 overflow-y-auto px-8 pb-4 scrollbar-thin scrollbar-thumb-slate-700 scrollbar-track-transparent">

        {activeTab === 'macro' && (
          <div className="max-w-4xl mx-auto space-y-8 mt-6">
            <div className="text-center mb-4">
              <span className="bg-slate-800 text-yellow-400 text-xs px-3 py-1 rounded-full border border-yellow-500/30">
                Selecione um Stratagem abaixo para equipar no Slot {settings.shortcuts[activeSlot] || `F${activeSlot + 1}`}
              </span>
            </div>

            {sortedTags.map((tag) => (
              <div key={tag} className="bg-slate-800/30 p-4 rounded-xl border border-slate-700/50">
                <h2 className="text-lg font-bold text-slate-300 uppercase tracking-wider mb-4 flex items-center gap-2">
                  <div className="w-2 h-2 bg-yellow-500 rounded-full"></div>
                  {tag}
                </h2>

                <div className="grid grid-cols-4 gap-3">
                  {stratagemsByTag[tag].map((strat, idx) => {
                    const isEquipped = slots.some(s => s && s.id === strat.id)
                    const isMecha = strat.tag && strat.tag.includes('Mecha')
                    const hasOtherMecha = slots.some((s, i) => i !== activeSlot && s && s.tag && s.tag.includes('Mecha'))
                    const disabled = isEquipped || (isMecha && hasOtherMecha)

                    return (
                      <button
                        key={idx}
                        onClick={() => !disabled && handleAssignStratagem(strat)}
                        className={`flex flex-col items-center p-3 transition-all rounded-lg text-center gap-2 border 
                          ${disabled
                            ? 'bg-slate-900 border-slate-800 opacity-40 cursor-not-allowed'
                            : 'bg-slate-800 hover:bg-slate-700 border-slate-600 hover:border-yellow-500/50 cursor-pointer'}`}
                      >
                        <img src={`${strat.imagem}`} alt={strat.nome} className="w-10 h-10 object-contain drop-shadow-md" />
                        <span className="text-xs font-semibold text-slate-300 leading-tight">{strat.nome}</span>
                        <div className="flex gap-1 mt-auto bg-slate-900/60 px-1.5 py-1 rounded shadow-inner">
                          {strat.codex.map((dir, i) => <ArrowIcon key={i} direction={dir} size={14} />)}
                        </div>
                      </button>
                    )
                  })}
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'settings' && (
          <div className="max-w-2xl mx-auto space-y-6 mt-8">
            <div className="bg-slate-800/40 p-6 rounded-xl border border-slate-700/50">
              <h2 className="text-xl font-bold text-yellow-500 mb-6 border-b border-slate-700/50 pb-3 flex items-center gap-2">
                <div className="w-2 h-2 bg-yellow-500 rounded-full"></div>
                Teclas de Atalho
              </h2>
              <div className="grid grid-cols-2 gap-6">
                {[0, 1, 2, 3].map(i => (
                  <div key={i} className="flex flex-col gap-2">
                    <label className="text-sm font-semibold text-slate-300">Slot {i + 1}</label>
                    <button
                      onClick={async () => {
                        await window.api.invoke('set-recording-mode', true)
                        setCapturingSlot(i)
                      }}
                      className={`px-4 py-3 rounded-lg font-mono text-sm font-bold tracking-widest border-2 transition-all text-center ${capturingSlot === i
                          ? 'bg-yellow-500/20 border-yellow-400 text-yellow-300 animate-pulse'
                          : 'bg-slate-900 border-slate-600 text-slate-200 hover:border-yellow-500/60 hover:bg-slate-800'
                        }`}
                    >
                      {capturingSlot === i ? '🎯 Pressione uma tecla...' : settings.shortcuts[i]}
                    </button>
                  </div>
                ))}
              </div>
              <p className="text-xs text-slate-500 mt-5">
                Clique num slot e pressione a tecla desejada. <kbd className="bg-slate-900 px-1.5 py-0.5 rounded border border-slate-600 text-[10px]">Esc</kbd> cancela.
              </p>
            </div>

            <div className="bg-slate-800/40 p-6 rounded-xl border border-slate-700/50">
              <h2 className="text-xl font-bold text-yellow-500 mb-6 border-b border-slate-700/50 pb-3 flex items-center gap-2">
                <div className="w-2 h-2 bg-yellow-500 rounded-full"></div>
                Jogo
              </h2>
              <div className="flex flex-col gap-3">
                <label className="text-sm font-semibold text-slate-300">Tecla de Menu (Estratagemas)</label>
                <div className="flex flex-wrap gap-2">
                  {[
                    { id: 'LeftControl', label: 'Ctrl' },
                    { id: 'LeftAlt', label: 'Alt' },
                    { id: 'Equal', label: '=' },
                    { id: 'Minus', label: '-' }
                  ].map((btn) => (
                    <button
                      key={btn.id}
                      onClick={() => handleSettingChange('modifierKey', btn.id)}
                      className={`px-6 py-3 rounded-lg border transition-all font-bold ${settings.modifierKey === btn.id
                          ? 'bg-yellow-500 border-yellow-400 text-slate-900 shadow-lg shadow-yellow-500/20'
                          : 'bg-slate-900 border-slate-700 text-slate-400 hover:border-slate-500 hover:bg-slate-800'
                        }`}
                    >
                      {btn.label}
                    </button>
                  ))}
                </div>
                <p className="text-xs text-slate-500 mt-1">
                  Selecione a tecla que você usa no jogo para abrir o menu de estratagemas.
                </p>
              </div>

              <div className="flex flex-col gap-4 mt-6 pt-6 border-t border-slate-700/50">
                <div className="flex items-center justify-between">
                  <div className="flex flex-col gap-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-semibold text-slate-200">Usar Setas para as Estratagemas</span>
                      <span className="bg-yellow-500/20 text-yellow-500 text-[9px] font-bold px-1.5 py-0.5 rounded border border-yellow-500/30 uppercase tracking-wider">Recomendado</span>
                    </div>
                    <span className="text-[10px] text-slate-500">O padrão WASD pode interferir no movimento.</span>
                  </div>
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.useArrows}
                      onChange={(e) => handleSettingChange('useArrows', e.target.checked)}
                      className="sr-only peer"
                    />
                    <div className="w-11 h-6 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-yellow-500"></div>
                  </label>
                </div>
              </div>
            </div>

            <div className="bg-slate-800/40 p-6 rounded-xl border border-slate-700/50">
              <h2 className="text-xl font-bold text-yellow-500 mb-6 border-b border-slate-700/50 pb-3 flex items-center gap-2">
                <div className="w-2 h-2 bg-yellow-500 rounded-full"></div>
                Estratagemas de Suporte
              </h2>
              <div className="grid grid-cols-2 gap-4">
                {[
                  { nome: 'Reinforce', imagem: '/Reinforce_Stratagem_Icon.png', codex: ['UP', 'DOWN', 'RIGHT', 'LEFT', 'UP'] },
                  { nome: 'Resupply', imagem: '/Resupply_Stratagem_Icon.png', codex: ['DOWN', 'DOWN', 'UP', 'RIGHT'] },
                  { nome: 'Eagle Rearm', imagem: '/Eagle_Rearm_Stratagem_Icon.png', codex: ['UP', 'UP', 'LEFT', 'UP', 'RIGHT'] }
                ].map((strat, i) => (
                  <div key={i} className="flex flex-col bg-slate-900/60 p-4 rounded-lg border border-slate-700/50 hover:border-slate-600 transition-all group gap-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <img src={strat.imagem} alt={strat.nome} className="w-8 h-8 object-contain drop-shadow-md group-hover:scale-110 transition-transform" />
                        <span className="text-sm font-bold text-slate-200">{strat.nome}</span>
                      </div>
                      <div className="flex gap-1">
                        {strat.codex.map((dir, idx) => (
                          <ArrowIcon key={idx} direction={dir} size={14} />
                        ))}
                      </div>
                    </div>
                    
                    <button
                      onClick={async () => {
                        await window.api.invoke('set-recording-mode', true)
                        setCapturingSlot(`support-${i}`)
                      }}
                      className={`w-full px-3 py-3 rounded-lg border-2 font-mono text-xs font-bold tracking-wider transition-all ${
                        capturingSlot === `support-${i}`
                          ? 'bg-yellow-500/20 border-yellow-400 text-yellow-300 animate-pulse'
                          : 'bg-slate-800 border-slate-600 text-slate-200 hover:border-yellow-500/60 hover:bg-slate-900'
                      }`}
                    >
                      {capturingSlot === `support-${i}` ? '🎯 Pressione...' : (settings.supportShortcuts && settings.supportShortcuts[i]) || 'NENHUMA'}
                    </button>
                  </div>
                ))}
              </div>
              <p className="text-xs text-slate-500 mt-5">
                Clique num slot e pressione a tecla desejada. <kbd className="bg-slate-900 px-1.5 py-0.5 rounded border border-slate-600 text-[10px]">Esc</kbd> cancela.
              </p>
            </div>

            <div className="bg-slate-800/40 p-6 rounded-xl border border-slate-700/50">
              <h2 className="text-xl font-bold text-yellow-500 mb-6 border-b border-slate-700/50 pb-3 flex items-center gap-2">
                <div className="w-2 h-2 bg-yellow-500 rounded-full"></div>
                Atualização
              </h2>
              <div className="flex items-center justify-between">
                <div className="flex flex-col gap-1">
                  <span className="text-sm font-semibold text-slate-200">Versão v0.1.0</span>
                </div>
                <div className="flex items-center gap-2 bg-green-500/10 border border-green-500/20 px-3 py-1.5 rounded-lg">
                  <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                  <span className="text-[10px] font-bold text-green-500 uppercase tracking-widest">Atualizado</span>
                </div>
              </div>
            </div>
          </div>
        )}

      </main>

      {/* BOTTOM: 4 SLOTS LADO A LADO */}
      {activeTab === 'macro' && (
        <footer className="shrink-0 bg-slate-900 border-t border-slate-800 p-3 pb-2 shadow-[0_-10px_40px_rgba(0,0,0,0.5)]">
          <div className="flex justify-center gap-4">
            {slots.map((slot, index) => (
              <Slot
                key={index}
                index={index}
                selectedStratagem={slot}
                isActive={activeSlot === index}
                onSelectSlot={setActiveSlot}
                shortcut={settings.shortcuts[index]}
              />
            ))}
          </div>
          <p className="text-center mt-2 text-[10px] text-slate-500">
            Aperte <strong className="text-yellow-500">{settings.shortcuts.join(', ')}</strong> para ativar os slots.
          </p>
        </footer>
      )}

    </div>
  )
}

export default App
