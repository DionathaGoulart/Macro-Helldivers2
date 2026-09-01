// Regras e helpers compartilhados entre a aba de macro e a aba de builds.
// Ficam fora do App pra que a aba de builds possa virar um chunk separado.

// Tags que permitem apenas 1 estratagema equipado por vez (Mecha = exos, Vehicle = FRVs)
export const EXCLUSIVE_TAGS = ['Mecha', 'Vehicle']

export const EQUIPMENT_SLOTS = ['primary', 'secondary', 'grenade', 'armor', 'helmet', 'cape', 'booster']

export const hasExclusiveConflict = (strat, slots, activeSlot) =>
  EXCLUSIVE_TAGS.some(tag =>
    strat.tag?.includes(tag) &&
    slots.some((s, i) => i !== activeSlot && s?.tag?.includes(tag))
  )

// Busca sem acento e sem case: "orbital" acha "Orbital", "gatling" acha "A/G-16 Gatling"
export const normalizeText = (s) => s.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '')
