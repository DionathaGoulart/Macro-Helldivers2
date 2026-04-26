import { useState, useEffect } from 'react'
import { ArrowUp, ArrowDown, ArrowLeft, ArrowRight } from 'lucide-react'

export const ArrowIcon = ({ direction, className = "", size = 16 }) => {
  const icons = {
    UP: ArrowUp,
    DOWN: ArrowDown,
    LEFT: ArrowLeft,
    RIGHT: ArrowRight
  }
  const Icon = icons[direction.toUpperCase()]
  return Icon ? <Icon size={size} className={`text-yellow-500/90 ${className}`} /> : null
}

export default function Slot({ index, selectedStratagem, isActive, onSelectSlot, shortcut }) {
  const [activeVisual, setActiveVisual] = useState(false)

  useEffect(() => {
    if (window.api) {
      const removeListener = window.api.onMacroTriggered((triggeredIndex) => {
        if (triggeredIndex === index) {
          setActiveVisual(true)
          setTimeout(() => setActiveVisual(false), 500)
        }
      })
      return () => {
        if (removeListener) removeListener()
      }
    }
  }, [index])

  return (
    <button
      onClick={() => onSelectSlot(index)}
      className={`relative w-16 h-16 border-2 flex flex-col items-center justify-center transition-all duration-300 overflow-hidden rounded-xl
        ${activeVisual
          ? 'bg-yellow-500/40 border-yellow-400 scale-105 shadow-[0_0_20px_rgba(251,191,36,0.3)]'
          : isActive
            ? 'bg-slate-800/80 border-yellow-500 shadow-[0_0_15px_rgba(251,191,36,0.15)] scale-105 z-10'
            : 'bg-slate-900/40 border-slate-800 hover:border-slate-600 hover:bg-slate-800/50'}
      `}
    >
      {selectedStratagem ? (
        <div className="absolute inset-0 w-full h-full">
          {/* Full-bleed background image */}
          <img 
            src={`${selectedStratagem.imagem}`} 
            alt={selectedStratagem.nome} 
            className={`w-full h-full object-cover opacity-80 transition-transform duration-300 ${activeVisual ? 'scale-110' : 'scale-100'}`}
          />
          {/* Overlay for better readability of indicators */}
          <div className="absolute inset-x-0 top-0 h-10 bg-gradient-to-b from-slate-950/60 to-transparent"></div>
        </div>
      ) : (
        <div className="flex flex-col items-center gap-1 opacity-20">
          <div className="w-5 h-5 border-2 border-dashed border-slate-500 rounded-md" />
          <span className="text-[7px] font-black uppercase tracking-[0.2em]">Open</span>
        </div>
      )}

      {/* Shortcut Indicator - Always on top */}
      <div className={`absolute top-1 left-1/2 -translate-x-1/2 px-1.5 py-0.5 rounded border text-[9px] font-black tracking-tighter transition-colors z-30
        ${isActive ? 'bg-yellow-500 border-yellow-600 text-slate-950 shadow-[0_2px_10px_rgba(251,191,36,0.4)]' : 'bg-slate-950 border-slate-700 text-slate-500'}
      `}>
        {shortcut || `F${index + 1}`}
      </div>

      {/* Active Line Indicator */}
      {isActive && (
        <div className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-6 h-1 bg-yellow-500 rounded-full z-30" />
      )}
    </button>
  )
}
