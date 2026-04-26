import { useState, useEffect } from 'react'
import { ArrowUp, ArrowDown, ArrowLeft, ArrowRight } from 'lucide-react'

export const ArrowIcon = ({ direction, className = "", size = 16 }) => {
  const iconProps = { size, className: `text-yellow-500/90 ${className}` }
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
      className={`relative w-16 h-16 border-2 flex flex-col items-center justify-center gap-1 rounded-xl transition-all duration-300
        ${activeVisual 
          ? 'bg-yellow-500/40 border-yellow-400 scale-105 shadow-[0_0_20px_rgba(251,191,36,0.3)]' 
          : isActive 
            ? 'bg-slate-800/80 border-yellow-500 shadow-[0_0_15px_rgba(251,191,36,0.15)] scale-105 z-10' 
            : 'bg-slate-900/40 border-slate-800 hover:border-slate-600 hover:bg-slate-800/50'}
      `}
    >
      {/* Shortcut Indicator */}
      <div className={`absolute -top-2 px-1.5 py-0.5 rounded border text-[9px] font-black tracking-tighter transition-colors
        ${isActive ? 'bg-yellow-500 border-yellow-600 text-slate-950' : 'bg-slate-950 border-slate-700 text-slate-500'}
      `}>
        {shortcut || `F${index + 1}`}
      </div>

      {selectedStratagem ? (
        <div className="flex flex-col items-center gap-1">
          <img 
            src={`${selectedStratagem.imagem}`} 
            alt={selectedStratagem.nome} 
            className={`w-7 h-7 object-contain transition-transform duration-300 ${activeVisual ? 'scale-110' : 'scale-100'}`}
          />
          <div className="flex gap-0.5 px-1 py-0.5 rounded-sm bg-slate-950/40 border border-slate-800/50 scale-[0.7]">
            {selectedStratagem.codex.map((dir, i) => <ArrowIcon key={i} direction={dir} size={12} />)}
          </div>
        </div>
      ) : (
        <div className="flex flex-col items-center gap-1 opacity-20">
          <div className="w-6 h-6 border-2 border-dashed border-slate-500 rounded-md" />
          <span className="text-[8px] font-bold uppercase tracking-widest">Open</span>
        </div>
      )}

      {/* Active Line Indicator */}
      {isActive && (
        <div className="absolute -bottom-1 w-8 h-1 bg-yellow-500 rounded-full" />
      )}
    </button>
  )
}
