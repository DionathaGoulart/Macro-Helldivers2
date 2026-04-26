const { app, BrowserWindow, globalShortcut, ipcMain, Tray, Menu } = require('electron')
const path = require('path')
const fs = require('fs')
const { autoUpdater } = require('electron-updater')

// Configurações Globais
let win
let tray
let isQuitting = false
let isRecordingState = false
let currentSlots = [null, null, null, null]
let isGameFocused = false
let nut = null // Carregamento tardio (Lazy)

const isDev = !!process.env.VITE_DEV_SERVER_URL

// SINGLE INSTANCE LOCK
const gotTheLock = app.requestSingleInstanceLock()

if (!gotTheLock) {
  app.quit()
} else {
  app.on('second-instance', (event, commandLine, workingDirectory) => {
    // Alguém tentou abrir uma segunda instância, focamos e restauramos nossa janela.
    if (win) {
      if (win.isMinimized()) win.restore()
      win.show()
      win.focus()
    }
  })
}

// Motor de Macro (Carregamento Seguro)
function loadMacroEngine() {
  try {
    if (nut) return nut
    // Carregamos os módulos nativos apenas quando necessário
    const nutjs = require('@nut-tree-fork/nut-js')
    const { runStratagem } = require('../macro/stratagemRunner.js')
    nut = { ...nutjs, runStratagem }
    console.log('Motor de macro carregado com sucesso.')
    return nut
  } catch (e) {
    console.error('Falha crítica ao carregar motor nativo:', e)
    return null
  }
}

const SUPPORT_CODEXES = [
  ['Up', 'Down', 'Right', 'Left', 'Up'],     // Reinforce
  ['Down', 'Down', 'Up', 'Right'],           // Resupply
  ['Up', 'Up', 'Left', 'Up', 'Right']        // Eagle Rearm
]

const DEFAULT_SETTINGS = {
  shortcuts: ['F1', 'F2', 'F3', 'F4'],
  supportShortcuts: [null, null, null],
  modifierKey: 'LeftControl',
  useArrows: false
}

function getSettingsPath() {
  const userDataPath = app.getPath('userData')
  const settingsDir = path.join(userDataPath, 'Helldivers Macro')
  if (!fs.existsSync(settingsDir)) fs.mkdirSync(settingsDir, { recursive: true })
  return path.join(settingsDir, 'settings.json')
}

function saveSettings(settings) {
  fs.writeFileSync(getSettingsPath(), JSON.stringify(settings, null, 2))
}

function loadSettings() {
  const p = getSettingsPath()
  if (fs.existsSync(p)) {
    try { return JSON.parse(fs.readFileSync(p, 'utf8')) } catch (e) { return DEFAULT_SETTINGS }
  }
  return DEFAULT_SETTINGS
}

let currentSettings = loadSettings()

// Polling de Foco (Seguro)
setInterval(async () => {
  const engine = loadMacroEngine()
  if (!engine || !engine.getActiveWindow) return

  try {
    const activeWindow = await engine.getActiveWindow()
    if (!activeWindow) {
      if (isGameFocused) {
        isGameFocused = false
        if (win && !win.isDestroyed()) win.webContents.send('game-focus-changed', false)
      }
      return
    }
    
    // Suporte a propriedade ou função para compatibilidade com diferentes versões do nut-js
    const title = typeof activeWindow.title === 'function' ? await activeWindow.title() : await activeWindow.title
    
    if (!title) {
      if (isGameFocused) {
        isGameFocused = false
        if (win && !win.isDestroyed()) win.webContents.send('game-focus-changed', false)
      }
      return
    }

    // Busca mais flexível pelo título do jogo
    const isFocused = title.toUpperCase().includes('HELLDIVERS')
    
    if (isFocused !== isGameFocused) {
      isGameFocused = isFocused
      if (win && !win.isDestroyed()) {
        win.webContents.send('game-focus-changed', isGameFocused)
      }
    }
  } catch (e) {
    // Se der erro, por segurança desativamos para não disparar macro em apps errados
    if (isGameFocused) {
      isGameFocused = false
      if (win && !win.isDestroyed()) win.webContents.send('game-focus-changed', false)
    }
  }
}, 1500)

async function handleShortcutTrigger(index) {
  if (isRecordingState || !isGameFocused) return
  const engine = loadMacroEngine()
  if (!engine) return

  const stratagem = currentSlots[index]
  if (stratagem && stratagem.codex) {
    if (win && !win.isDestroyed()) win.webContents.send('macro-triggered', index)
    try {
      await engine.runStratagem(stratagem.codex, currentSettings.modifierKey, currentSettings.useArrows)
    } catch (e) {
      console.error('Erro ao executar macro:', e)
    }
  }
}

async function handleSupportTrigger(index) {
  if (isRecordingState || !isGameFocused) return
  const engine = loadMacroEngine()
  if (!engine) return

  const codex = SUPPORT_CODEXES[index]
  if (codex) {
    if (win && !win.isDestroyed()) win.webContents.send('support-macro-triggered', index)
    try {
      await engine.runStratagem(codex, currentSettings.modifierKey, currentSettings.useArrows)
    } catch (e) {
      console.error('Erro ao executar macro de suporte:', e)
    }
  }
}

function registerMacros() {
  if (isRecordingState) return
  globalShortcut.unregisterAll()
  
  // Slots normais
  currentSettings.shortcuts.forEach((key, index) => {
    if (!key) return
    try {
      globalShortcut.register(key, () => handleShortcutTrigger(index))
      globalShortcut.register(`Shift+${key}`, () => handleShortcutTrigger(index))
    } catch (e) {}
  })

  // Slots de suporte
  if (currentSettings.supportShortcuts) {
    currentSettings.supportShortcuts.forEach((key, index) => {
      if (!key) return
      try {
        globalShortcut.register(key, () => handleSupportTrigger(index))
        globalShortcut.register(`Shift+${key}`, () => handleSupportTrigger(index))
      } catch (e) {}
    })
  }
}

function createTray() {
  if (tray) return
  const iconPath = app.isPackaged
    ? path.join(__dirname, '../dist/icon.png')
    : path.join(__dirname, '../public/icon.png')
  
  try {
    tray = new Tray(iconPath)
    const contextMenu = Menu.buildFromTemplate([
      { label: 'Abrir Macro Helldivers 2', click: () => win.show() },
      { type: 'separator' },
      { label: 'Sair', click: () => { isQuitting = true; app.quit() }}
    ])
    tray.setToolTip('Macro Helldivers 2')
    tray.setContextMenu(contextMenu)
    tray.on('click', () => win.show())
  } catch (e) {
    console.error('Erro ao criar Tray:', e)
  }
}

function createWindow() {
  win = new BrowserWindow({
    width: 820,
    height: 640,
    title: "Macro Helldivers 2",
    icon: path.join(__dirname, '../public/icon.png'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
    },
    autoHideMenuBar: true,
  })

  if (isDev) win.loadURL(process.env.VITE_DEV_SERVER_URL)
  else win.loadFile(path.join(__dirname, '../dist/index.html'))

  win.webContents.on('did-finish-load', () => {
    autoUpdater.checkForUpdatesAndNotify().catch(() => {})
  })

  win.on('close', (event) => {
    if (!isQuitting) {
      event.preventDefault()
      win.hide()
      return false
    }
  })
}

app.whenReady().then(() => {
  createWindow()
  createTray()
  registerMacros()
})

// Gerenciamento de Atualizações
autoUpdater.autoDownload = true

function sendStatusToWindow(status, extra = {}) {
  if (win && !win.isDestroyed()) {
    win.webContents.send('update-status', { status, ...extra })
  }
}

autoUpdater.on('checking-for-update', () => {
  console.log('Verificando atualizações...')
  sendStatusToWindow('checking')
})

autoUpdater.on('update-available', (info) => {
  console.log('Atualização disponível:', info.version)
  sendStatusToWindow('available', { version: info.version })
})

autoUpdater.on('update-not-available', () => {
  console.log('Nenhuma atualização disponível.')
  sendStatusToWindow('up-to-date')
})

autoUpdater.on('download-progress', (progress) => {
  sendStatusToWindow('downloading', { percent: progress.percent })
})

autoUpdater.on('update-downloaded', (info) => {
  console.log('Atualização baixada.')
  sendStatusToWindow('ready', { version: info.version })
})

autoUpdater.on('error', (err) => {
  console.error('Erro no Auto-Updater:', err)
  sendStatusToWindow('error', { message: err.message })
})

ipcMain.handle('install-update', () => {
  autoUpdater.quitAndInstall()
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin' && isQuitting) app.quit()
})

ipcMain.on('update-slots', (event, slots) => { currentSlots = slots })
ipcMain.handle('get-settings', () => currentSettings)
ipcMain.on('save-settings', (event, settings) => {
  currentSettings = settings
  saveSettings(settings)
  if (!isRecordingState) registerMacros()
})
ipcMain.handle('set-recording-mode', (event, isRecording) => {
  isRecordingState = isRecording
  if (isRecording) globalShortcut.unregisterAll()
  else registerMacros()
  return true
})
