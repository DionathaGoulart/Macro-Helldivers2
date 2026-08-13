const { app, BrowserWindow, globalShortcut, ipcMain, Tray, Menu, screen, dialog } = require('electron')
const path = require('path')
const fs = require('fs')
const { autoUpdater } = require('electron-updater')
import { SUPPORT_CODEXES } from '../shared/constants.js'

// Configurações Globais
let win
let tray
let isQuitting = false
let isRecordingState = false
let currentSlots = [null, null, null, null]
let isGameFocused = false
// A janela do overlay fica SEMPRE visível (transparente/click-through); esconder e
// mostrar de novo com show()/showInactive() ativa janelas transparentes no Windows,
// roubando o foco do jogo (que minimiza em tela cheia). Em vez disso alternam o
// CONTEÚDO (via IPC) e os BOUNDS (ver overlayBoundsFor) — janela de tela cheia é
// composta pelo DWM por cima do jogo mesmo sem nada desenhado.
// Estados: 'hidden' (1x1, nada renderizado) | 'minimal' (strip de slots) | 'panel' (painel completo)
let overlayState = 'hidden'
let isMacroRunning = false
let nut = null // Carregamento tardio (Lazy)

let overlayWin = null

// O HD2 em "Tela Cheia" (DXGI fullscreen) se auto-minimiza quando QUALQUER janela
// desenha por cima dele — comportamento do jogo, sem relação com foco. Overlay de
// janela só funciona em "Tela Cheia sem Borda"; detectamos o modo pra avisar o usuário.
// Lido em toda troca de estado/foco, então cacheamos: só relê quando o mtime muda.
let fullscreenCache = { mtime: -1, value: false }

function isGameExclusiveFullscreen() {
  try {
    const cfg = path.join(app.getPath('appData'), 'Arrowhead', 'Helldivers2', 'user_settings.config')
    const mtime = fs.statSync(cfg).mtimeMs
    if (mtime === fullscreenCache.mtime) return fullscreenCache.value
    const text = fs.readFileSync(cfg, 'utf8')
    const value = /^\s*fullscreen\s*=\s*true/m.test(text) && !/^\s*borderless_fullscreen\s*=\s*true/m.test(text)
    fullscreenCache = { mtime, value }
    return value
  } catch (e) {
    fullscreenCache = { mtime: -1, value: false }
    return false
  }
}

// Uma janela transparente de tela cheia é composta pelo DWM por cima do jogo em TODO
// frame, mesmo sem nada desenhado. Como não podemos escondê-la (ver overlayState),
// encolhemos os bounds pro tamanho real do conteúdo de cada estado.
// Medidas fixas em DIP, iguais ao CSS: painel = w-[820px] h-[640px] + folga da sombra;
// strip = 4 slots w-16 com gap-4 + p-4, escalado 0.70, + folga do brilho/pulse.
const OVERLAY_SIZES = {
  panel: { width: 840, height: 660 },
  minimal: { width: 340, height: 130 }
}

function overlayBoundsFor(state) {
  const display = screen.getPrimaryDisplay()
  const b = display.bounds
  // 1x1 num canto: janela segue viva (nunca damos show/hide) mas o compositor
  // praticamente não tem o que compor
  if (state === 'hidden') return { x: b.x, y: b.y, width: 1, height: 1 }
  const size = OVERLAY_SIZES[state]
  if (!size) return { ...b }
  if (state === 'minimal') {
    // Ancorado embaixo no centro, onde o CSS posiciona o strip (bottom-2)
    return {
      x: Math.round(b.x + (b.width - size.width) / 2),
      y: Math.round(b.y + b.height - size.height),
      width: size.width,
      height: size.height
    }
  }
  return {
    x: Math.round(b.x + (b.width - size.width) / 2),
    y: Math.round(b.y + (b.height - size.height) / 2),
    width: size.width,
    height: size.height
  }
}

function applyOverlayBounds(state) {
  if (!overlayWin || overlayWin.isDestroyed()) return
  try {
    // A janela é resizable:false pra ninguém arrastar a borda. No Windows isso trava
    // min/max size no tamanho atual e pode fazer setBounds virar no-op — liberamos
    // só durante a chamada.
    const wasResizable = overlayWin.isResizable()
    if (!wasResizable) overlayWin.setResizable(true)
    overlayWin.setBounds(overlayBoundsFor(state))
    if (!wasResizable) overlayWin.setResizable(false)
  } catch (e) {
    console.error('Erro ao redimensionar overlay:', e)
  }
}

function setOverlayState(state) {
  if (!overlayWin || overlayWin.isDestroyed()) return
  overlayState = state
  applyOverlayBounds(state)
  overlayWin.webContents.send('overlay-state', state)
  if (state !== 'hidden') {
    overlayWin.webContents.send('fullscreen-warning', isGameExclusiveFullscreen())
  }
  // Mouse só interage com o painel completo; nos demais estados tudo atravessa pro jogo
  if (state === 'panel') overlayWin.setIgnoreMouseEvents(false)
  else overlayWin.setIgnoreMouseEvents(true, { forward: true })
}

const toggleOverlay = () => {
  if (currentSettings.enableOverlay === false || !isGameFocused) return
  if (!overlayWin || overlayWin.isDestroyed()) return

  if (overlayState === 'panel') {
    setOverlayState(currentSettings.alwaysShowSlots ? 'minimal' : 'hidden')
  } else {
    setOverlayState('panel')
  }
}

function registerOverlayShortcut() {
  if (currentSettings.enableOverlay === false) return
  try {
    globalShortcut.register('CommandOrControl+H', toggleOverlay)
  } catch (e) {
    console.error('Erro ao registrar atalho do overlay:', e)
  }
}

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

const DEFAULT_SETTINGS = {
  shortcuts: ['F1', 'F2', 'F3', 'F4'],
  supportShortcuts: [null, null, null],
  modifierKey: 'LeftControl',
  sprintModifier: 'Shift',
  useArrows: false,
  enableOverlay: true,
  alwaysShowSlots: false
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
    try { 
      const loaded = JSON.parse(fs.readFileSync(p, 'utf8'))
      return { ...DEFAULT_SETTINGS, ...loaded }
    } catch (e) { return DEFAULT_SETTINGS }
  }
  return DEFAULT_SETTINGS
}

let currentSettings = loadSettings()

// Polling de Foco (Seguro)
// Só começa depois que a janela principal pintou: o primeiro loadMacroEngine()
// carrega módulos nativos e bloqueia o processo main por ~1s — durante o boot
// isso trava a abertura do app.
let focusPollTimer = null
let isPollingFocus = false

function startFocusPolling() {
  if (focusPollTimer) return
  focusPollTimer = setInterval(pollGameFocus, 500)
}

async function pollGameFocus() {
  if (isPollingFocus) return // getActiveWindow lento não pode empilhar chamadas
  isPollingFocus = true
  try {
    await checkGameFocus()
  } finally {
    isPollingFocus = false
  }
}

async function checkGameFocus() {
  const engine = loadMacroEngine()
  if (!engine || !engine.getActiveWindow) return

  try {
    const activeWindow = await engine.getActiveWindow()
    if (!activeWindow) return // Ignora nulos momentâneos (transições de janela)
    
    // Suporte a propriedade ou função para compatibilidade com diferentes versões do nut-js
    const title = typeof activeWindow.title === 'function' ? await activeWindow.title() : await activeWindow.title
    
    if (!title) return // Ignora se não conseguir ler o título momentaneamente

    // Busca pelo título do jogo e janelas do app
    const isOverlayActive = title.toUpperCase().includes('HD2_OVERLAY')
    const isMainApp = title.toUpperCase().includes('MACRO') && title.toUpperCase().includes('HELLDIVERS')
    const isGame = title.toUpperCase().includes('HELLDIVERS') && !isMainApp
    
    // O sistema fica "ativo" se estiver no jogo, no overlay ou na janela de configuração
    const isFocused = isGame || isOverlayActive || isMainApp

    // Jogos podem re-agarrar o topo do z-order (alt-tab, troca de modo de vídeo);
    // reafirma o overlay acima enquanto o jogo está em foco e há conteúdo visível
    if (isFocused && overlayState !== 'hidden' && overlayWin && !overlayWin.isDestroyed() && overlayWin.isVisible()) {
      overlayWin.setAlwaysOnTop(true, 'screen-saver')
      overlayWin.moveTop()
    }

    if (isFocused !== isGameFocused) {
      isGameFocused = isFocused
      
      if (isGameFocused) {
        registerMacros()
        if (currentSettings.alwaysShowSlots) {
          setOverlayState('minimal')
        }
        // Avisa as duas janelas se o jogo está em "Tela Cheia" (modo incompatível com overlay)
        broadcast('fullscreen-warning', isGameExclusiveFullscreen())
      } else {
        globalShortcut.unregisterAll()
        registerOverlayShortcut()
        setOverlayState('hidden')
      }

      if (win && !win.isDestroyed()) {
        win.webContents.send('game-focus-changed', isGameFocused)
      }
      if (overlayWin && !overlayWin.isDestroyed()) {
        overlayWin.webContents.send('game-focus-changed', isGameFocused)
      }
    }
  } catch (e) {
    // Se der erro, por segurança desativamos para não disparar macro em apps errados
    if (isGameFocused) {
      isGameFocused = false
      if (win && !win.isDestroyed()) win.webContents.send('game-focus-changed', false)
    }
  }
}

function broadcast(channel, payload) {
  if (win && !win.isDestroyed()) win.webContents.send(channel, payload)
  if (overlayWin && !overlayWin.isDestroyed()) overlayWin.webContents.send(channel, payload)
}

async function handleMacroTrigger(codex, index, isSupport) {
  if (isRecordingState || !isGameFocused || !codex) return

  // O runner ignora chamadas concorrentes, então avisamos a UI em vez de falhar em silêncio
  if (isMacroRunning) {
    broadcast('macro-blocked', { slot: index, isSupport })
    return
  }

  const engine = loadMacroEngine()
  if (!engine) return

  isMacroRunning = true
  // A sequência de teclas começa ANTES dos avisos: cada broadcast são dois sends IPC
  // com serialização estruturada, e isso é latência gasta no ponto mais sensível do app.
  // O feedback visual chegar alguns ms depois ninguém percebe; o input, sim.
  const running = engine.runStratagem(codex, currentSettings.modifierKey, currentSettings.useArrows, currentSettings.macroSpeed)
  broadcast(isSupport ? 'support-macro-triggered' : 'macro-triggered', index)
  broadcast('macro-status-changed', { slot: index, isSupport, running: true })
  try {
    await running
  } catch (e) {
    console.error('Erro ao executar macro:', e)
  }
  isMacroRunning = false
  broadcast('macro-status-changed', { slot: index, isSupport, running: false })
}

function registerMacros() {
  if (isRecordingState || !isGameFocused) return
  globalShortcut.unregisterAll()
  registerOverlayShortcut()

  // Atalho extra para quem segura a tecla de corrida enquanto chama o estratagema
  const sprintModifier = currentSettings.sprintModifier ?? 'Shift'
  const sprintPrefix = sprintModifier === 'None' ? null : `${sprintModifier}+`

  const registerWithSprint = (key, run) => {
    globalShortcut.register(key, run)
    if (sprintPrefix) globalShortcut.register(`${sprintPrefix}${key}`, run)
  }

  // Slots normais
  currentSettings.shortcuts.forEach((key, index) => {
    if (!key) return
    try {
      registerWithSprint(key, () => handleMacroTrigger(currentSlots[index]?.codex, index, false))
    } catch (e) {}
  })

  // Slots de suporte
  if (currentSettings.supportShortcuts) {
    currentSettings.supportShortcuts.forEach((key, index) => {
      if (!key) return
      try {
        registerWithSprint(key, () => handleMacroTrigger(SUPPORT_CODEXES[index], index, true))
      } catch (e) {}
    })
  }
}

// Um Tray sem ícone válido lança e deixa a janela inalcançável depois de fechada,
// então tentamos todos os caminhos possíveis antes de desistir
// tray.png é a versão 64px; icon.png (1024px) fica como fallback, mas decodificar
// um bitmap de 1024 pra desenhar 16px na barra de tarefas é desperdício puro
function resolveTrayIcon() {
  const candidates = app.isPackaged
    ? [
        path.join(process.resourcesPath, 'tray.png'),
        path.join(process.resourcesPath, 'icon.png'),
        path.join(process.resourcesPath, 'public', 'icon.png'),
        path.join(app.getAppPath(), 'public', 'icon.png')
      ]
    : [
        path.join(__dirname, '../public/tray.png'),
        path.join(__dirname, '../public/icon.png')
      ]
  return candidates.find(p => fs.existsSync(p)) || null
}

function createTray() {
  if (tray) return
  const iconPath = resolveTrayIcon()
  if (!iconPath) {
    console.error('Ícone do tray não encontrado; a janela vai fechar em vez de minimizar')
    return
  }

  const restore = () => {
    if (!win || win.isDestroyed()) return
    if (win.isMinimized()) win.restore()
    win.show()
    win.focus()
  }

  try {
    tray = new Tray(iconPath)
    const contextMenu = Menu.buildFromTemplate([
      { label: 'Abrir Macro Helldivers 2', click: restore },
      { type: 'separator' },
      { label: 'Sair', click: () => { isQuitting = true; app.quit() }}
    ])
    tray.setToolTip('Macro Helldivers 2')
    tray.setContextMenu(contextMenu)
    tray.on('click', restore)
    tray.on('double-click', restore)
  } catch (e) {
    tray = null
    console.error('Erro ao criar Tray:', e)
  }
}

function getWindowBoundsPath() {
  return path.join(app.getPath('userData'), 'Helldivers Macro', 'window-bounds.json')
}

let saveBoundsTimer = null

function saveWindowBounds() {
  if (!win || win.isDestroyed()) return
  clearTimeout(saveBoundsTimer)
  saveBoundsTimer = setTimeout(() => {
    try {
      fs.writeFileSync(getWindowBoundsPath(), JSON.stringify(win.getBounds()))
    } catch (e) {
      console.error('Erro ao salvar posição da janela:', e)
    }
  }, 500)
}

// Descarta posições fora dos monitores atuais para a janela não abrir invisível
function loadWindowBounds() {
  try {
    const p = getWindowBoundsPath()
    if (!fs.existsSync(p)) return null
    const bounds = JSON.parse(fs.readFileSync(p, 'utf8'))
    const isVisible = screen.getAllDisplays().some(display => {
      const { x, y, width, height } = display.workArea
      return bounds.x >= x && bounds.x + bounds.width <= x + width &&
             bounds.y >= y && bounds.y + bounds.height <= y + height
    })
    return isVisible ? bounds : null
  } catch (e) {
    return null
  }
}

function createWindow() {
  const savedBounds = loadWindowBounds()

  win = new BrowserWindow({
    width: savedBounds?.width || 820,
    height: savedBounds?.height || 640,
    x: savedBounds?.x,
    y: savedBounds?.y,
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
    // Fora do caminho crítico do boot: ambos tocam rede/módulo nativo
    setTimeout(() => {
      autoUpdater.checkForUpdatesAndNotify().catch(() => {})
      startFocusPolling()
    }, 1500)
  })

  win.on('resize', saveWindowBounds)
  win.on('move', saveWindowBounds)

  // Minimizar também recolhe pro tray: o app segue rodando os macros em segundo plano
  win.on('minimize', (event) => {
    if (!tray) return
    event.preventDefault()
    win.hide()
  })

  win.on('close', (event) => {
    // Sem tray a janela escondida seria irrecuperável — nesse caso fechar encerra mesmo
    if (!isQuitting && tray) {
      event.preventDefault()
      saveWindowBounds()
      win.hide()
      return false
    }
    if (!isQuitting) isQuitting = true
  })
}

function createOverlayWindow() {
  // Nasce já do tamanho do painel; setOverlayState ajusta pro estado real assim que carrega
  const initial = overlayBoundsFor('panel')

  overlayWin = new BrowserWindow({
    width: initial.width,
    height: initial.height,
    x: initial.x,
    y: initial.y,
    title: 'HD2_OVERLAY',
    transparent: true,
    frame: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    // WS_EX_NOACTIVATE: recebe cliques sem nunca ativar — o jogo mantém o foco
    focusable: false,
    resizable: false,
    fullscreenable: false,
    hasShadow: false,
    roundedCorners: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
    }
  })

  overlayWin.setIgnoreMouseEvents(true, { forward: true })
  overlayWin.setAlwaysOnTop(true, 'screen-saver')

  if (isDev) overlayWin.loadURL(process.env.VITE_DEV_SERVER_URL + '#overlay')
  else overlayWin.loadFile(path.join(__dirname, '../dist/index.html'), { hash: 'overlay' })

  // Sincroniza o estado inicial quando o renderer terminar de carregar
  overlayWin.webContents.on('did-finish-load', () => {
    setOverlayState(isGameFocused && currentSettings.alwaysShowSlots ? 'minimal' : 'hidden')
  })

  // Mostrada uma única vez, aqui, e nunca mais escondida (ver comentário em overlayState)
  overlayWin.showInactive()
}

app.whenReady().then(() => {
  createWindow()
  createTray()
  registerOverlayShortcut()
  // A janela de overlay carrega um segundo renderer inteiro; fora do boot pra
  // não competir com a primeira pintura da janela principal
  setTimeout(createOverlayWindow, 1000)

  screen.on('display-metrics-changed', () => {
    applyOverlayBounds(overlayState)
  })
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

ipcMain.on('update-slots', (event, slots) => {
  currentSlots = slots
  // Não ecoa de volta pra janela que enviou — ela já tem o estado (evita render duplo)
  if (overlayWin && !overlayWin.isDestroyed() && overlayWin.webContents !== event.sender) overlayWin.webContents.send('sync-slots', slots)
  if (win && !win.isDestroyed() && win.webContents !== event.sender) win.webContents.send('sync-slots', slots)
})
ipcMain.on('update-loadouts', (event, loadouts) => {
  // Só repassa pra outra janela — o main não usa loadouts (macros seguem via update-slots)
  if (overlayWin && !overlayWin.isDestroyed() && overlayWin.webContents !== event.sender) overlayWin.webContents.send('sync-loadouts', loadouts)
  if (win && !win.isDestroyed() && win.webContents !== event.sender) win.webContents.send('sync-loadouts', loadouts)
})
// Estatísticas de pick rate da comunidade (backend do helldive.live).
// API não documentada de site de fã — pode mudar sem aviso; o renderer degrada com mensagem de erro.
const HELLDIVE_API = 'https://utm7j5pjvi.us-east-1.awsapprunner.com'
const HELLDIVE_PATCH_ID = 12 // "Exo Experts" — atualizar quando o site adicionar patch novo

ipcMain.handle('fetch-meta-stats', async (event, faction, difficulty) => {
  try {
    const results = {}
    for (const type of ['strategem', 'weapons', 'armor']) {
      const url = `${HELLDIVE_API}/items_stats?faction=${encodeURIComponent(faction)}&patch_id=${HELLDIVE_PATCH_ID}&difficulty=${Number(difficulty) || 0}&mission=All&modifier=ALL&type=${type}`
      const res = await fetch(url, { signal: AbortSignal.timeout(10000) })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      results[type] = await res.json()
    }
    return { ok: true, data: results }
  } catch (e) {
    return { ok: false, error: e.message }
  }
})

ipcMain.handle('export-data', async (event, data) => {
  const { canceled, filePath } = await dialog.showSaveDialog({
    title: 'Exportar backup',
    defaultPath: 'macro-helldivers2-backup.json',
    filters: [{ name: 'JSON', extensions: ['json'] }]
  })
  if (canceled || !filePath) return { ok: false, canceled: true }
  try {
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf8')
    return { ok: true }
  } catch (e) {
    return { ok: false, error: e.message }
  }
})

ipcMain.handle('import-data', async () => {
  const { canceled, filePaths } = await dialog.showOpenDialog({
    title: 'Importar backup',
    filters: [{ name: 'JSON', extensions: ['json'] }],
    properties: ['openFile']
  })
  if (canceled || !filePaths?.length) return { ok: false, canceled: true }
  try {
    const data = JSON.parse(fs.readFileSync(filePaths[0], 'utf8'))
    return { ok: true, data }
  } catch (e) {
    return { ok: false, error: e.message }
  }
})

ipcMain.handle('get-settings', () => currentSettings)
ipcMain.on('save-settings', (event, settings) => {
  const previousAlwaysShow = currentSettings.alwaysShowSlots;
  currentSettings = settings
  saveSettings(settings)
  if (!isRecordingState) registerMacros()
  
  if (overlayWin && !overlayWin.isDestroyed()) {
    overlayWin.webContents.send('sync-settings', settings)
    if (isGameFocused && overlayState !== 'panel') {
      if (settings.alwaysShowSlots && !previousAlwaysShow) {
        setOverlayState('minimal')
      } else if (!settings.alwaysShowSlots && previousAlwaysShow) {
        setOverlayState('hidden')
      }
    }
  }
  if (win && !win.isDestroyed()) win.webContents.send('sync-settings', settings)
})
ipcMain.on('hide-window', () => {
  if (win && !win.isDestroyed()) {
    win.hide()
  }
})
ipcMain.on('hide-overlay', () => {
  if (overlayWin && !overlayWin.isDestroyed()) {
    toggleOverlay()
  }
})
ipcMain.handle('set-recording-mode', (event, isRecording) => {
  isRecordingState = isRecording
  if (isRecording) {
    globalShortcut.unregisterAll()
    registerOverlayShortcut()
  } else {
    registerMacros()
  }
  return true
})
