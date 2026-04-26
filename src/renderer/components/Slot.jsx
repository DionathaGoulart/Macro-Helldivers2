import { useState, useEffect } from 'react'
import { ArrowUp, ArrowDown, ArrowLeft, ArrowRight } from 'lucide-react'

export const ArrowIcon = ({ direction, className = "", size = 16 }) => {
  const iconProps = { size, className: `text-yellow-400 ${className}` }
  switch(direction.toUpperCase()) {
    case 'UP': return <ArrowUp {...iconProps} />
    case 'DOWN': return <ArrowDown {...iconProps} />
    case 'LEFT': return <ArrowLeft {...iconProps} />
    case 'RIGHT': return <ArrowRight {...iconProps} />
    default: return null
  }
}

export default function Slot({ index, selectedStratagem, isActive, onSelectSlot, shortcut }) {
  const [activeVisual, setActiveVisual] = useState(false)

  // Listen to macro triggers for visual feedback
  useEffect(() => {
    if (window.api) {
      const handleTrigger = (triggeredIndex) => {
        if (triggeredIndex === index) {
          setActiveVisual(true)
          setTimeout(() => setActiveVisual(false), 500)
        }
      }
      window.api.onMacroTriggered(handleTrigger)
    }
  }, [index])

  return (
    <button 
      onClick={() => onSelectSlot(index)}
      className={`relative w-14 h-14 border flex flex-col items-center justify-center gap-0.5 rounded-lg transition-all duration-200
        ${activeVisual ? 'bg-yellow-500/30 border-yellow-400 scale-105' : 
          isActive ? 'bg-slate-700 border-yellow-500 ring-1 ring-yellow-500/50 shadow-md shadow-yellow-500/10' : 
          'bg-slate-800/80 hover:bg-slate-700 border-slate-600'}
      `}
    >
      <span className="absolute -top-2 bg-slate-900 px-1 text-[8px] font-bold rounded border border-slate-700 text-yellow-500">
        {shortcut || `F${index + 1}`}
      </span>
      {selectedStratagem ? (
        <>
          <img src={`${selectedStratagem.imagem}`} alt={selectedStratagem.nome} className="w-6 h-6 object-contain drop-shadow-sm" />
          <div className="flex gap-0.5 bg-slate-900/80 px-0.5 py-0.5 rounded scale-[0.6]">
            {selectedStratagem.codex.map((dir, i) => <ArrowIcon key={i} direction={dir} />)}
          </div>
        </>
      ) : (
        <span className="text-[9px] text-slate-600 font-medium">Vazio</span>
      )}
    </button>
  )
}
