const { contextBridge, ipcRenderer } = require('electron')

// Registra o listener e devolve a função que o remove, evitando duplicatas a cada re-render
function on(channel, callback) {
  const handler = (_event, ...args) => callback(...args)
  ipcRenderer.on(channel, handler)
  return () => ipcRenderer.removeListener(channel, handler)
}

contextBridge.exposeInMainWorld('api', {
  // Funções de Escuta (Main -> Renderer)
  onUpdateStatus: (callback) => on('update-status', callback),
  onGameFocusChanged: (callback) => on('game-focus-changed', callback),
  onMacroTriggered: (callback) => on('macro-triggered', callback),
  onSupportMacroTriggered: (callback) => on('support-macro-triggered', callback),
  onMacroBlocked: (callback) => on('macro-blocked', callback),
  onMacroStatusChanged: (callback) => on('macro-status-changed', callback),
  onToggleMinimalMode: (callback) => on('toggle-minimal-mode', callback),
  onSyncSlots: (callback) => on('sync-slots', callback),
  onSyncSettings: (callback) => on('sync-settings', callback),

  // Funções de Envio (Renderer -> Main)
  updateSlots: (slots) => ipcRenderer.send('update-slots', slots),
  saveSettings: (settings) => ipcRenderer.send('save-settings', settings),
  hideWindow: () => ipcRenderer.send('hide-window'),
  hideOverlay: () => ipcRenderer.send('hide-overlay'),

  // Chamadas com retorno (Renderer -> Main -> Renderer)
  getSettings: () => ipcRenderer.invoke('get-settings'),
  setRecordingMode: (isRecording) => ipcRenderer.invoke('set-recording-mode', isRecording),
  installUpdate: () => ipcRenderer.invoke('install-update')
})
