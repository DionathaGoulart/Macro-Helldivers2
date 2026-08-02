const { keyboard, Key } = require('@nut-tree-fork/nut-js')

const delay = (ms) => new Promise(r => setTimeout(r, ms))
// Jitter humanizado: ±5ms pra os intervalos não saírem roboticamente exatos
const jitter = (ms) => Math.max(1, ms + Math.random() * 10 - 5)
let isRunning = false

// auto = delay interno do nut-js por press/release; hold = tecla segurada;
// gap = pausa entre teclas; lead = após segurar o modificador; tail = antes de soltá-lo
const SPEED_PROFILES = {
  normal: { auto: 10, hold: 20, gap: 20, lead: 100, tail: 50 },
  fast:   { auto: 5,  hold: 15, gap: 15, lead: 70,  tail: 40 },
  turbo:  { auto: 1,  hold: 10, gap: 10, lead: 50,  tail: 30 },
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
        await delay(jitter(p.hold))
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
