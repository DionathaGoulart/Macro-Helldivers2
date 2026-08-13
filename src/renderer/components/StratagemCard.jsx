import { memo } from 'react'
import { ArrowIcon } from './Slot'

const TAG_HOVER_CLASSES = {
  Offensive: 'hover:border-red-500/50 hover:shadow-[0_0_30px_rgba(239,68,68,0.15)]',
  Defensive: 'hover:border-green-500/50 hover:shadow-[0_0_30px_rgba(34,197,94,0.15)]',
  Supply: 'hover:border-cyan-500/50 hover:shadow-[0_0_30px_rgba(6,182,212,0.15)]'
}

const StratagemCard = memo(function StratagemCard({ strat, tag, isInActiveSlot, disabled, clearLabel, onAssign }) {
  const hoverClasses = TAG_HOVER_CLASSES[tag] || TAG_HOVER_CLASSES.Supply

  return (
    <button
      onClick={() => !disabled && onAssign(strat)}
      title={isInActiveSlot ? clearLabel : strat.nome}
      className={`group relative aspect-square rounded-2xl border-2 overflow-hidden
        ${disabled
          ? 'bg-slate-950/50 border-slate-900 opacity-20 cursor-not-allowed'
          : isInActiveSlot
            ? 'bg-slate-900/40 border-yellow-500/60 shadow-[0_0_20px_rgba(234,179,8,0.15)]'
            : `bg-slate-900/40 border-slate-800/50 ${hoverClasses}`}`}
    >
      {/* Stratagem Icon - Full bleed.
          Sem will-change/transform-gpu: promoviam TODOS os ~91 cards a camadas de
          composição permanentes só por causa do scale do hover. O transform do
          :hover já promove sob demanda, e só o card sob o cursor. */}
      <img
        src={strat.imagem}
        alt={strat.nome}
        decoding="async"
        loading="lazy"
        className="w-full h-full object-cover opacity-70 group-hover:scale-110 group-hover:opacity-100 transition-all duration-500"
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
})

export default StratagemCard
