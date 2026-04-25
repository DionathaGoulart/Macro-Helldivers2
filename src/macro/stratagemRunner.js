const { keyboard, Key } = require('@nut-tree-fork/nut-js')

keyboard.config.autoDelayMs = 1
const delay = (ms) => new Promise(r => setTimeout(r, ms))
let isRunning = false

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

async function runStratagem(codex, modifierKey = 'LeftControl', useArrows = false) {
  if (isRunning) return
  isRunning = true
  const activeMap = useArrows ? arrowMap : wasdMap
  let modKey = nutKeyMap[modifierKey] || Key[modifierKey] || Key.LeftControl

  try {
    await keyboard.pressKey(modKey)
    await delay(100)
    for (const action of codex) {
      const key = activeMap[action.toUpperCase()]
      if (key) {
        await keyboard.pressKey(key)
        await delay(20)
        await keyboard.releaseKey(key)
        await delay(20)
      }
    }
    await delay(50)
    await keyboard.releaseKey(modKey)
  } catch (err) {
    try { await keyboard.releaseKey(modKey) } catch (_) { }
  } finally {
    isRunning = false
  }
}

module.exports = { runStratagem }
