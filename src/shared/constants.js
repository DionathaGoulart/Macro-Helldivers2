// Estratagemas de apoio fixo, compartilhados entre o processo main e o renderer
export const SUPPORT_CODEXES = [
  ['UP', 'DOWN', 'RIGHT', 'LEFT', 'UP'],  // Reinforce
  ['DOWN', 'DOWN', 'UP', 'RIGHT'],        // Resupply
  ['UP', 'UP', 'LEFT', 'UP', 'RIGHT']     // Eagle Rearm
]

export const SUPPORT_STRATS = [
  { nome: 'Reinforce', imagem: 'Reinforce_Stratagem_Icon.webp', codex: SUPPORT_CODEXES[0] },
  { nome: 'Resupply', imagem: 'Resupply_Stratagem_Icon.webp', codex: SUPPORT_CODEXES[1] },
  { nome: 'Eagle Rearm', imagem: 'Eagle_Rearm_Stratagem_Icon.webp', codex: SUPPORT_CODEXES[2] }
]
