const { keyboard, Key } = require('@nut-tree-fork/nut-js')

const delay = (ms) => new Promise(r => setTimeout(r, ms))
// Jitter humanizado: ±5ms pra os intervalos não saírem roboticamente exatos.
// O piso é obrigatório no `hold` (ver MIN_HOLD_MS) — sem ele o jitter derrubava
// o turbo pra 5ms e o input sumia.
const jitter = (ms, floor = 1) => Math.max(floor, ms + Math.random() * 10 - 5)
let isRunning = false

// O jogo lê teclado uma vez por frame: 16,7ms a 60fps, 33,3ms a 30fps. Tecla
// pressionada e solta entre dois polls simplesmente não existe pro jogo, e a macro
// falha de forma intermitente. Por isso o `hold` de cada perfil é medido em frames,
// e a velocidade vem de encurtar o `gap`, que não tem esse limite.
const MIN_HOLD_MS = 20

// auto = delay interno do nut-js por press/release; hold = tecla segurada;
// gap = pausa entre teclas; lead = após segurar o modificador; tail = antes de soltá-lo
const SPEED_PROFILES = {
  normal: { auto: 10, hold: 34, gap: 20, lead: 100, tail: 50 }, // ~1 frame a 30fps
  fast:   { auto: 5,  hold: 24, gap: 12, lead: 70,  tail: 40 }, // ~1,5 frame a 60fps
  turbo:  { auto: 1,  hold: 20, gap: 6,  lead: 50,  tail: 30 }, // ~1,2 frame a 60fps
}

const nutKeyMap = {
  'LeftControl': Key.LeftControl,
  'RightControl': Key.RightControl,
  'LeftShift': Key.LeftShift,
  'RightShift': Key.RightShift,
  'LeftAlt': Key.LeftAlt,
  'RightAlt': Key.RightAlt,
  'Equal': Key.Equal,
  'Minus': Key.Minus,
  'Up': Key.Up, 'Down': Key.Down, 'Left': Key.Left, 'Right': Key.Right,
  'Space': Key.Space, 'Escape': Key.Escape, 'Enter': Key.Enter, 'Tab': Key.Tab,
}

const arrowMap = { 'UP': Key.Up, 'DOWN': Key.Down, 'LEFT': Key.Left, 'RIGHT': Key.Right }
const wasdMap = { 'UP': Key.W, 'DOWN': Key.S, 'LEFT': Key.A, 'RIGHT': Key.D }

async function runStratagem(codex, modifierKey = 'LeftControl', useArrows = false, speed = 'normal') {
  if (isRunning) return
  isRunning = true
  const p = SPEED_PROFILES[speed] || SPEED_PROFILES.normal
  keyboard.config.autoDelayMs = p.auto
  const activeMap = useArrows ? arrowMap : wasdMap
  let modKey = nutKeyMap[modifierKey] || Key[modifierKey] || Key.LeftControl

  try {
    await keyboard.pressKey(modKey)
    await delay(jitter(p.lead))
    for (const action of codex) {
      const key = activeMap[action.toUpperCase()]
      if (key) {
        await keyboard.pressKey(key)
        await delay(jitter(p.hold, MIN_HOLD_MS))
        await keyboard.releaseKey(key)
        await delay(jitter(p.gap))
      }
    }
    await delay(jitter(p.tail))
    await keyboard.releaseKey(modKey)
  } catch (err) {
    try { await keyboard.releaseKey(modKey) } catch (_) { }
  } finally {
    isRunning = false
  }
}

module.exports = { runStratagem }
